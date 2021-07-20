use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;
use winit::dpi::PhysicalSize;

use crate::render_env::{utils};
use crate::render_env::env::RenderEnv;

pub struct SwapChain {
    device: ash::Device,
    pub swapchain_api: ash::extensions::khr::Swapchain,

    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub format: vk::Format,
    pub size: vk::Extent2D,
}

impl SwapChain {
    pub fn new(
        env: &RenderEnv, size: PhysicalSize<u32>,
    ) -> SwapChain
    {
        let swapchain_support = utils::SwapChainSupportDetail::load(&env);

        let swapchain_format = swapchain_support.format();
        let extent = swapchain_support.adjust_extent(size);

        let queue_family_indices = vec![];
        let swapchain_ci = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface: env.surface,
            min_image_count: swapchain_support.get_image_count(),
            image_color_space: swapchain_format.color_space,
            image_format: swapchain_format.format,
            image_extent: extent,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            p_queue_family_indices: queue_family_indices.as_ptr(),
            queue_family_index_count: 0,
            pre_transform: swapchain_support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: swapchain_support.present_mode(),
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            image_array_layers: 1,
        };

        let swapchain_api = ash::extensions::khr::Swapchain::new(env.instance(), env.device());
        let swapchain = unsafe {
            swapchain_api
                .create_swapchain(&swapchain_ci, None)
                .expect("Failed to create Swapchain!")
        };

        let swapchain_images = unsafe {
            swapchain_api
                .get_swapchain_images(swapchain)
                .expect("Failed to get Swapchain Images.")
        };

        let mut image_views = Vec::new();
        for &img in swapchain_images.iter() {
            let view_ci = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                image: img,
                view_type: vk::ImageViewType::TYPE_2D,
                format: swapchain_format.format,
                components: Default::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };

            let image_view = unsafe { env.device().create_image_view(&view_ci, None).unwrap() };
            image_views.push(image_view);
        }

        SwapChain {
            device: env.device().clone(),
            swapchain_api,
            swapchain,
            format: swapchain_format.format,
            size: extent,
            images: swapchain_images,
            image_views,
            framebuffers: vec![],
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            for &framebuffer in self.framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }

            for &img_view in &self.image_views {
                self.device.destroy_image_view(img_view, None);
            }

            self.swapchain_api.destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn create_framebuffers(&mut self, device: &ash::Device, render_pass: vk::RenderPass) {
        let mut framebuffers = vec![];

        for &image_view in self.image_views.iter() {
            let attachments = [
                image_view,
            ];

            let framebuffer_create_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass,
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                width: self.size.width,
                height: self.size.height,
                layers: 1,
            };

            let framebuffer = unsafe {
                device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .expect("Failed to create Framebuffer!")
            };

            framebuffers.push(framebuffer);
        }

        self.framebuffers = framebuffers;
    }
}
