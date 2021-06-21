use std::ffi::CStr;
use std::os::raw::c_char;

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
pub mod ubo;

pub fn vk_to_string(raw_string_array: &[c_char]) -> String {
    let raw_string = unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    };

    raw_string.to_str().expect("Failed to convert vulkan raw string.").to_owned()
}
