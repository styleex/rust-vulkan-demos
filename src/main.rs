use std::path::Path;
use std::ptr;
use std::sync::Arc;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;

use utils::{commands, pipeline, render_pass,
            sync, uniform_buffer, vertex};

use crate::render_env::{descriptors, env, frame_buffer, pipeline_builder};
use crate::render_env::egui::EguiRenderer;
use crate::utils::sync::MAX_FRAMES_IN_FLIGHT;
use crate::utils::texture;

mod utils;
mod camera;
mod fps_limiter;
mod render_env;


struct HelloApplication {
    egui_render: EguiRenderer,
    egui_ctx: egui::CtxRef,

    swapchain_stuff: render_env::swapchain::SwapChain,

    vertex_buffer: vertex::VertexBuffer,
    uniform_buffers: uniform_buffer::UboBuffers,
    sync: sync::SyncObjects,

    texture: texture::Texture,

    current_frame: usize,
    is_window_resized: bool,

    msaa_samples: vk::SampleCountFlags,
    camera: camera::Camera,

    framebuffer: frame_buffer::FrameBuffer,

    draw_mesh_second_cmd: vk::CommandBuffer,
    geometry_pass_cmds: [vk::CommandBuffer; MAX_FRAMES_IN_FLIGHT],
    pipeline_second: pipeline_builder::Pipeline,
    descriptor_set_second: descriptors::DescriptorSet,

    quad_pipeline: pipeline_builder::Pipeline,
    quad_render_pass: vk::RenderPass,
    quad_descriptors: Vec<descriptors::DescriptorSet>,
    draw_quad_primary_cmds: Vec<vk::CommandBuffer>,  // Per frame command buffers

    env: Arc<env::RenderEnv>,
}

impl HelloApplication {
    pub fn new(wnd: &winit::window::Window) -> HelloApplication {
        let env = Arc::new(env::RenderEnv::new(wnd));

        let msaa_samples = render_env::utils::get_max_usable_sample_count(&env);

        let mut swapchain_stuff = render_env::swapchain::SwapChain::new(&env, wnd.inner_size());

        let quad_render_pass = render_pass::create_quad_render_pass(env.device(), swapchain_stuff.format);
        swapchain_stuff.create_framebuffers(env.device(), quad_render_pass);

        let vertex_buffer = vertex::VertexBuffer::create(env.instance(), env.physical_device(), env.device().clone(), env.command_pool(), env.queue());

        let uniform_buffers = uniform_buffer::UboBuffers::new(env.instance(), env.device().clone(), env.physical_device(), swapchain_stuff.images.len());

        let mut camera = camera::Camera::new();
        camera.set_viewport(
            swapchain_stuff.size.width,
            swapchain_stuff.size.height,
        );

        let mem_properties =
            unsafe { env.instance().get_physical_device_memory_properties(env.physical_device()) };

        let texture = texture::Texture::new(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &mem_properties,
            Path::new("assets/chalet.jpg"),
        );

        let dimensions = [swapchain_stuff.size.width, swapchain_stuff.size.height];
        let mut framebuffer = frame_buffer::FrameBuffer::new(env.clone(), vec!(
            frame_buffer::AttachmentDesciption {
                samples_count: msaa_samples,
                format: vk::Format::R8G8B8A8_SRGB,
            },
            frame_buffer::AttachmentDesciption {
                samples_count: msaa_samples,
                format: vk::Format::D32_SFLOAT,
            },
        ));
        framebuffer.resize_swapchain(dimensions);

        let pipeline_second = pipeline::create_graphics_pipeline(
            env.device().clone(), framebuffer.render_pass(), msaa_samples,
        );
        let descriptor_set_second = descriptors::DescriptorSetBuilder::new(
            env.device(), pipeline_second.descriptor_set_layouts.get(0).unwrap())
            .add_buffer(uniform_buffers.uniform_buffers[0])
            .add_image(texture.texture_image_view, texture.texture_sampler)
            .build();

        let second_buffer = commands::create_second_command_buffers(
            env.device(),
            env.command_pool(),
            pipeline_second.graphics_pipeline,
            framebuffer.render_pass(),
            swapchain_stuff.size,
            vertex_buffer.vertex_buffer,
            vertex_buffer.index_buffer,
            vertex_buffer.index_count,
            pipeline_second.pipeline_layout,
            descriptor_set_second.set,
        );

        let quad_pipeline = pipeline::create_quad_graphics_pipeline(
            env.device().clone(), quad_render_pass, msaa_samples,
        );

        let mut quad_descriptors = Vec::new();
        for _ in swapchain_stuff.image_views.iter() {
            quad_descriptors.push(
                descriptors::DescriptorSetBuilder::new(
                    env.device(), quad_pipeline.descriptor_set_layouts.get(0).unwrap())
                    .add_image(framebuffer.attachments.get(0).unwrap().view, texture.texture_sampler)
                    .build()
            );
        }

        let quad_command_buffers = commands::create_quad_command_buffers(
            env.device(),
            env.command_pool(),
            quad_pipeline.graphics_pipeline,
            &swapchain_stuff.framebuffers,
            quad_render_pass,
            swapchain_stuff.size,
            quad_pipeline.pipeline_layout,
            quad_descriptors.iter().map(|x| x.set).collect(),
        );

        println!("created");

        let sync = sync::create_sync_objects(env.device());

        let mut egui_ctx = egui::CtxRef::default();
        let mut init_input = egui::RawInput::default();
        init_input.pixels_per_point = Some(wnd.scale_factor() as f32);

        egui_ctx.begin_frame(init_input);
        let (_output, _shapes) = egui_ctx.end_frame();

        let egui_renderer = EguiRenderer::new(env.clone(), egui_ctx.clone(), quad_render_pass.clone());
        HelloApplication {
            env,

            swapchain_stuff,

            vertex_buffer,

            uniform_buffers,
            texture,

            sync,
            current_frame: 0,
            is_window_resized: false,
            msaa_samples,
            camera,

            framebuffer,
            draw_mesh_second_cmd: second_buffer,
            geometry_pass_cmds: [vk::CommandBuffer::null(), vk::CommandBuffer::null()],
            pipeline_second,
            descriptor_set_second,

            quad_render_pass,
            quad_pipeline,
            quad_descriptors,

            draw_quad_primary_cmds: quad_command_buffers,
            egui_ctx,
            egui_render: egui_renderer,
        }
    }

