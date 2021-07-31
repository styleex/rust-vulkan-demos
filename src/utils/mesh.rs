use std::path::Path;
use std::time;

use ash::version::{DeviceV1_0};
use ash::vk;
use memoffset::offset_of;
use tobj;

use crate::utils::buffer_utils::create_data_buffer;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
    tex_coord: [f32; 2],
    normal: [f32; 3],
}

impl Vertex {
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
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 3,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, normal) as u32,
            },
        ]
    }
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
                normal: [
                    mesh.normals[i * 3],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                ],
            };
            vertices.push(vertex);
        }

        indices = mesh.indices.clone();
    }

    (vertices, indices)
}

pub struct MeshVertexData {
    device: ash::Device,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: usize,
}

impl MeshVertexData {
    pub fn create(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
    ) -> MeshVertexData
    {
        let t1 = time::Instant::now();
        let (vertices, indices) = load_model(Path::new("assets/chalet2.obj"));
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

        MeshVertexData {
            device,

            vertex_buffer,
            vertex_buffer_memory,

            index_buffer,
            index_buffer_memory,

            index_count,
        }
    }
}

impl Drop for MeshVertexData {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
        }
    }
}
