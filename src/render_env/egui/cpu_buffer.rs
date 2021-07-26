use core::mem;
use std::{ptr};
use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::env::RenderEnv;

pub struct CpuBuffer {
    buffer_memory: vk::DeviceMemory,
    pub buffer: vk::Buffer,

    device: ash::Device,
}

impl CpuBuffer {
    pub fn from_vec<T>(env: &RenderEnv, usage: vk::BufferUsageFlags, data: &Vec<T>) -> CpuBuffer {
        let size = (data.len() * mem::size_of::<T>()) as u64;

        let buffer_create_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };

        let buffer = unsafe {
            env.device()
                .create_buffer(&buffer_create_info, None)
                .expect("Failed to create Buffer")
        };

        let mem_requirements = unsafe { env.device().get_buffer_memory_requirements(buffer) };
        let memory_type = env.find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
        );

        let allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: ptr::null(),
            allocation_size: mem_requirements.size,
            memory_type_index: memory_type,
        };

        let buffer_memory = unsafe {
            env.device()
                .allocate_memory(&allocate_info, None)
                .expect("Failed to allocate buffer memory!")
        };

        unsafe {
            env.device().bind_buffer_memory(buffer, buffer_memory, 0).unwrap();
        }

        unsafe {
            let mem = env.device()
                .map_memory(buffer_memory, 0, mem_requirements.size, vk::MemoryMapFlags::empty())
                .unwrap() as *mut T;
            mem.copy_from_nonoverlapping(data.as_ptr(), data.len());

            env.device().flush_mapped_memory_ranges(&[
                vk::MappedMemoryRange {
                    s_type: vk::StructureType::MAPPED_MEMORY_RANGE,
                    p_next: ptr::null(),
                    memory: buffer_memory,
                    offset: 0,
                    size: mem_requirements.size,
                }
            ]).unwrap();

            env.device().unmap_memory(buffer_memory);
        };

        CpuBuffer {
            device: env.device().clone(),

            buffer_memory,
            buffer,
        }
    }
}

impl Drop for CpuBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.buffer_memory, None);
        }
    }
}
