use std::ptr;

use ash::version::{DeviceV1_0};
use ash::vk;

use winit::dpi::PhysicalSize;
use crate::render_env::env::RenderEnv;
use crate::render_env::{attachment_texture, utils};

pub struct SwapChainStuff {
    device: ash::Device,

    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    pub swapchain_framebuffers: Vec<vk::Framebuffer>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,


    pub depth_buffer: attachment_texture::AttachmentImage,
    pub msaa_buffer: attachment_texture::AttachmentImage,
}

impl SwapChainStuff {
    pub fn new(
        env: &RenderEnv, size: PhysicalSize<u32>, msaa_samples: vk::SampleCountFlags
    ) -> SwapChainStuff
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

        let swapchain_loader = ash::extensions::khr::Swapchain::new(env.instance(), env.device());
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

        let image_views = create_image_views(env.device(), &swapchain_images, swapchain_format.format);

        let depth_buffer = attachment_texture::AttachmentImage::new(
            env,
            [extent.width, extent.height],
            vk::Format::D32_SFLOAT,
            1,
            msaa_samples,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        );

        // FIXME: required memory props=vk::MemoryPropertyFlags::DEVICE_LOCAL
        let msaa_buffer = attachment_texture::AttachmentImage::new(
            env,
            [extent.width, extent.height],
            swapchain_format.format,
            1,
            msaa_samples,
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
        );

        SwapChainStuff {
            device: env.device().clone(),
            swapchain_loader,
            swapchain,
            swapchain_format: swapchain_format.format,
            swapchain_extent: extent,
            swapchain_images,
            image_views,
            swapchain_framebuffers: vec![],

            depth_buffer,
            msaa_buffer,
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }

            self.msaa_buffer.destroy();
            self.depth_buffer.destroy();

            for &img_view in &self.image_views {
                self.device.destroy_image_view(img_view, None);
            }

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn create_framebuffers(&mut self, device: &ash::Device, render_pass: vk::RenderPass) {
        let mut framebuffers = vec![];

        for &image_view in self.image_views.iter() {
            let attachments = [
                self.msaa_buffer.image_view,
                self.depth_buffer.image_view,
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
