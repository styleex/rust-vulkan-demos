use std::path::Path;

use ash::version::DeviceV1_0;
use ash::vk;
use image::GenericImageView;

use crate::utils::texture_utils::{create_image_view, create_texture_image, create_texture_sampler, create_texture_sampler2};


#[allow(dead_code)]
pub struct Texture {
    device: ash::Device,
    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,

    pub texture_image_view: vk::ImageView,
    pub texture_sampler: vk::Sampler,
    _mip_levels: u32,
    format: vk::Format,
}

impl Texture {
    pub fn new(
        device: ash::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        image_path: &Path,
    ) -> Texture {
        let mut image_object = image::open(image_path).unwrap();
        image_object = image_object.flipv();

        let image_data = match &image_object {
            image::DynamicImage::ImageLumaA8(_)
            | image::DynamicImage::ImageBgra8(_)
            | image::DynamicImage::ImageRgba8(_) => image_object.to_rgba8().into_raw(),
            _ => image_object.to_rgba8().into_raw(),
        };

        let (image_width, image_height) = (image_object.width(), image_object.height());

        Texture::from_pixels(device, command_pool, submit_queue, device_memory_properties, vk::Format::R8G8B8A8_SRGB,
                             &image_data, image_width, image_height, true)
    }

    pub fn from_pixels(device: ash::Device,
                       command_pool: vk::CommandPool,
                       submit_queue: vk::Queue,
                       device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
                       format: vk::Format,
                       pixel_data: &Vec<u8>, width: u32, height: u32, create_mips: bool) -> Texture
    {
        let (texture_image, texture_image_memory, mip_levels) = create_texture_image(
            &device, command_pool, submit_queue, device_memory_properties, format, pixel_data, width, height, 1, create_mips);

        let texture_image_view = create_image_view(
            &device, texture_image, format,
            vk::ImageAspectFlags::COLOR, mip_levels, 1);
        let texture_sampler = create_texture_sampler2(&device, mip_levels);

        Texture {
            device,
            texture_image,
            texture_image_memory,
            texture_image_view,
            texture_sampler,
            _mip_levels: mip_levels,
            format,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.texture_sampler, None);
            self.device.destroy_image_view(self.texture_image_view, None);
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_image_memory, None);
        }
    }
}
