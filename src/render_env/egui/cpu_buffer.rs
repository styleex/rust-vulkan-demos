use core::mem;
use std::{ptr, ffi};

use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::env::RenderEnv;
use crate::utils::buffer_utils::find_memory_type;
use std::ffi::c_void;

pub struct CpuBuffer {
    buffer_memory: vk::DeviceMemory,
    buffer: vk::Buffer,

    device: ash::Device,
    size: u64,
}

impl CpuBuffer {
    pub fn new(env: &RenderEnv, size: u64, usage: vk::BufferUsageFlags) -> CpuBuffer {
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

        CpuBuffer {
            device: env.device().clone(),

            buffer_memory,
            buffer,
            size,
        }
    }

    pub fn upload_data<T: Sized>(&mut self, data: T) {
        unsafe {
            let mem = self.device.map_memory(self.buffer_memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap();
            mem.copy_from_nonoverlapping(&data as *const _ as *const ffi::c_void, mem::size_of_val(&data));
            self.device.unmap_memory(self.buffer_memory);

            self.device.flush_mapped_memory_ranges(&[
                vk::MappedMemoryRange{
                    s_type: vk::StructureType::MAPPED_MEMORY_RANGE,
                    p_next: ptr::null(),
                    memory: self.buffer_memory,
                    offset: 0,
                    size: vk::WHOLE_SIZE,
                }
            ]);
        };
    }
}
