use ash::version::DeviceV1_0;
use ash::vk;
use memoffset::offset_of;

use crate::utils::buffer_utils::create_data_buffer;
use crate::utils::cube_texture::CubeTexture;
use std::path::Path;
use std::sync::Arc;
use ash_render_env::env::RenderEnv;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SkyboxVertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl SkyboxVertex {
    pub fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<Self>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }
        ]
    }

    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
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

    pub(super) texture: CubeTexture,
}

impl SkyboxVertexData {
    pub fn create(env: Arc<RenderEnv>) -> SkyboxVertexData
    {
        let (vertices, indices) = load_model();

        let index_count = indices.len();

        let (vertex_buffer, vertex_buffer_memory) = create_data_buffer(
            env.instance(),
            env.physical_device(),
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vertices);

        let (index_buffer, index_buffer_memory) = create_data_buffer(
            env.instance(),
            env.physical_device(),
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            vk::BufferUsageFlags::INDEX_BUFFER,
            indices);

        let texture = CubeTexture::new(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &env.mem_properties,
            Path::new("./assets/skybox"),
        );

        SkyboxVertexData {
            device: env.device().clone(),

            vertex_buffer,
            vertex_buffer_memory,

            index_buffer,
            index_buffer_memory,

            index_count,
            texture,
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
