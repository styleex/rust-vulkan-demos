use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent, VirtualKeyCode};
use ash::version::{EntryV1_0, InstanceV1_0};
use ash::vk;
use std::ffi::CString;
use std::ptr;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::extensions::khr::XlibSurface;

struct HelloApplication {
    entry: ash::Entry,
    instance: ash::Instance,
}

impl HelloApplication {
    pub fn new() -> HelloApplication {
        let entry = unsafe{ ash::Entry::new().unwrap() };
        let instance = HelloApplication::create_instance(&entry);

        HelloApplication {
            entry,
            instance
        }
    }

    pub fn run(&self) {
        let event_loop = EventLoop::new();

        let wnd = winit::window::WindowBuilder::new()
            .with_title("test")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
            .build(&event_loop)
            .expect("Failed to create window");

        event_loop.run(move |event, target, control_flow| {
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
                _ => ()
            }
        })
    }

    fn create_instance(entry: &ash::Entry) -> ash::Instance {
        let app_name = CString::new("test").unwrap();
        let engine_name = CString::new("Vulkan Engine").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&engine_name)
            .engine_version(0)
            .api_version(vk::make_version(1, 0, 0));

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();
        let extension_names = vec![Surface::name().as_ptr(), XlibSurface::name().as_ptr(), DebugUtils::name().as_ptr()];

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names);

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
            self.instance.destroy_instance(None)
        }
    }
}


fn main() {
    let app = HelloApplication::new();
    app.run();
}
