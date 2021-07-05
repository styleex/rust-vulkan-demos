use std::path::Path;
use std::time;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use memoffset::offset_of;
use tobj;

use crate::utils::buffer_utils;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
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
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
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

fn load_model(model_path: &Path) -> (Vec<Vertex>, Vec<u32>) {
    let model_obj = tobj::load_obj(model_path, &tobj::LoadOptions {
        single_index: true,
        ..Default::default()
    })
        .expect("Failed to load model object!");

    let mut vertices = vec![];
    let mut indices = vec![];

    let (models, _) = model_obj;

    for m in models.iter() {
        let mesh = &m.mesh;

        if mesh.texcoords.len() == 0 {
            panic!("Missing texture coordinate for the model.")
        }

        println!("{}", mesh.texcoord_indices.len());

        let total_vertices_count = mesh.positions.len() / 3;
        for i in 0..total_vertices_count {
            let vertex = Vertex {
                pos: [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                    1.0,
                ],
                color: [1.0, 1.0, 1.0, 1.0],
                tex_coord: [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]],
            };
            vertices.push(vertex);
        }

        indices = mesh.indices.clone();
    }

    (vertices, indices)
}

pub struct VertexBuffer {
    device: ash::Device,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: usize,
}

impl VertexBuffer {
    pub fn create(instance: &ash::Instance,
                  physical_device: vk::PhysicalDevice,
                  device: ash::Device,
                  command_pool: vk::CommandPool,
                  submit_queue: vk::Queue,
    ) -> VertexBuffer {
        let t1 = time::Instant::now();
        let (vertices, indices) = load_model(Path::new("assets/chalet.obj"));
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

        VertexBuffer {
            device,

            vertex_buffer,
            vertex_buffer_memory,

            index_buffer,
            index_buffer_memory,

            index_count,
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