    pub fn run(&mut self, mut event_loop: EventLoop<()>, wnd: winit::window::Window) {
        let mut tick_counter = fps_limiter::FPSLimiter::new();

        event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, window_id } => {
                    if window_id != wnd.id() {
                        return;
                    }

                    if let WindowEvent::CloseRequested = event {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    if let WindowEvent::KeyboardInput { input, .. } = event {
                        if input.virtual_keycode.is_some() && input.virtual_keycode.unwrap() == VirtualKeyCode::Escape {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }

                    if let WindowEvent::Resized(_) = event {
                        self.is_window_resized = true;
                    }

                    self.camera.handle_event(&event);
                }
                Event::MainEventsCleared => {
                    wnd.request_redraw()
                }
                Event::RedrawRequested(_) => {
                    self.draw_frame(&wnd);

                    // print!("FPS: {}\r", tick_counter.fps());
                    tick_counter.tick_frame();
                }
                // Important!
                Event::LoopDestroyed => {
                    unsafe { self.env.device().device_wait_idle().unwrap(); }
                }
                _ => (),
            }
        })
    }

    fn draw_frame(&mut self, wnd: &winit::window::Window) {
        let wait_fences = [self.sync.inflight_fences[self.current_frame]];

        let (image_index, _is_sub_optimal) = unsafe {
            self.env.device()
                .wait_for_fences(&wait_fences, true, std::u64::MAX)
                .expect("Failed to wait for Fence!");

            let result = self.swapchain_stuff.swapchain_api
                .acquire_next_image(
                    self.swapchain_stuff.swapchain,
                    std::u64::MAX,
                    self.sync.image_available_semaphores[self.current_frame],
                    vk::Fence::null(),
                );
            match result {
                Ok(image_index) => image_index,
                Err(vk_result) => match vk_result {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.recreate_swapchain(&wnd);
                        return;
                    }
                    _ => panic!("Failed to acquire Swap Chain Image!"),
                },
            }
        };
        self.uniform_buffers.update_uniform_buffer(image_index as usize,
                                                   self.camera.view_matrix(), self.camera.proj_matrix());

        let wait_semaphores = [self.sync.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let first_pass_finished = [self.sync.render_finished_semaphores[self.current_frame]];
        let second_pass_finished = [self.sync.render_quad_semaphore];
        let gui_finished = [self.sync.render_gui_semaphore];


        let geometry_pass_cmd = frame_buffer::draw_to_framebuffer(
            &self.env, &self.framebuffer,
            |cmd| {
                unsafe {
                    self.env.device().cmd_execute_commands(cmd, &[self.draw_mesh_second_cmd]);
                }
            });

        unsafe {
            if self.geometry_pass_cmds[self.current_frame] != vk::CommandBuffer::null() {
                self.env.device().free_command_buffers(self.env.command_pool(), &[self.geometry_pass_cmds[self.current_frame]]);
            }
        }
        self.geometry_pass_cmds[self.current_frame] = geometry_pass_cmd;

        let raw_input = egui::RawInput::default();
        self.egui_ctx.begin_frame(raw_input);
        egui::CentralPanel::default().show(&self.egui_ctx, |ui| {
            ui.heading("Test");
            // ui.checkbox(&mut false, "Qwe");
        });

        // egui::SidePanel::left("Qwe").show(&self.egui_ctx, |ui| {
        //     ui.heading("Test")
        // });

        let (_output, shapes) = self.egui_ctx.end_frame();
        let clipped_meshes = self.egui_ctx.tessellate(shapes);
        let gui_render_op = self.egui_render.render(
            clipped_meshes,
            self.swapchain_stuff.framebuffers[image_index as usize],
            [self.swapchain_stuff.size.width, self.swapchain_stuff.size.height],
            MAX_FRAMES_IN_FLIGHT,
        );

        let submit_infos = [
            vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: wait_semaphores.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: [self.geometry_pass_cmds[self.current_frame]].as_ptr(),
                signal_semaphore_count: first_pass_finished.len() as u32,
                p_signal_semaphores: first_pass_finished.as_ptr(),
            },
            vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: first_pass_finished.len() as u32,
                p_wait_semaphores: first_pass_finished.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: [self.draw_quad_primary_cmds[image_index as usize]].as_ptr(),
                signal_semaphore_count: second_pass_finished.len() as u32,
                p_signal_semaphores: second_pass_finished.as_ptr(),
            },
            vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: second_pass_finished.len() as u32,
                p_wait_semaphores: second_pass_finished.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: [gui_render_op].as_ptr(),
                signal_semaphore_count: gui_finished.len() as u32,
                p_signal_semaphores: gui_finished.as_ptr(),
            }
        ];

        unsafe {
            self.env.device()
                .reset_fences(&wait_fences)
                .expect("Failed to reset Fence!");

            self.env.device()
                .queue_submit(
                    self.env.queue(),
                    &submit_infos,
                    self.sync.inflight_fences[self.current_frame],
                )
                .expect("Failed to execute queue submit.");
        }
        let swapchains = [self.swapchain_stuff.swapchain];

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: gui_finished.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: ptr::null_mut(),
        };

        let result = unsafe {
            self.swapchain_stuff.swapchain_api
                .queue_present(self.env.queue(), &present_info)
        };

        let is_resized = match result {
            Ok(_) => self.is_window_resized,
            Err(vk_result) => match vk_result {
                vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => true,
                _ => panic!("Failed to execute queue present"),
            }
        };

        if is_resized {
            self.recreate_swapchain(wnd);
            self.is_window_resized = false;
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn recreate_swapchain(&mut self, wnd: &winit::window::Window) {
        unsafe {
            self.env.device()
                .device_wait_idle()
                .expect("Failed to wait device idle!")
        };
        self.cleanup_swapchain();

        self.swapchain_stuff = render_env::swapchain::SwapChain::new(&self.env, wnd.inner_size());
        self.swapchain_stuff.create_framebuffers(self.env.device(), self.quad_render_pass);

        let dimensions = [self.swapchain_stuff.size.width, self.swapchain_stuff.size.height];
        self.framebuffer.resize_swapchain(dimensions);

        let mut quad_descriptors = Vec::new();
        for _img_view in self.swapchain_stuff.image_views.iter() {
            quad_descriptors.push(
                descriptors::DescriptorSetBuilder::new(
                    self.env.device(),
                    self.quad_pipeline.descriptor_set_layouts.get(0).unwrap(),
                )
                    .add_image(self.framebuffer.attachments.get(0).unwrap().view, self.texture.texture_sampler)
                    .build()
            );
        };
        self.quad_descriptors = quad_descriptors;

        self.draw_quad_primary_cmds = commands::create_quad_command_buffers(
            &self.env.device(),
            self.env.command_pool(),
            self.quad_pipeline.graphics_pipeline,
            &self.swapchain_stuff.framebuffers,
            self.quad_render_pass,
            self.swapchain_stuff.size,
            self.quad_pipeline.pipeline_layout,
            self.quad_descriptors.iter().map(|x| x.set).collect(),
        );


        self.draw_mesh_second_cmd = commands::create_second_command_buffers(
            self.env.device(),
            self.env.command_pool(),
            self.pipeline_second.graphics_pipeline,
            self.framebuffer.render_pass(),
            self.swapchain_stuff.size,
            self.vertex_buffer.vertex_buffer,
            self.vertex_buffer.index_buffer,
            self.vertex_buffer.index_count,
            self.pipeline_second.pipeline_layout,
            self.descriptor_set_second.set,
        );
    }

    fn cleanup_swapchain(&mut self) {
        self.swapchain_stuff.destroy();

        for mut set in self.quad_descriptors.drain(0..) {
            set.destroy();
        }
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            self.sync.destroy();
            self.cleanup_swapchain();

            self.descriptor_set_second.destroy();

            self.framebuffer.destroy();
            self.env.device().destroy_render_pass(self.quad_render_pass, None);

            self.uniform_buffers.destroy();
            self.vertex_buffer.destroy();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let wnd = winit::window::WindowBuilder::new()
        .with_title("test")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");

    let mut app = HelloApplication::new(&wnd);
    app.run(event_loop, wnd);
}
