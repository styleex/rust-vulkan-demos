use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::utils::texture;

pub struct DescriptorSets {
    device: ash::Device,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
}

impl DescriptorSets {
    pub fn new(device: ash::Device, swapchain_images_size: usize, descriptor_set_layout: vk::DescriptorSetLayout,
               uniforms_buffers: &Vec<vk::Buffer>, texture: &texture::Texture) -> DescriptorSets {
        let descriptor_pool = create_descriptor_pool(&device, swapchain_images_size);
        let descriptor_sets = create_descriptor_sets(
            &device, descriptor_pool, descriptor_set_layout, uniforms_buffers,
            texture.texture_image_view, texture.texture_sampler, swapchain_images_size);

        DescriptorSets {
            device,
            descriptor_pool,
            descriptor_sets,
        }
    }

    pub fn destroy(&self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}


fn create_descriptor_pool(
    device: &ash::Device,
    swapchain_images_size: usize,
) -> vk::DescriptorPool {
    let pool_sizes = [
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: swapchain_images_size as u32,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: swapchain_images_size as u32,
        }
    ];

    let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
        s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::DescriptorPoolCreateFlags::empty(),
        max_sets: swapchain_images_size as u32,
        pool_size_count: pool_sizes.len() as u32,
        p_pool_sizes: pool_sizes.as_ptr(),
    };

    unsafe {
        device
            .create_descriptor_pool(&descriptor_pool_create_info, None)
            .expect("Failed to create Descriptor Pool!")
    }
}

fn create_descriptor_sets(
    device: &ash::Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    uniforms_buffers: &Vec<vk::Buffer>,
    texture_image_view: vk::ImageView,
    texture_sampler: vk::Sampler,
    swapchain_images_size: usize,
) -> Vec<vk::DescriptorSet> {
    let mut layouts: Vec<vk::DescriptorSetLayout> = vec![];
    for _ in 0..swapchain_images_size {
        layouts.push(descriptor_set_layout);
    }

    let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo {
        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
        p_next: ptr::null(),
        descriptor_pool,
        descriptor_set_count: swapchain_images_size as u32,
        p_set_layouts: layouts.as_ptr(),
    };

    let descriptor_sets = unsafe {
        device
            .allocate_descriptor_sets(&descriptor_set_allocate_info)
            .expect("Failed to allocate descriptor sets!")
    };

    for (i, &descritptor_set) in descriptor_sets.iter().enumerate() {
        let descriptor_buffer_infos = [
            vk::DescriptorBufferInfo {
                buffer: uniforms_buffers[i],
                offset: 0,
                range: vk::WHOLE_SIZE,
            }
        ];

        let descriptor_image_infos = [
            vk::DescriptorImageInfo {
                sampler: texture_sampler,
                image_view: texture_image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }
        ];

        let descriptor_write_sets = [
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: ptr::null(),
                dst_set: descritptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_image_info: ptr::null(),
                p_buffer_info: descriptor_buffer_infos.as_ptr(),
                p_texel_buffer_view: ptr::null(),
            },
            vk::WriteDescriptorSet {
                // sampler uniform
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: ptr::null(),
                dst_set: descritptor_set,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: descriptor_image_infos.as_ptr(),
                p_buffer_info: ptr::null(),
                p_texel_buffer_view: ptr::null(),
            },
        ];


        unsafe {
            device.update_descriptor_sets(&descriptor_write_sets, &[]);
        }
    }

    descriptor_sets
}
