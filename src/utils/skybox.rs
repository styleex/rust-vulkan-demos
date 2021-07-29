use std::path::Path;
use std::time;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use memoffset::offset_of;
use tobj;

use crate::utils::buffer_utils;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SkyboxVertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl SkyboxVertex {
    pub fn get_binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<Self>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }
        ]
    }

    pub fn get_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, color) as u32,
            },
        ]
    }
}

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

fn load_model() -> (Vec<SkyboxVertex>, Vec<u32>) {
    let h = 1.0;
    let vertices = vec![
        // up
        SkyboxVertex { position: [-1.0, -h, 1.0], color: [0.0, 1.0, 0.0] },
        SkyboxVertex { position: [-1.0, -h, -1.0], color: [0.0, 1.0, 0.0] },
        SkyboxVertex { position: [1.0, -h, -1.0], color: [0.0, 1.0, 0.0] },
        SkyboxVertex { position: [1.0, -h, 1.0], color: [0.0, 1.0, 0.0] },

        // bottom
        SkyboxVertex { position: [-1.0, 1.0, 1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [-1.0, 1.0, -1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, 1.0, -1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, 1.0, 1.0], color: [1.0, 1.0, 1.0] },

        // front
        SkyboxVertex { position: [-1.0, 1.0, 1.0], color: [1.0, 0.0, 0.0] },
        SkyboxVertex { position: [-1.0, -h, 1.0], color: [1.0, 0.0, 0.0] },
        SkyboxVertex { position: [1.0, -h, 1.0], color: [1.0, 0.0, 0.0] },
        SkyboxVertex { position: [1.0, 1.0, 1.0], color: [1.0, 0.0, 0.0] },

        // back
        SkyboxVertex { position: [-1.0, 1.0, -1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [-1.0, -h, -1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, -h, -1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, 1.0, -1.0], color: [1.0, 1.0, 1.0] },

        // left
        SkyboxVertex { position: [-1.0, 1.0, -1.0], color: [0.0, 0.0, 1.0] },
        SkyboxVertex { position: [-1.0, -h, -1.0], color: [0.0, 0.0, 1.0] },
        SkyboxVertex { position: [-1.0, -h, 1.0], color: [0.0, 0.0, 1.0] },
        SkyboxVertex { position: [-1.0, 1.0, 1.0], color: [0.0, 0.0, 1.0] },

        // right
        SkyboxVertex { position: [1.0, 1.0, 1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, -h, 1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, -h, -1.0], color: [1.0, 1.0, 1.0] },
        SkyboxVertex { position: [1.0, 1.0, -1.0], color: [1.0, 1.0, 1.0] },
    ];

    let indices = vec![
        // top
        0, 3, 1, 1, 3, 2,

        // bottom
        7, 4, 6, 6, 4, 5,

        // front
        8, 11, 9, 9, 11, 10,

        // back
        15, 12, 14, 14, 12, 13,

        //left
        16, 19, 17, 17, 19, 18,

        //right
        20, 23, 21, 21, 23, 22,
    ];

    (vertices, indices)
}

pub struct SkyboxVertexData {
    device: ash::Device,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: usize,
}

impl SkyboxVertexData {
    pub fn create(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
    ) -> SkyboxVertexData
    {
        let t1 = time::Instant::now();
        let (vertices, indices) = load_model();
        // let (vertices, indices) = (VERTICES_DATA.to_vec(), INDICES_DATA.to_vec());
        println!("Model loaded: {}", t1.elapsed().as_secs_f32());

        let index_count = indices.len();

        let (vertex_buffer, vertex_buffer_memory) = create_data_buffer(
            instance,
            physical_device,
            device.clone(),
            command_pool,
            submit_queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vertices);

        let (index_buffer, index_buffer_memory) = create_data_buffer(
            instance,
            physical_device,
            device.clone(),
            command_pool,
            submit_queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            indices);

        println!("Model uploaded: {}", t1.elapsed().as_secs_f32());

        SkyboxVertexData {
            device,

            vertex_buffer,
            vertex_buffer_memory,

            index_buffer,
            index_buffer_memory,

            index_count,
        }
    }
}

impl Drop for SkyboxVertexData {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
        }
    }
}
