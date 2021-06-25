use ash::vk;
use ash::version::InstanceV1_0;
use crate::{surface, physical_device};


pub fn create_logical_device(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface_stuff: &surface::SurfaceStuff) -> (ash::Device, physical_device::QueueFamilyIndices) {
    let indices = physical_device::find_queue_family(instance, physical_device, surface_stuff);

    let queue_priorities = [1.0_f32];
    let queue_ci = vec!(
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(indices.graphics_family.unwrap())
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

    let device = unsafe { instance.create_device(physical_device, &device_ci, None).unwrap() };

    (device, indices)
}
