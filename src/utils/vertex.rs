use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use memoffset::offset_of;

use crate::utils::buffer_utils;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
    tex_coord: [f32; 2],
}

impl Vertex {
    pub fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, color) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, tex_coord) as u32,
            },
        ]
    }
}

const VERTICES_DATA: [Vertex; 4] = [
    Vertex {
        pos: [-0.75, -0.75],
        color: [1.0, 0.0, 0.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        pos: [0.75, -0.75],
        color: [0.0, 1.0, 0.0],
        tex_coord: [0.0, 0.0],
    },
    Vertex {
        pos: [0.75, 0.75],
        color: [0.0, 0.0, 1.0],
        tex_coord: [0.0, 1.0],
    },
    Vertex {
        pos: [-0.75, 0.75],
        color: [1.0, 1.0, 1.0],
        tex_coord: [1.0, 1.0],
    },
];

const INDICES_DATA: [u32; 6] = [0, 1, 2, 2, 3, 0];

fn copy_buffer(
    device: &ash::Device,
    submit_queue: vk::Queue,
    command_pool: vk::CommandPool,
    src_buffer: vk::Buffer,
    dst_buffer: vk::Buffer,
    size: vk::DeviceSize,
) {
    let command_buffer = buffer_utils::begin_single_time_command(device, command_pool);

    unsafe {
        let copy_regions = [vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        }];

        device.cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &copy_regions);
    }

    buffer_utils::end_single_time_command(device, command_pool, submit_queue, command_buffer);
}

fn create_data_buffer<T: Sized>(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    command_pool: vk::CommandPool,
    submit_queue: vk::Queue,
    usage: vk::BufferUsageFlags,
    data: Vec<T>) -> (vk::Buffer, vk::DeviceMemory)
{
    let mem_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let data_size = (std::mem::size_of::<T>() * data.len()) as u64;
    let (staging_buffer, staging_buffer_memory) = buffer_utils::create_buffer(
        &device,
        data_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        &mem_properties,
    );

    unsafe {
        let data_ptr = device
            .map_memory(
                staging_buffer_memory,
                0,
                data_size,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to Map Memory") as *mut T;

        data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());

        device.unmap_memory(staging_buffer_memory);
    }

    let (vertex_buffer, vertex_buffer_memory) = buffer_utils::create_buffer(
        &device,
        data_size,
        vk::BufferUsageFlags::TRANSFER_DST | usage,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        &mem_properties);

    copy_buffer(
        &device,
        submit_queue,
        command_pool,
        staging_buffer,
        vertex_buffer,
        data_size,
    );

    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }

    (vertex_buffer, vertex_buffer_memory)
}

pub struct VertexBuffer {
    device: ash::Device,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
}

impl VertexBuffer {
    pub fn create(instance: &ash::Instance,
                  physical_device: vk::PhysicalDevice,
                  device: ash::Device,
                  command_pool: vk::CommandPool,
                  submit_queue: vk::Queue,
    ) -> VertexBuffer {
        let (vertex_buffer, vertex_buffer_memory) = create_data_buffer(
            instance,
            physical_device,
            device.clone(),
            command_pool,
            submit_queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            VERTICES_DATA.to_vec());

        let (index_buffer, index_buffer_memory) = create_data_buffer(
            instance,
            physical_device,
            device.clone(),
            command_pool,
            submit_queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            INDICES_DATA.to_vec());

        VertexBuffer {
            device,

            vertex_buffer,
            vertex_buffer_memory,

            index_buffer,
            index_buffer_memory,
        }
    }

    pub fn destroy(&self) {
        unsafe {
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
        }
    }
}
