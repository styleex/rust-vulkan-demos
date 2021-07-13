use std::path::Path;
use std::ptr;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;

use utils::{commands, descriptor_set, pipeline, render_pass,
            sync, uniform_buffer, vertex};

use crate::utils::sync::MAX_FRAMES_IN_FLIGHT;
use crate::utils::texture;

use crate::render_env::env;

mod utils;
mod camera;
mod fps_limiter;
mod render_env;


struct HelloApplication {
    env: env::RenderEnv,

    swapchain_stuff: render_env::swapchain::SwapChainStuff,

    render_pass: vk::RenderPass,
    ubo_layout: vk::DescriptorSetLayout,
    descriptor_sets: descriptor_set::DescriptorSets,

    pipeline: pipeline::Pipeline,
    command_buffers: Vec<vk::CommandBuffer>,
    vertex_buffer: vertex::VertexBuffer,
    uniform_buffers: uniform_buffer::UboBuffers,
    sync: sync::SyncObjects,

    texture: texture::Texture,

    current_frame: usize,
    is_window_resized: bool,

    msaa_samples: vk::SampleCountFlags,
    camera: camera::Camera,
}

impl HelloApplication {
    pub fn new(wnd: &winit::window::Window) -> HelloApplication {
        let env = env::RenderEnv::new(wnd);

        let msaa_samples = render_env::utils::get_max_usable_sample_count(&env);


        let mut swapchain_stuff = render_env::swapchain::SwapChainStuff::new(
            &env,
            wnd.inner_size(),
            msaa_samples
        );

        let render_pass = render_pass::create_render_pass(
            env.device(),
            swapchain_stuff.swapchain_format,
            swapchain_stuff.depth_buffer.format,
            msaa_samples
        );

        swapchain_stuff.create_framebuffers(env.device(), render_pass);

        let ubo_layout = uniform_buffer::create_descriptor_set_layout(env.device());

        let pipeline = pipeline::create_graphics_pipeline(env.device().clone(), render_pass, swapchain_stuff.swapchain_extent, ubo_layout, msaa_samples);

        let vertex_buffer = vertex::VertexBuffer::create(env.instance(), env.physical_device(), env.device().clone(), env.command_pool(), env.queue());
        let uniform_buffers = uniform_buffer::UboBuffers::new(env.instance(), env.device().clone(), env.physical_device(), swapchain_stuff.swapchain_images.len());

        let mut camera = camera::Camera::new();
        camera.set_viewport(swapchain_stuff.swapchain_extent.width, swapchain_stuff.swapchain_extent.height);

        // FIXME: pass me to all other funcs
        let mem_properties =
            unsafe { env.instance().get_physical_device_memory_properties(env.physical_device()) };

        let texture = texture::Texture::new(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &mem_properties,
            Path::new("assets/chalet.jpg"));

        let descriptor_sets = descriptor_set::DescriptorSets::new(
            env.device().clone(),
            swapchain_stuff.swapchain_images.len(),
            ubo_layout,
            &uniform_buffers.uniform_buffers,
            &texture,
        );

        let command_buffers = commands::create_command_buffers(
            env.device(),
            env.command_pool(),
            pipeline.graphics_pipeline,
            &swapchain_stuff.swapchain_framebuffers,
            render_pass,
            swapchain_stuff.swapchain_extent,
            vertex_buffer.vertex_buffer,
            vertex_buffer.index_buffer,
            vertex_buffer.index_count,
            pipeline.pipeline_layout,
            &descriptor_sets.descriptor_sets,
        );

        let sync = sync::create_sync_objects(env.device());

        HelloApplication {
            env,

            swapchain_stuff,
            render_pass,
            ubo_layout,
            pipeline,

            vertex_buffer,

            command_buffers,
            uniform_buffers,
            descriptor_sets,

            texture,

            sync,
            current_frame: 0,
            is_window_resized: false,
            msaa_samples,
            camera,
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

                    print!("FPS: {}\r", tick_counter.fps());
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

            let result = self.swapchain_stuff.swapchain_loader
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
        let signal_semaphores = [self.sync.render_finished_semaphores[self.current_frame]];

        let submit_infos = [vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[image_index as usize],
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
        }];

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
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: ptr::null_mut(),
        };

        let result = unsafe {
            self.swapchain_stuff.swapchain_loader
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

        self.swapchain_stuff = render_env::swapchain::SwapChainStuff::new(
            &self.env,
            wnd.inner_size(),
            self.msaa_samples
        );

        self.pipeline = pipeline::create_graphics_pipeline(
            self.env.device().clone(),
            self.render_pass,
            self.swapchain_stuff.swapchain_extent,
            self.ubo_layout,
            self.msaa_samples,
        );
        self.swapchain_stuff.create_framebuffers(self.env.device(), self.render_pass);

        self.descriptor_sets = descriptor_set::DescriptorSets::new(
            self.env.device().clone(),
            self.swapchain_stuff.swapchain_images.len(),
            self.ubo_layout,
            &self.uniform_buffers.uniform_buffers,
            &self.texture,
        );

        self.command_buffers = commands::create_command_buffers(
            &self.env.device(),
            self.env.command_pool(),
            self.pipeline.graphics_pipeline,
            &self.swapchain_stuff.swapchain_framebuffers,
            self.render_pass,
            self.swapchain_stuff.swapchain_extent,
            self.vertex_buffer.vertex_buffer,
            self.vertex_buffer.index_buffer,
            self.vertex_buffer.index_count,
            self.pipeline.pipeline_layout,
            &self.descriptor_sets.descriptor_sets,
        );
    }

    fn cleanup_swapchain(&mut self) {
        unsafe {
            self.env.device().free_command_buffers(self.env.command_pool(), &self.command_buffers);
            self.descriptor_sets.destroy();
            self.pipeline.destroy();
        }

        self.swapchain_stuff.destroy();
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.env.device().destroy_semaphore(self.sync.image_available_semaphores[i], None);
                self.env.device().destroy_semaphore(self.sync.render_finished_semaphores[i], None);
                self.env.device().destroy_fence(self.sync.inflight_fences[i], None);
            }

            self.cleanup_swapchain();

            self.env.device().destroy_render_pass(self.render_pass, None);

            self.texture.destroy();
            self.env.device().destroy_descriptor_set_layout(self.ubo_layout, None);
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
