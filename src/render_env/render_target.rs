use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::env::RenderEnv;

struct AttachmentImage {
    memory: vk::DeviceMemory,
    image: vk::Image,
    view: vk::ImageView,
}

impl AttachmentImage {
    fn new(env: &RenderEnv, size: [u32; 2], format: vk::Format, mip_levels: u32,
           samples: vk::SampleCountFlags, usage: vk::ImageUsageFlags) -> AttachmentImage {
        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::empty(),
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D {
                width: size[0],
                height: size[1],
                depth: 1,
            },
            mip_levels,
            array_layers: 1,
            samples,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
        };

        let texture_image = unsafe {
            env.device()
                .create_image(&image_create_info, None)
                .expect("Failed to create Texture Image!")
        };

        let image_memory_requirement =
            unsafe { env.device().get_image_memory_requirements(texture_image) };

        let memory_allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: ptr::null(),
            allocation_size: image_memory_requirement.size,
            memory_type_index: env.find_memory_type(
                image_memory_requirement.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
        };

        let texture_image_memory = unsafe {
            env.device()
                .allocate_memory(&memory_allocate_info, None)
                .expect("Failed to allocate Texture Image memory!")
        };

        unsafe {
            env
                .device()
                .bind_image_memory(texture_image, texture_image_memory, 0)
                .expect("Failed to bind Image Memmory!");
        }

        let imageview_create_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            },
            image,
        };

        let image_view = unsafe {
            device
                .create_image_view(&imageview_create_info, None)
                .expect("Failed to create Image View!")
        }

        AttachmentImage {
            memory: texture_image_memory,
            image: texture_image,
        }
    }
}
