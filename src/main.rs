use std::ptr;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;

use utils::{commands, logical_device, physical_device, pipeline, render_pass, surface,
            swapchain, sync, validation_layer, vertex, ubo};

use crate::utils::physical_device::QueueFamilyIndices;
use crate::utils::sync::MAX_FRAMES_IN_FLIGHT;

mod utils;

// FIXME: Последняя синхронизация из тутора не сделана;
struct HelloApplication {
    debug_enabled: bool,

    _entry: ash::Entry,
    instance: ash::Instance,

    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,

    family_indices: QueueFamilyIndices,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    physical_device: vk::PhysicalDevice,

    // Logical device
    device: ash::Device,

    surface_stuff: surface::SurfaceStuff,
    swapchain_stuff: swapchain::SwapChainStuff,

    render_pass: vk::RenderPass,
    ubo_layout: vk::DescriptorSetLayout,
    pipeline: pipeline::Pipeline,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    vertex_buffer: vertex::VertexBuffer,
    uniform_buffers: ubo::UboBuffers,
    sync: sync::SyncObjects,

    current_frame: usize,
    is_window_resized: bool,
}

impl HelloApplication {
    pub fn new(wnd: &winit::window::Window, debug_enabled: bool) -> HelloApplication {
        let entry = unsafe { ash::Entry::new().unwrap() };
        if debug_enabled {
            println!("Debug enabled");

            if !validation_layer::check_validation_layer_support(&entry) {
                panic!("Validation layers requested, but not available");
            }
        } else {
            println!("Debug disabled");
        }

        let instance = physical_device::create_instance(&entry, debug_enabled);
        let (debug_utils_loader, debug_messenger) =
            validation_layer::setup_debug_utils(&entry, &instance, debug_enabled);

        let surface_stuff = surface::create_surface(&entry, &instance, wnd);

        let physical_device = physical_device::pick_physical_device(&instance, &surface_stuff);
        let (device, family_indices) = logical_device::create_logical_device(&instance, physical_device, &surface_stuff);

        let graphics_queue =
            unsafe { device.get_device_queue(family_indices.graphics_family.unwrap(), 0) };
        let present_queue =
            unsafe { device.get_device_queue(family_indices.present_family.unwrap(), 0) };

        let mut swapchain_stuff = swapchain::create_swapchain(&instance, device.clone(), physical_device,
                                                              &surface_stuff, &family_indices, wnd);

        let render_pass = render_pass::create_render_pass(&device, swapchain_stuff.swapchain_format);
        swapchain_stuff.create_framebuffers(&device, render_pass);

        let ubo_layout = ubo::create_descriptor_set_layout(&device);
        let pipeline = pipeline::create_graphics_pipeline(device.clone(), render_pass, swapchain_stuff.swapchain_extent, ubo_layout);

        let command_pool = commands::create_command_pool(&device, family_indices.graphics_family.unwrap());

        let vertex_buffer = vertex::VertexBuffer::create(&instance, physical_device, device.clone(), command_pool, graphics_queue);
        let command_buffers = commands::create_command_buffers(
            &device,
            command_pool,
            pipeline.graphics_pipeline,
            &swapchain_stuff.swapchain_framebuffers,
            render_pass,
            swapchain_stuff.swapchain_extent,
            vertex_buffer.vertex_buffer,
            vertex_buffer.index_buffer,
        );
        let uniform_buffers = ubo::UboBuffers::new(&instance, device.clone(), physical_device, swapchain_stuff.swapchain_images.len(), swapchain_stuff.swapchain_extent);

        let sync = sync::create_sync_objects(&device);

        HelloApplication {
            debug_enabled,
            instance,
            debug_utils_loader,
            debug_messenger,
            device,

            surface_stuff,

            _entry: entry,

            family_indices,
            graphics_queue,
            present_queue,
            physical_device,

            swapchain_stuff,
            render_pass,
            ubo_layout,
            pipeline,

            vertex_buffer,

            command_pool,
            command_buffers,
            uniform_buffers,

            sync,
            current_frame: 0,
            is_window_resized: false,
        }
    }

    pub fn run(&mut self, mut event_loop: EventLoop<()>, wnd: winit::window::Window) {
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
                            *control_flow = ControlFlow::Exit
                        }

                        return;
                    }

                    if let WindowEvent::Resized(_) = event {
                        self.is_window_resized = true;
                    }
                }
                Event::MainEventsCleared => {
                    wnd.request_redraw()
                }
                Event::RedrawRequested(_) => {
                    self.draw_frame(&wnd, 0.0);
                }
                // Important!
                Event::LoopDestroyed => {
                    unsafe { self.device.device_wait_idle().unwrap(); }
                }
                _ => (),
            }
        })
    }

    fn draw_frame(&mut self, wnd: &winit::window::Window, delta_time: f32) {
        let wait_fences = [self.sync.inflight_fences[self.current_frame]];

        let (image_index, _is_sub_optimal) = unsafe {
            self.device
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
        self.uniform_buffers.update_uniform_buffer(image_index as usize, delta_time);
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
            self.device
                .reset_fences(&wait_fences)
                .expect("Failed to reset Fence!");

            self.device
                .queue_submit(
                    self.graphics_queue,
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
                .queue_present(self.present_queue, &present_info)
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
            self.device
                .device_wait_idle()
                .expect("Failed to wait device idle!")
        };
        self.cleanup_swapchain();

        self.swapchain_stuff = swapchain::create_swapchain(
            &self.instance,
            self.device.clone(),
            self.physical_device,
            &self.surface_stuff,
            &self.family_indices,
            wnd,
        );

        self.render_pass = render_pass::create_render_pass(&self.device, self.swapchain_stuff.swapchain_format);
        self.pipeline = pipeline::create_graphics_pipeline(
            self.device.clone(),
            self.render_pass,
            self.swapchain_stuff.swapchain_extent,
            self.ubo_layout,
        );
        self.swapchain_stuff.create_framebuffers(&self.device, self.render_pass);

        self.command_buffers = commands::create_command_buffers(
            &self.device,
            self.command_pool,
            self.pipeline.graphics_pipeline,
            &self.swapchain_stuff.swapchain_framebuffers,
            self.render_pass,
            self.swapchain_stuff.swapchain_extent,
            self.vertex_buffer.vertex_buffer,
            self.vertex_buffer.index_buffer,
        );
    }

    fn cleanup_swapchain(&mut self) {
        unsafe {
            self.device.free_command_buffers(self.command_pool, &self.command_buffers);

            self.pipeline.destroy();
            self.device.destroy_render_pass(self.render_pass, None);
        }

        self.swapchain_stuff.destroy();
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device.destroy_semaphore(self.sync.image_available_semaphores[i], None);
                self.device.destroy_semaphore(self.sync.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.sync.inflight_fences[i], None);
            }

            self.cleanup_swapchain();
            self.device.destroy_descriptor_set_layout(self.ubo_layout, None);
            self.uniform_buffers.destroy();
            self.vertex_buffer.destroy();
            self.device.destroy_command_pool(self.command_pool, None);

            if self.debug_enabled {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
            }

            self.device.destroy_device(None);
            self.surface_stuff.surface_loader.destroy_surface(self.surface_stuff.surface, None);

            self.instance.destroy_instance(None)
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

    let mut app = HelloApplication::new(&wnd, true);
    app.run(event_loop, wnd);
}
