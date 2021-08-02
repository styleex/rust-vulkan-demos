use std::path::Path;

use ash::version::{DeviceV1_0};
use ash::vk;
use image::GenericImageView;

use crate::utils::texture_utils::{create_image_view, create_texture_image, create_texture_sampler};


#[allow(dead_code)]
pub struct CubeTexture {
    device: ash::Device,
    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,

    pub texture_image_view: vk::ImageView,
    pub texture_sampler: vk::Sampler,
    _mip_levels: u32,
    format: vk::Format,
}


impl CubeTexture {
    pub fn new(
        device: ash::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        image_path: &Path,
    ) -> CubeTexture {
        // Face order: +X, -X, +Y, -Y, +Z, -Z
        // FROM: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkImageSubresourceRange.html#_description
        let faces = [
            "right.jpg",
            "left.jpg",
            "top.jpg",
            "bottom.jpg",
            "front.jpg",
            "back.jpg",
        ];

        let mut initialized = false;
        let mut image_width = 0;
        let mut image_height = 0;
        let mut image_array_data = Vec::new();

        for face in faces.iter() {
            let image_object = image::open(image_path.join(face)).unwrap();

            let image_data = match &image_object {
                image::DynamicImage::ImageLumaA8(_)
                | image::DynamicImage::ImageBgra8(_)
                | image::DynamicImage::ImageRgba8(_) => image_object.to_rgba8().into_raw(),
                _ => image_object.to_rgba8().into_raw(),
            };

            if !initialized {
                image_width = image_object.width();
                image_height = image_object.height();

                image_array_data.reserve_exact((4 * image_width * image_height) as usize * faces.len());
                initialized = true;
            }

            image_array_data.extend(image_data);
        }

        CubeTexture::from_pixels(device, command_pool, submit_queue, device_memory_properties, vk::Format::R8G8B8A8_SRGB,
                                 &image_array_data, image_width, image_height, faces.len() as u32, true)
    }

    pub fn from_pixels(device: ash::Device,
                       command_pool: vk::CommandPool,
                       submit_queue: vk::Queue,
                       device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
                       format: vk::Format,
                       pixel_data: &Vec<u8>, width: u32, height: u32, array_size: u32, create_mips: bool) -> CubeTexture
    {
        let (texture_image, texture_image_memory, mip_levels) = create_texture_image(
            &device, command_pool, submit_queue, device_memory_properties, format, pixel_data, width, height, array_size, create_mips);

        let texture_image_view = create_image_view(
            &device, texture_image, format,
            vk::ImageAspectFlags::COLOR,
            mip_levels, array_size);
        let texture_sampler = create_texture_sampler(&device, mip_levels);

        CubeTexture {
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

impl Drop for CubeTexture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.texture_sampler, None);
            self.device.destroy_image_view(self.texture_image_view, None);
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_image_memory, None);
        }
    }
}
