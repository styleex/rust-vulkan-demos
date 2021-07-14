use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use cgmath::{Deg, Matrix4, Rad};

use crate::utils::buffer_utils;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct UniformBufferObject {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
}


pub struct UboBuffers {
    device: ash::Device,
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
}

impl UboBuffers {
    pub fn new(
        instance: &ash::Instance,
        device: ash::Device,
        physical_device: vk::PhysicalDevice,
        swapchain_image_count: usize,
    ) -> UboBuffers {
        let buffer_size = std::mem::size_of::<UniformBufferObject>();

        let mut uniform_buffers = vec![];
        let mut uniform_buffers_memory = vec![];

        let mem_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        for _ in 0..swapchain_image_count {
            let (uniform_buffer, uniform_buffer_memory) = buffer_utils::create_buffer(
                &device,
                buffer_size as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &mem_properties,
            );
            uniform_buffers.push(uniform_buffer);
            uniform_buffers_memory.push(uniform_buffer_memory);
        }

        UboBuffers {
            device,
            uniform_buffers,
            uniform_buffers_memory,
        }
    }

    pub fn update_uniform_buffer(&self, current_image: usize, view: Matrix4<f32>, proj: Matrix4<f32>) {
        let ubos = [UniformBufferObject {
            model: Matrix4::from_angle_x(Rad::from(Deg(90.0))),
            view,
            proj,
        }];

        let buffer_size = (std::mem::size_of::<UniformBufferObject>() * ubos.len()) as u64;

        unsafe {
            let data_ptr =
                self.device
                    .map_memory(
                        self.uniform_buffers_memory[current_image],
                        0,
                        buffer_size,
                        vk::MemoryMapFlags::empty(),
                    )
                    .expect("Failed to Map Memory") as *mut UniformBufferObject;

            data_ptr.copy_from_nonoverlapping(ubos.as_ptr(), ubos.len());

            self.device
                .unmap_memory(self.uniform_buffers_memory[current_image]);
        }
    }

    pub fn destroy(&self) {
        unsafe {
            for i in 0..self.uniform_buffers.len() {
                self.device.destroy_buffer(self.uniform_buffers[i], None);
                self.device
                    .free_memory(self.uniform_buffers_memory[i], None);
            }
        }
    }
}
