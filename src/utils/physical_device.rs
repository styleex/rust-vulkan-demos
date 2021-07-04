use ash;
use ash::vk;
use crate::surface::SurfaceStuff;
use crate::{utils, swapchain, validation_layer};
use std::collections::HashSet;
use ash::version::{InstanceV1_0, EntryV1_0};
use std::ffi::{CString, CStr};
use ash::extensions::khr::XlibSurface;
use ash::extensions::ext::DebugUtils;
use crate::utils::platforms;


pub struct QueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn new() -> QueueFamilyIndices {
        QueueFamilyIndices {
            graphics_family: None,
            present_family: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

pub fn create_instance(entry: &ash::Entry, debug_enabled: bool) -> ash::Instance {
    let app_name = CString::new("test").unwrap();
    let engine_name = CString::new("Vulkan Engine").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(0)
        .engine_name(&engine_name)
        .engine_version(0)
        .api_version(vk::make_version(1, 0, 0));

    let extension_names = platforms::required_extension_names();

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

fn is_phys_device_suitable(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface_stuff: &SurfaceStuff) -> bool {
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

    let indices = find_queue_family(instance, physical_device, surface_stuff);

    let is_device_extension_supported = check_device_extension_support(instance, physical_device, vec!(String::from("VK_KHR_swapchain")));
    let is_swapchain_supported = if is_device_extension_supported {
        let swapchain_support = swapchain::query_swapchain_support(physical_device, surface_stuff);
        !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
    } else {
        false
    };

    return indices.is_complete() && is_device_extension_supported && is_swapchain_supported;
}

fn check_device_extension_support(instance: &ash::Instance, physical_device: vk::PhysicalDevice, extensions: Vec<String>) -> bool {
    let available_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .expect("Failed to get device extension properties.")
    };

    let mut available_extension_names = vec![];

    println!("\tAvailable Device Extensions: ");
    for extension in available_extensions.iter() {
        let extension_name = utils::vk_to_string(&extension.extension_name);
        println!("\t\tName: {}, Version: {}", extension_name, extension.spec_version);

        available_extension_names.push(extension_name);
    }

    let mut required_extensions = HashSet::new();
    for extension in extensions {
        required_extensions.insert(extension);
    }

    for extension_name in available_extension_names.iter() {
        required_extensions.remove(extension_name);
    }

    return required_extensions.is_empty();
}

pub fn find_queue_family(instance: &ash::Instance, physical_device: vk::PhysicalDevice,
                     surface_stuff: &SurfaceStuff) -> QueueFamilyIndices {
    let queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let mut queue_family_indices = QueueFamilyIndices::new();

    let mut index = 0;

    for queue_family in queue_families.iter() {
        if queue_family.queue_count > 0 && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            queue_family_indices.graphics_family = Some(index);
        }

        let is_present_support = unsafe {
            surface_stuff
                .surface_loader
                .get_physical_device_surface_support(
                    physical_device,
                    index as u32,
                    surface_stuff.surface,
                ).unwrap()
        };
        if queue_family.queue_count > 0 && is_present_support {
            queue_family_indices.present_family = Some(index);
        }

        if queue_family_indices.is_complete() {
            break;
        }

        index += 1;
    }

    queue_family_indices
}

pub fn pick_physical_device(instance: &ash::Instance, surface_stuff: &SurfaceStuff) -> vk::PhysicalDevice {
    let devices = unsafe { instance.enumerate_physical_devices().unwrap() };

    let mut ret_device: Option<vk::PhysicalDevice> = None;
    for device in devices {
        if is_phys_device_suitable(&instance, device, surface_stuff) {
            ret_device = Some(device)
        }
    }

    ret_device.expect("failed to find suitable GPU!")
}
