use std::ptr;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use crate::texture;
use winit::dpi::PhysicalSize;
use crate::render_env::utils::SwapChainSupportDetail;

pub struct SwapChainStuff {
    device: ash::Device,

    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    pub swapchain_framebuffers: Vec<vk::Framebuffer>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,

    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,
    pub depth_image_format: vk::Format,

    msaa_color_image: vk::Image,
    msaa_color_image_memory: vk::DeviceMemory,
    msaa_color_image_view: vk::ImageView,
}

impl SwapChainStuff {
    pub fn new(
        instance: &ash::Instance,
        device: ash::Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        size: PhysicalSize<u32>,
        msaa_samples: vk::SampleCountFlags,
        swapchain_support: SwapChainSupportDetail,
    ) -> SwapChainStuff {
        let surface_format = choose_swapchain_format(&swapchain_support.formats);
        let present_mode =
            choose_swapchain_present_mode(&swapchain_support.present_modes);
        let extent = choose_swapchain_extent(&swapchain_support.capabilities, size);

        let image_count = swapchain_support.capabilities.min_image_count + 1;
        let image_count = if swapchain_support.capabilities.max_image_count > 0 {
            image_count.min(swapchain_support.capabilities.max_image_count)
        } else {
            image_count
        };

        let queue_family_indices = vec![];
        let swapchain_ci = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: image_count,
            image_color_space: surface_format.color_space,
            image_format: surface_format.format,
            image_extent: extent,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            p_queue_family_indices: queue_family_indices.as_ptr(),
            queue_family_index_count: 0,
            pre_transform: swapchain_support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            image_array_layers: 1,
        };

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, &device);
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_ci, None)
                .expect("Failed to create Swapchain!")
        };

        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .expect("Failed to get Swapchain Images.")
        };

        let image_views = create_image_views(&device, &swapchain_images, surface_format.format);

        let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let depth_image_format = vk::Format::D32_SFLOAT;
        let (depth_image, depth_image_memory) = texture::create_image(
            &device,
            extent.width, extent.height, 1, msaa_samples, depth_image_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &mem_props,
        );

        let depth_image_view = texture::create_image_view(
            &device, depth_image, depth_image_format, vk::ImageAspectFlags::DEPTH, 1);


        let (msaa_color_image, msaa_color_image_memory) = texture::create_image(
            &device,
            extent.width, extent.height,
            1,
            msaa_samples,
            surface_format.format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &mem_props,
        );

        let msaa_color_image_view = texture::create_image_view(
            &device,
            msaa_color_image,
            surface_format.format,
            vk::ImageAspectFlags::COLOR,
            1,
        );

        SwapChainStuff {
            device,
            swapchain_loader,
            swapchain,
            swapchain_format: surface_format.format,
            swapchain_extent: extent,
            swapchain_images,
            image_views,
            swapchain_framebuffers: vec![],

            depth_image,
            depth_image_memory,
            depth_image_view,
            depth_image_format,

            msaa_color_image,
            msaa_color_image_memory,
            msaa_color_image_view,
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }

            for &img_view in &self.image_views {
                self.device.destroy_image_view(img_view, None);
            }

            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_image_memory, None);

            self.device.destroy_image_view(self.msaa_color_image_view, None);
            self.device.destroy_image(self.msaa_color_image, None);
            self.device.free_memory(self.msaa_color_image_memory, None);

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn create_framebuffers(&mut self, device: &ash::Device, render_pass: vk::RenderPass) {
        let mut framebuffers = vec![];

        for &image_view in self.image_views.iter() {
            let attachments = [
                self.msaa_color_image_view,
                self.depth_image_view,
                image_view,
            ];

            let framebuffer_create_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass,
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                width: self.swapchain_extent.width,
                height: self.swapchain_extent.height,
                layers: 1,
            };

            let framebuffer = unsafe {
                device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .expect("Failed to create Framebuffer!")
            };

            framebuffers.push(framebuffer);
        }

        self.swapchain_framebuffers = framebuffers;
    }
}

fn choose_swapchain_format(available_formats: &Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {

    // check if list contains most widely used R8G8B8A8 format with nonlinear color space
    for available_format in available_formats {
        if available_format.format == vk::Format::B8G8R8A8_SRGB
            && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return available_format.clone();
        }
    }

    // return the first format from the list
    return available_formats.first().unwrap().clone();
}

fn choose_swapchain_present_mode(available_present_modes: &Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
    if available_present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
        return vk::PresentModeKHR::MAILBOX;
    }

    return vk::PresentModeKHR::FIFO;
}

fn choose_swapchain_extent(capabilities: &vk::SurfaceCapabilitiesKHR, size: PhysicalSize<u32>) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::max_value() {
        capabilities.current_extent
    } else {
        use num::clamp;

        vk::Extent2D {
            width: clamp(
                size.width,
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: clamp(
                size.height,
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}

fn create_image_views(device: &ash::Device, swapchain_images: &Vec<vk::Image>, swapchain_format: vk::Format) -> Vec<vk::ImageView> {
    let mut ret = Vec::new();

    for &img in swapchain_images {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let view_ci = vk::ImageViewCreateInfo::builder()
            .image(img)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(swapchain_format)
            .subresource_range(subresource_range);

        let image_view = unsafe { device.create_image_view(&view_ci, None).unwrap() };
        ret.push(image_view);
    }

    ret
}
