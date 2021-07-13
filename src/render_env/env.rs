use core::ptr;
use std::ffi::{c_void, CStr, CString};

use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;
use ash::vk::{ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCreateFlagsEXT, DebugUtilsMessengerCreateInfoEXT};
use winit::window::Window;

use crate::utils::platforms;

#[allow(dead_code)]
pub struct RenderEnv {
    entry: ash::Entry,
    pub(super) instance: ash::Instance,
    pub(super) physical_device: vk::PhysicalDevice,

    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    pub(super) mem_properties: vk::PhysicalDeviceMemoryProperties,
    device: ash::Device,
    queue: vk::Queue,

    command_pool: vk::CommandPool,

    pub(super) surface_loader: ash::extensions::khr::Surface,
    pub(super) surface: vk::SurfaceKHR,
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    message_type: DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message = CStr::from_ptr((*p_callback_data).p_message);
    println!("[Debug]{}{}{:?}", severity, types, message);

    vk::FALSE
}


#[allow(dead_code)]
impl RenderEnv {
    pub fn new(window: &Window) -> RenderEnv {
        unsafe {
            let app_name = CString::new("test").unwrap();
            let engine_name = CString::new("Vulkan Engine").unwrap();

            let app_info = ApplicationInfo::builder()
                .application_name(&app_name)
                .application_version(0)
                .engine_name(&engine_name)
                .engine_version(0)
                .api_version(vk::make_version(1, 0, 0));

            let extension_names = platforms::required_extension_names();

            let mut debug_utils_create_info = DebugUtilsMessengerCreateInfoEXT {
                s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                p_next: ptr::null(),
                flags: DebugUtilsMessengerCreateFlagsEXT::empty(),
                message_severity: DebugUtilsMessageSeverityFlagsEXT::WARNING |
                    // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
                    // vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
                    DebugUtilsMessageSeverityFlagsEXT::ERROR,
                message_type: DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                pfn_user_callback: Some(vulkan_debug_utils_callback),
                p_user_data: ptr::null_mut(),
            };

            let debug_layers = vec![CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0")
                .unwrap()
                .as_ptr()];

            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(&extension_names)
                .push_next(&mut debug_utils_create_info)
                .enabled_layer_names(debug_layers.as_slice());

            let entry = ash::Entry::new().unwrap();
            let instance: ash::Instance = entry
                .create_instance(&create_info, None)
                .expect("Failed to create instance!");

            // loaders
            let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

            let debug_messenger = debug_utils_loader
                .create_debug_utils_messenger(&debug_utils_create_info, None)
                .expect("Debug Utils Callback");

            let surface = platforms::create_surface(&entry, &instance, &window).unwrap();
            let pdevices = instance.enumerate_physical_devices().unwrap();
            let (physical_device, queue_family_index) = pdevices
                .iter()
                .map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .filter_map(|(index, ref info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                    .get_physical_device_surface_support(
                                        *pdevice,
                                        index as u32,
                                        surface,
                                    )
                                    .unwrap();

                            if supports_graphic_and_surface {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                        .next()
                })
                .flatten()
                .next()
                .expect("Couldn't find suitable device.");

            let mem_properties= instance.get_physical_device_memory_properties(physical_device);
            let queue_family_index = queue_family_index as u32;

            // logical device
            let queue_priorities = [1.0_f32];
            let queue_ci = vec!(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(0)
                    .queue_priorities(&queue_priorities).build()
            );

            let enable_extension_names = [
                ash::extensions::khr::Swapchain::name().as_ptr(), // currently just enable the Swapchain extension.
            ];
            let physical_device_features = vk::PhysicalDeviceFeatures {
                sampler_anisotropy: vk::TRUE, // enable anisotropy device feature from Chapter-24.
                sample_rate_shading: vk::TRUE,
                ..Default::default()
            };

            let device_ci = vk::DeviceCreateInfo::builder()
                .queue_create_infos(queue_ci.as_slice())
                .enabled_extension_names(&enable_extension_names)
                .enabled_features(&physical_device_features);

            let device = instance.create_device(physical_device, &device_ci, None).unwrap();
            let queue = device.get_device_queue(queue_family_index, 0);

            let command_pool_create_info = vk::CommandPoolCreateInfo {
                s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::CommandPoolCreateFlags::empty(),
                queue_family_index: queue_family_index,
            };

            let command_pool = device
                .create_command_pool(&command_pool_create_info, None)
                .expect("Failed to create Command Pool!");

            RenderEnv {
                entry,
                instance,
                physical_device,

                surface,
                surface_loader,

                device,
                mem_properties,
                queue,

                command_pool,

                debug_utils_loader,
                debug_messenger,
            }
        }
    }

    pub(crate) fn find_memory_type(&self, type_filter: u32, required_properties: vk::MemoryPropertyFlags) -> u32 {
        for (i, memory_type) in self.mem_properties.memory_types.iter().enumerate() {
            if (type_filter & (1 << i)) > 0
                && memory_type.property_flags.contains(required_properties)
            {
                return i as u32;
            }
        }

        panic!("Failed to find suitable memory type!")
    }


    #[inline]
    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    #[inline]
    pub fn device(&self) -> &ash::Device {
        &self.device
    }

    #[inline]
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    #[inline]
    pub fn surface(&self) -> vk::SurfaceKHR {
        self.surface.clone()
    }

    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool.clone()
    }

    pub fn queue(&self) -> vk::Queue {
        self.queue.clone()
    }
}

impl Drop for RenderEnv {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);

            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);

            self.instance.destroy_instance(None);
        }
    }
}
