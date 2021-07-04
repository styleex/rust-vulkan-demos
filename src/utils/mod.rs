use std::ffi::CStr;
use std::os::raw::c_char;
use ash::vk;
use ash::version::InstanceV1_0;

pub mod validation_layer;
pub mod swapchain;
pub mod physical_device;
pub mod surface;
pub mod logical_device;
pub mod pipeline;
pub mod render_pass;
pub mod commands;
pub mod sync;
pub mod vertex;
pub mod uniform_buffer;
pub mod descriptor_set;
pub mod texture;
pub mod buffer_utils;
pub mod platforms;


pub fn vk_to_string(raw_string_array: &[c_char]) -> String {
    let raw_string = unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    };

    raw_string.to_str().expect("Failed to convert vulkan raw string.").to_owned()
}


pub fn get_max_usable_sample_count(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> vk::SampleCountFlags {
    let physical_device_properties =
        unsafe { instance.get_physical_device_properties(physical_device) };

    let count = std::cmp::min(
        physical_device_properties
            .limits
            .framebuffer_color_sample_counts,
        physical_device_properties
            .limits
            .framebuffer_depth_sample_counts,
    );

    if count.contains(vk::SampleCountFlags::TYPE_64) {
        return vk::SampleCountFlags::TYPE_64;
    }
    if count.contains(vk::SampleCountFlags::TYPE_32) {
        return vk::SampleCountFlags::TYPE_32;
    }
    if count.contains(vk::SampleCountFlags::TYPE_16) {
        return vk::SampleCountFlags::TYPE_16;
    }
    if count.contains(vk::SampleCountFlags::TYPE_8) {
        return vk::SampleCountFlags::TYPE_8;
    }
    if count.contains(vk::SampleCountFlags::TYPE_4) {
        return vk::SampleCountFlags::TYPE_4;
    }
    if count.contains(vk::SampleCountFlags::TYPE_2) {
        return vk::SampleCountFlags::TYPE_2;
    }

    vk::SampleCountFlags::TYPE_1
}
