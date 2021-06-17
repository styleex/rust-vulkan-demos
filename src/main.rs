use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod utils;
use utils::{validation_layer, surface, logical_device, physical_device, swapchain};


struct HelloApplication {
    debug_enabled: bool,
    _entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,

    // Logical device
    device: ash::Device,

    _graphics_queue: vk::Queue,
    _present_queue: vk::Queue,
    _physical_device: vk::PhysicalDevice,

    surface_stuff: utils::surface::SurfaceStuff,

    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    _swapchain_images: Vec<vk::Image>,
    _swapchain_format: vk::Format,
    _swapchain_extent: vk::Extent2D,
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
        let swapchain_stuff = swapchain::create_swapchain(&instance, &device, physical_device,
                                                          &surface_stuff, &family_indices, wnd);

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

            swapchain_loader: swapchain_stuff.swapchain_loader,
            swapchain: swapchain_stuff.swapchain,
            _swapchain_format: swapchain_stuff.swapchain_format,
            _swapchain_images: swapchain_stuff.swapchain_images,
            _swapchain_extent: swapchain_stuff.swapchain_extent,
        }
    }

    pub fn run(&self, event_loop: EventLoop<()>, wnd: winit::window::Window) {
        event_loop.run(move |event, _, control_flow| {
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
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_stuff.surface_loader.destroy_surface(self.surface_stuff.surface, None);

            if self.debug_enabled {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None)
            }
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
