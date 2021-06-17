use std::ffi::{CStr, CString};

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, XlibSurface};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod utils;
mod validation_layer;

struct QueueFamilyIndices {
    graphics_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some()
    }
}

struct HelloApplication {
    debug_enabled: bool,
    _entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    device: ash::Device, // Logical device
    _graphics_queue: vk::Queue,
    _physical_device: vk::PhysicalDevice,
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

        let physical_device = HelloApplication::pick_physical_device(&instance);
        let (device, graphics_queue) = HelloApplication::create_logical_device(&instance, physical_device);

        HelloApplication {
            debug_enabled,
            instance,
            debug_utils_loader,
            debug_messenger,
            device,

            _entry: entry,
            _graphics_queue: graphics_queue,
            _physical_device: physical_device,
        }
    }

    fn is_phys_device_suitable(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> bool {
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_features = unsafe { instance.get_physical_device_features(physical_device) };
        let device_queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let device_type = match device_properties.device_type {
            vk::PhysicalDeviceType::CPU => "Cpu",
            vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU",
            vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
            vk::PhysicalDeviceType::OTHER => "Unknown",
            _ => panic!(),
        };

        let device_name = utils::vk_to_string(&device_properties.device_name);
        println!("\tDevice Name: {}, id: {}, type: {}", device_name, device_properties.device_id, device_type);

        let major_version = vk::version_major(device_properties.api_version);
        let minor_version = vk::version_minor(device_properties.api_version);
        let patch_version = vk::version_patch(device_properties.api_version);

        println!("\tAPI Version: {}.{}.{}", major_version, minor_version, patch_version);

        println!("\tSupport Queue Family: {}", device_queue_families.len());
        println!("\t\tQueue Count | Graphics, Compute, Transfer, Sparse Binding");
        for queue_family in device_queue_families.iter() {
            let is_graphics_support = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let is_compute_support = queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE);
            let is_transfer_support = queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER);
            let is_sparse_support = queue_family.queue_flags.contains(vk::QueueFlags::SPARSE_BINDING);

            println!(
                "\t\t{}\t    | {},     {},    {},     {}", queue_family.queue_count,
                is_graphics_support, is_compute_support, is_transfer_support, is_sparse_support
            );
        }

        // there are plenty of features
        println!("\tGeometry Shader support: {}", device_features.geometry_shader == 1);

        let indices = HelloApplication::find_queue_family(instance, physical_device);
        return indices.is_complete();
    }

    fn find_queue_family(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> QueueFamilyIndices {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut queue_family_indices = QueueFamilyIndices {
            graphics_family: None,
        };

        let mut index = 0;
        for queue_family in queue_families.iter() {
            if queue_family.queue_count > 0
                && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            {
                queue_family_indices.graphics_family = Some(index);
            }

            if queue_family_indices.is_complete() {
                break;
            }

            index += 1;
        }

        queue_family_indices
    }

    fn pick_physical_device(instance: &ash::Instance) -> vk::PhysicalDevice {
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };

        let mut ret_device: Option<vk::PhysicalDevice> = None;
        for device in devices {
            if HelloApplication::is_phys_device_suitable(&instance, device) {
                ret_device = Some(device)
            }
        }

        ret_device.expect("failed to find suitable GPU!")
    }

    fn create_logical_device(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> (ash::Device, vk::Queue) {
        let indices = HelloApplication::find_queue_family(instance, physical_device);

        let queue_priorities = [1.0_f32];
        let queue_ci = vec!(
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(indices.graphics_family.unwrap())
                .queue_priorities(&queue_priorities).build()
        );

        let mut device_ci = vk::DeviceCreateInfo::builder()
            .queue_create_infos(queue_ci.as_slice());

        let device = unsafe { instance.create_device(physical_device, &device_ci, None).unwrap() };
        let graphics_queue = unsafe { device.get_device_queue(indices.graphics_family.unwrap(), 0) };

        (device, graphics_queue)
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
            let layer_name = utils::vk_to_string(&layer.layer_name);

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
            self.device.destroy_device(None);

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
