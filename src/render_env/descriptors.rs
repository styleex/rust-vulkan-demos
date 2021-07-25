use core::ptr;

use ash::{RawPtr, vk};
use ash::version::DeviceV1_0;

use crate::render_env::shader;

pub struct DescriptorSet {
    device: ash::Device,
    pub set: vk::DescriptorSet,
    pool: vk::DescriptorPool,
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
        }
    }
}


// From existing shader::DescriptorSetLayout:
//  1. Create pool with descriptors
//  2. Bind resources to descriptors (add_buffer(..), add_image(..) etc.) with simple validation
//  3. allocate descriptors from pool and write it
pub struct DescriptorSetBuilder {
    device: ash::Device,
    pool: vk::DescriptorPool,
    current_binding: usize,
    binding_desc: Vec<vk::DescriptorSetLayoutBinding>,
    layout: vk::DescriptorSetLayout,

    image_writes: Vec<vk::DescriptorImageInfo>,
    buffer_writes: Vec<vk::DescriptorBufferInfo>,
}

impl DescriptorSetBuilder {
    pub fn new(device: &ash::Device, layout: &shader::DescriptorSetLayout) -> DescriptorSetBuilder {
        let mut pool_sizes = Vec::<vk::DescriptorPoolSize>::new();
        for binding in layout.binding_desc.iter() {
            pool_sizes.push(
                vk::DescriptorPoolSize {
                    ty: binding.descriptor_type,
                    descriptor_count: 1,
                }
            )
        }

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DescriptorPoolCreateFlags::empty(),
            max_sets: 1,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
        };

        let pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_create_info, None)
                .expect("Failed to create Descriptor Pool!")
        };

        DescriptorSetBuilder {
            device: device.clone(),
            current_binding: 0,
            binding_desc: layout.binding_desc.clone(),
            image_writes: vec!(),
            buffer_writes: vec!(),
            pool,
            layout: layout.layout,
        }
    }
    pub fn add_buffer(&mut self, buffer: vk::Buffer) -> &mut Self {
        let desc = self.binding_desc.get(self.current_binding).unwrap();
        if desc.descriptor_type != vk::DescriptorType::UNIFORM_BUFFER {
            panic!("Invalid value for descriptor {}: expected {:?}, found buffer", desc.binding, desc.descriptor_type);
        }

        self.buffer_writes.push(
            vk::DescriptorBufferInfo {
                buffer,
                offset: 0,
                range: vk::WHOLE_SIZE,
            }
        );

        self.current_binding += 1;
        self
    }

    pub fn add_image(&mut self, image_view: vk::ImageView, sampler: vk::Sampler) -> &mut Self {
        let desc = self.binding_desc.get(self.current_binding).unwrap();

        if ![vk::DescriptorType::SAMPLED_IMAGE, vk::DescriptorType::COMBINED_IMAGE_SAMPLER].contains(&desc.descriptor_type) {
            panic!("Invalid value for descriptor {}: expected {:?}, found image", desc.binding, desc.descriptor_type);
        }

        self.image_writes.push(
            vk::DescriptorImageInfo {
                sampler,
                image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }
        );

        self.current_binding += 1;

        self
    }

    pub fn build(&self) -> DescriptorSet {
        let layouts = [self.layout];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: ptr::null(),
            descriptor_pool: self.pool,
            descriptor_set_count: 1,
            p_set_layouts: layouts.as_ptr(),
        };

        let descriptor_sets = unsafe {
            self.device
                .allocate_descriptor_sets(&descriptor_set_allocate_info)
                .expect("Failed to allocate descriptor sets!")
        };

        let &descriptor_set = descriptor_sets.get(0).unwrap();

        let mut cur_img_idx = 0;
        let mut cur_buf_idx = 0;

        let mut write_sets = Vec::new();
        for binding in self.binding_desc.iter() {
            let mut write_desc = vk::WriteDescriptorSet {
                p_next: ptr::null(),
                dst_set: descriptor_set,
                dst_binding: binding.binding,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: binding.descriptor_type,
                p_image_info: ptr::null(),
                p_buffer_info: ptr::null(),
                p_texel_buffer_view: ptr::null(),
                ..vk::WriteDescriptorSet::default()
            };

            if binding.descriptor_type == vk::DescriptorType::COMBINED_IMAGE_SAMPLER {
                write_desc.p_image_info = self.image_writes.get(cur_img_idx).as_raw_ptr();
                cur_img_idx += 1;
            }

            if binding.descriptor_type == vk::DescriptorType::UNIFORM_BUFFER {
                write_desc.p_buffer_info = self.buffer_writes.get(cur_buf_idx).as_raw_ptr();
                cur_buf_idx += 1;
            }

            write_sets.push(write_desc);
        }

        unsafe {
            self.device.update_descriptor_sets(&write_sets, &[]);
        }

        DescriptorSet {
            device: self.device.clone(),
            pool: self.pool,
            set: descriptor_set,
        }
    }
}
