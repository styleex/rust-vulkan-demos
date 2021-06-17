use std::ffi::{CStr, CString};

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, XlibSurface};
use ash::version::{EntryV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::utils::vk_to_string;

mod utils;
mod validation_layer;

struct HelloApplication {
    debug_enabled: bool,
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl HelloApplication {
    pub fn new(debug_enabled: bool) -> HelloApplication {
        let entry = unsafe { ash::Entry::new().unwrap() };
        if debug_enabled {
            println!("Debug enabled");

            if !HelloApplication::check_validation_layer_support(&entry) {
                panic!("Validation layers requested, but not available");
            }
        } else {
            println!("Debug disabled");
        }

        let instance = HelloApplication::create_instance(&entry, debug_enabled);
        let (debug_utils_loader, debug_messenger) =
            validation_layer::setup_debug_utils(&entry, &instance, debug_enabled);

        HelloApplication {
            debug_enabled,
            entry,
            instance,
            debug_utils_loader,
            debug_messenger,
        }
    }

    pub fn run(&self) {
        let event_loop = EventLoop::new();

        let wnd = winit::window::WindowBuilder::new()
            .with_title("test")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
            .build(&event_loop)
            .expect("Failed to create window");

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
                        if input.virtual_keycode.unwrap() == VirtualKeyCode::Escape {
                            *control_flow = ControlFlow::Exit
                        }

                        return;
                    }
                }
                _ => (),
            }
        })
    }

    fn check_validation_layer_support(entry: &ash::Entry) -> bool {
        let layers = entry.enumerate_instance_layer_properties().unwrap();
        println!("Available layers:");

        let mut layer_names = Vec::new();
        for layer in layers.iter() {
            let layer_name = vk_to_string(&layer.layer_name);

            println!("\t{}", layer_name);
            layer_names.push(layer_name);
        }

        return layer_names.contains(&"VK_LAYER_KHRONOS_validation".to_string());
    }

    fn create_instance(entry: &ash::Entry, debug_enabled: bool) -> ash::Instance {
        let app_name = CString::new("test").unwrap();
        let engine_name = CString::new("Vulkan Engine").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&engine_name)
            .engine_version(0)
            .api_version(vk::make_version(1, 0, 0));

        let extension_names = vec![
            Surface::name().as_ptr(),
            XlibSurface::name().as_ptr(),
            DebugUtils::name().as_ptr(),
        ];

        let mut debug_utils_create_info = validation_layer::populate_debug_messenger_create_info();
        let debug_layers = vec![CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0")
            .unwrap()
            .as_ptr()];

        let mut create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);

        if debug_enabled {
            create_info = create_info
                .push_next(&mut debug_utils_create_info)
                .enabled_layer_names(debug_layers.as_slice());
        }

        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create instance!")
        };

        instance
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            if self.debug_enabled {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None)
            }
            self.instance.destroy_instance(None)
        }
    }
}

fn main() {
    let app = HelloApplication::new(true);
    app.run();
}
