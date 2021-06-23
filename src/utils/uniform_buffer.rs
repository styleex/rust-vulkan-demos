use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use std::ptr;
use crate::utils::vertex;
use cgmath::{Deg, Point3, Vector3, Matrix4, Rad};


#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct UniformBufferObject {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
}


pub fn create_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let ubo_layout_bindings = [vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX,
        p_immutable_samplers: ptr::null(),
    }];

    let ubo_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
        s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::DescriptorSetLayoutCreateFlags::empty(),
        binding_count: ubo_layout_bindings.len() as u32,
        p_bindings: ubo_layout_bindings.as_ptr(),
    };

    unsafe {
        device
            .create_descriptor_set_layout(&ubo_layout_create_info, None)
            .expect("Failed to create Descriptor Set Layout!")
    }
}

pub struct UboBuffers {
    device: ash::Device,
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    swapchain_extent: vk::Extent2D,
}

impl UboBuffers {
    pub fn new(
        instance: &ash::Instance,
        device: ash::Device,
        physical_device: vk::PhysicalDevice,
        swapchain_image_count: usize,
        swapchain_extent: vk::Extent2D,
    ) -> UboBuffers {
        let buffer_size = std::mem::size_of::<UniformBufferObject>();

        let mut uniform_buffers = vec![];
        let mut uniform_buffers_memory = vec![];

        let mem_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        for _ in 0..swapchain_image_count {
            let (uniform_buffer, uniform_buffer_memory) = vertex::create_buffer(
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
            swapchain_extent,
        }
    }

    pub fn update_uniform_buffer(&self, current_image: usize, delta_time: f32) {
        let ubos = [UniformBufferObject {
            model: Matrix4::from_angle_z(Rad::from(Deg(90.0 * delta_time))),
            view: Matrix4::look_at_rh(
                Point3::new(2.0, 2.0, 1.0),
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, -1.0),
            ),
            proj: cgmath::perspective(
                Rad::from(Deg(45.0)),
                self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32,
                0.1,
                10.0,
            ),
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
