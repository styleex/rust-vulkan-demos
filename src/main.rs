use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;

use utils::{logical_device, physical_device, pipeline, surface, swapchain, validation_layer, render_pass};

mod utils;

struct HelloApplication {
    debug_enabled: bool,

    _entry: ash::Entry,
    instance: ash::Instance,

    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,

    _graphics_queue: vk::Queue,
    _present_queue: vk::Queue,
    _physical_device: vk::PhysicalDevice,

    // Logical device
    device: ash::Device,

    surface_stuff: surface::SurfaceStuff,
    swapchain_stuff: swapchain::SwapChainStuff,

    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
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

        let (graphics_pipeline, pipeline_layout) = pipeline::create_graphics_pipeline(&device, render_pass, swapchain_stuff.swapchain_extent);

        HelloApplication {
            debug_enabled,
            instance,
            debug_utils_loader,
            debug_messenger,
            device,

            surface_stuff,

            _entry: entry,

            _graphics_queue: graphics_queue,
            _present_queue: present_queue,
            _physical_device: physical_device,

            swapchain_stuff,
            render_pass,
            pipeline_layout,
            graphics_pipeline,
        }
    }

    pub fn run(&self, mut event_loop: EventLoop<()>, wnd: winit::window::Window) {
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
                }
                _ => (),
            }
        })
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.swapchain_stuff.destroy();

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

    let app = HelloApplication::new(&wnd, true);
    app.run(event_loop, wnd);
}
