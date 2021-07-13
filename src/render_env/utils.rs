use ash::version::InstanceV1_0;
use ash::vk;

use super::env::RenderEnv;
use winit::dpi::PhysicalSize;

pub fn get_max_usable_sample_count(env: &RenderEnv) -> vk::SampleCountFlags {
    let physical_device_properties =
        unsafe { env.instance.get_physical_device_properties(env.physical_device) };

    let max_sample_count = std::cmp::min(
        physical_device_properties
            .limits
            .framebuffer_color_sample_counts,
        physical_device_properties
            .limits
            .framebuffer_depth_sample_counts,
    );

    let all_samples = [
        vk::SampleCountFlags::TYPE_64,
        vk::SampleCountFlags::TYPE_32,
        vk::SampleCountFlags::TYPE_16,
        vk::SampleCountFlags::TYPE_8,
        vk::SampleCountFlags::TYPE_4,
        vk::SampleCountFlags::TYPE_2,
    ];

    for candidate in all_samples {
        if max_sample_count.contains(candidate) {
            return candidate;
        }
    }

    vk::SampleCountFlags::TYPE_1
}


#[derive(Clone)]
pub struct SwapChainSupportDetail {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetail {
    pub fn load(env: &RenderEnv) -> SwapChainSupportDetail {
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

    pub fn format(&self) -> vk::SurfaceFormatKHR {
        for available_format in self.formats.iter() {
            if available_format.format == vk::Format::B8G8R8A8_SRGB
                && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return available_format.clone();
            }
        }

        // return the first format from the list
        return self.formats.first().unwrap().clone();
    }

    pub fn present_mode(&self) -> vk::PresentModeKHR {
        if self.present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            return vk::PresentModeKHR::MAILBOX;
        }

        return vk::PresentModeKHR::FIFO;
    }

    pub fn adjust_extent(&self, size: PhysicalSize<u32>) -> vk::Extent2D {
        if self.capabilities.current_extent.width != u32::MAX {
            self.capabilities.current_extent
        } else {
            use num::clamp;

            vk::Extent2D {
                width: clamp(
                    size.width,
                    self.capabilities.min_image_extent.width,
                    self.capabilities.max_image_extent.width,
                ),
                height: clamp(
                    size.height,
                    self.capabilities.min_image_extent.height,
                    self.capabilities.max_image_extent.height,
                ),
            }
        }
    }

    pub fn get_image_count(&self) -> u32 {
        let image_count = self.capabilities.min_image_count + 1;

        if self.capabilities.max_image_count > 0 {
            image_count.min(self.capabilities.max_image_count)
        } else {
            image_count
        }
    }
}


#[inline]
pub fn format_has_depth(format: vk::Format) -> bool {
    [
        vk::Format::D16_UNORM,
        vk::Format::X8_D24_UNORM_PACK32,
        vk::Format::D32_SFLOAT,
        vk::Format::S8_UINT,
        vk::Format::D16_UNORM_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
        vk::Format::D32_SFLOAT_S8_UINT,
    ].contains(&format)
}
