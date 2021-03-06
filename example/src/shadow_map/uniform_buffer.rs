use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use ash_render_env::env::RenderEnv;
use ash_render_env::utils::buffer_utils::create_buffer_;
use std::marker::PhantomData;
use cgmath::Matrix4;
use std::ptr;

#[repr(C)]
pub struct ShadowMapData {
    pub light_wp: Matrix4<f32>,
}

pub struct UniformBuffer<T> {
    pub buffer: vk::Buffer,
    pub buffer_memory: vk::DeviceMemory,
    size: u64,
    device: ash::Device,

    phantom: PhantomData<T>,
}

impl<T> Drop for UniformBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.buffer_memory, None);
        }
    }
}

impl<T> UniformBuffer<T> {
    pub fn new(env: Arc<RenderEnv>) -> UniformBuffer<T> {
        let buffer_size = std::mem::size_of::<T>();

        let (buffer, buffer_memory, size) = create_buffer_(
            env.device(),
            buffer_size as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &env.mem_properties,
        );

        UniformBuffer {
            buffer,
            buffer_memory,
            size,
            device: env.device().clone(),
            phantom: PhantomData
        }
    }

    pub fn write_data(&self, data: T) {
        let buffer_size = std::mem::size_of::<T>() as u64;

        unsafe {
            let data_ptr =
                self.device
                    .map_memory(
                        self.buffer_memory,
                        0,
                        self.size,
                        vk::MemoryMapFlags::empty(),
                    )
                    .expect("Failed to Map Memory") as *mut T;

            data_ptr.copy_from_nonoverlapping(&data, 1);

            let range = vk::MappedMemoryRange {
                s_type: vk::StructureType::MAPPED_MEMORY_RANGE,
                p_next: ptr::null(),
                memory: self.buffer_memory,
                offset: 0,
                size: vk::WHOLE_SIZE,
            };
            self.device.flush_mapped_memory_ranges(&[range]);
            self.device
                .unmap_memory(self.buffer_memory);
        }
    }
}
