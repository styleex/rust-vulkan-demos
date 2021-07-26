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
use crate::render_env::egui::Egui;
use crate::utils::sync::MAX_FRAMES_IN_FLIGHT;
use crate::utils::texture;
use crate::utils::quad_render::QuadRenderer;

mod utils;
mod camera;
mod fps_limiter;
mod render_env;


struct HelloApplication {
    egui: Egui,

    quad_renderer: QuadRenderer,
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

    quad_render_pass: vk::RenderPass,
    env: Arc<env::RenderEnv>,

    clear_color: [f32; 3],
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
        let mut offscreen_framebuffer = frame_buffer::FrameBuffer::new(env.clone(), vec!(
            frame_buffer::AttachmentDesciption {
                samples_count: msaa_samples,
                format: vk::Format::R8G8B8A8_SRGB,
            },
            frame_buffer::AttachmentDesciption {
                samples_count: msaa_samples,
                format: vk::Format::D32_SFLOAT,
            },
        ));
        offscreen_framebuffer.resize_swapchain(dimensions);

        let draw_mesh_pipeline = pipeline::create_graphics_pipeline(
            env.device().clone(), offscreen_framebuffer.render_pass(), msaa_samples,
        );
        let draw_mesh_descriptor_set = descriptors::DescriptorSetBuilder::new(
            env.device(), draw_mesh_pipeline.descriptor_set_layouts.get(0).unwrap())
            .add_buffer(uniform_buffers.uniform_buffers[0])
            .add_image(texture.texture_image_view, texture.texture_sampler)
            .build();

        let draw_mesh_cmd = commands::create_second_command_buffers(
            env.device(),
            env.command_pool(),
            draw_mesh_pipeline.graphics_pipeline,
            offscreen_framebuffer.render_pass(),
            swapchain_stuff.size,
            vertex_buffer.vertex_buffer,
            vertex_buffer.index_buffer,
            vertex_buffer.index_count,
            draw_mesh_pipeline.pipeline_layout,
            draw_mesh_descriptor_set.set,
        );

        println!("created");

        let quad_renderer = QuadRenderer::new(env.clone(), &offscreen_framebuffer, quad_render_pass, msaa_samples, dimensions);
        let sync = sync::create_sync_objects(env.device());

        let egui = Egui::new(env.clone(), swapchain_stuff.format, wnd.scale_factor(), dimensions);
        HelloApplication {
            env,
            quad_renderer,
            swapchain_stuff,

            vertex_buffer,

            uniform_buffers,
            texture,

            sync,
            current_frame: 0,
            is_window_resized: false,
            msaa_samples,
            camera,

            framebuffer: offscreen_framebuffer,
            draw_mesh_second_cmd: draw_mesh_cmd,
            geometry_pass_cmds: [vk::CommandBuffer::null(), vk::CommandBuffer::null()],
            pipeline_second: draw_mesh_pipeline,
            descriptor_set_second: draw_mesh_descriptor_set,

            egui,

            clear_color: [0.0, 0.0, 0.0],
            quad_render_pass,
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

                    if !self.egui.context().is_pointer_over_area() {
                        self.camera.handle_event(&event);
                    }

                    if !self.camera.mouse_acquired() {
                        self.egui.handle_event(&event);
                    }
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


        let geometry_pass_cmd = frame_buffer::draw_to_framebuffer(
            &self.env, self.clear_color, &self.framebuffer,
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

        self.egui.begin_frame();

        egui::SidePanel::left("my_side_panel").show(&self.egui.context(), |ui| {
            ui.heading("Hello");
            ui.separator();

            // let mut rgb: [f32; 3] = [0.0, 0.0, 0.0];
            ui.color_edit_button_rgb(&mut self.clear_color);
        });

        let gui_render_op = self.egui.end_frame(
            wnd,
            [self.swapchain_stuff.size.width, self.swapchain_stuff.size.height],
            MAX_FRAMES_IN_FLIGHT,
        );

        let quad_cmd_buf = self.quad_renderer.render(
            [self.swapchain_stuff.size.width, self.swapchain_stuff.size.height],
            self.swapchain_stuff.framebuffers[image_index as usize],
            vec![gui_render_op],
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
                p_command_buffers: [quad_cmd_buf].as_ptr(),
                signal_semaphore_count: second_pass_finished.len() as u32,
                p_signal_semaphores: second_pass_finished.as_ptr(),
            },
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
            p_wait_semaphores: second_pass_finished.as_ptr(),
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
        self.quad_renderer.update_framebuffer(&self.framebuffer, dimensions);


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
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            self.sync.destroy();
            self.cleanup_swapchain();

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
