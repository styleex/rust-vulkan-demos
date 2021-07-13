use ash::version::InstanceV1_0;
use ash::vk;

use super::env::RenderEnv;

pub fn get_max_usable_sample_count(env: &RenderEnv) -> vk::SampleCountFlags {
    let physical_device_properties =
        unsafe { env.instance.get_physical_device_properties(env.physical_device) };

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


#[derive(Clone)]
pub struct SwapChainSupportDetail {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub fn query_swapchain_support(env: &RenderEnv) -> SwapChainSupportDetail {
    unsafe {
        let capabilities = env
            .surface_loader
            .get_physical_device_surface_capabilities(env.physical_device, env.surface)
            .expect("Failed to query for surface capabilities.");
        let formats = env
            .surface_loader
            .get_physical_device_surface_formats(env.physical_device, env.surface)
            .expect("Failed to query for surface formats.");
        let present_modes = env
            .surface_loader
            .get_physical_device_surface_present_modes(env.physical_device, env.surface)
            .expect("Failed to query for surface present mode.");

        SwapChainSupportDetail {
            capabilities,
            formats,
            present_modes,
        }
    }
}
