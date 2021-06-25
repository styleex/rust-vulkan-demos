use std::ptr;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use winit::window::Window;

use crate::physical_device::QueueFamilyIndices;
use crate::surface::SurfaceStuff;
use crate::texture;


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
}

impl SwapChainStuff {
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

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn create_framebuffers(&mut self, device: &ash::Device, render_pass: vk::RenderPass) {
        let mut framebuffers = vec![];

        for &image_view in self.image_views.iter() {
            let attachments = [image_view, self.depth_image_view];

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

pub struct SwapChainSupportDetail {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}


pub fn query_swapchain_support(physical_device: vk::PhysicalDevice, surface_stuff: &SurfaceStuff) -> SwapChainSupportDetail {
    unsafe {
        let capabilities = surface_stuff
            .surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface_stuff.surface)
            .expect("Failed to query for surface capabilities.");
        let formats = surface_stuff
            .surface_loader
            .get_physical_device_surface_formats(physical_device, surface_stuff.surface)
            .expect("Failed to query for surface formats.");
        let present_modes = surface_stuff
            .surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface_stuff.surface)
            .expect("Failed to query for surface present mode.");

        SwapChainSupportDetail {
            capabilities,
            formats,
            present_modes,
        }
    }
}

// TODO: Remove wnd param, pass extend param instead
pub fn create_swapchain(
    instance: &ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    surface_stuff: &SurfaceStuff,
    queue_family: &QueueFamilyIndices,
    wnd: &Window,
) -> SwapChainStuff {
    let swapchain_support = query_swapchain_support(physical_device, surface_stuff);

    let surface_format = choose_swapchain_format(&swapchain_support.formats);
    let present_mode =
        choose_swapchain_present_mode(&swapchain_support.present_modes);
    let extent = choose_swapchain_extent(&swapchain_support.capabilities, wnd);

    let image_count = swapchain_support.capabilities.min_image_count + 1;
    let image_count = if swapchain_support.capabilities.max_image_count > 0 {
        image_count.min(swapchain_support.capabilities.max_image_count)
    } else {
        image_count
    };

    let (image_sharing_mode, _, queue_family_indices) =
        if queue_family.graphics_family != queue_family.present_family {
            (
                vk::SharingMode::EXCLUSIVE,
                2,
                vec![
                    queue_family.graphics_family.unwrap(),
                    queue_family.present_family.unwrap(),
                ],
            )
        } else {
            (vk::SharingMode::EXCLUSIVE, 0, vec![])
        };

    let swapchain_ci = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface_stuff.surface)
        .min_image_count(image_count)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(extent)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(image_sharing_mode)
        .queue_family_indices(queue_family_indices.as_slice())
        .pre_transform(swapchain_support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null())
        .image_array_layers(1);

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
        extent.width, extent.height, 1, depth_image_format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        &mem_props,
    );

    let depth_image_view = texture::create_image_view(
        &device, depth_image, depth_image_format, vk::ImageAspectFlags::DEPTH, 1);

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

fn choose_swapchain_extent(capabilities: &vk::SurfaceCapabilitiesKHR, wnd: &Window) -> vk::Extent2D {
    let wnd_size = wnd.inner_size();

    if capabilities.current_extent.width != u32::max_value() {
        capabilities.current_extent
    } else {
        use num::clamp;

        vk::Extent2D {
            width: clamp(
                wnd_size.width,
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: clamp(
                wnd_size.height,
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
