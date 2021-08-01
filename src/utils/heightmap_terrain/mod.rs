pub mod terrain_renderer;

use std::path::Path;
use std::sync::Arc;

use ash::vk;
use cgmath::{InnerSpace, Vector3};
use memoffset::offset_of;

use crate::render_env::env::RenderEnv;
use crate::utils::buffer_utils::create_data_buffer;
use ash::version::DeviceV1_0;
use crate::utils::heightmap_terrain::terrain_renderer::TerrainRenderer;

pub struct HeightMap {
    pub w: u32,
    pub h: u32,
    height_fn: Box<dyn Fn(u32, u32) -> f32>,
}

#[allow(dead_code)]
impl HeightMap {
    pub fn from_png(path: &Path) -> HeightMap {
        let image_object = image::open(path).unwrap().to_rgba8();
        let w = image_object.width();
        let h = image_object.height();

        let image_data = image_object.into_raw();
        HeightMap {
            w,
            h,
            height_fn: Box::new(move |x: u32, y: u32| -> f32 {
                4.0 * (image_data[(w * y * 4 + x * 4) as usize] as f32) / 255.0
            }),
        }
    }

    pub fn empty(w: u32, h: u32) -> HeightMap {
        HeightMap {
            w,
            h,
            height_fn: Box::new(|_, _| -> f32 { 0.0 }),
        }
    }

    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        let clamp = |val: i32, min: i32, max: i32| -> i32 {
            if val < min {
                return min;
            }
            if val > max {
                return max;
            }

            return val;
        };

        let xx = clamp(x, 0, (self.w - 1) as i32);
        let yy = clamp(y, 0, (self.h - 1) as i32);
        let fn_ = &self.height_fn;

        -fn_(xx as u32, yy as u32)
    }
}

pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    texcoord: [f32; 2],
}

impl Vertex {
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
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R16G16_SFLOAT,
                offset: offset_of!(Self, texcoord) as u32,
            },
        ]
    }
}

pub struct TerrainData {
    device: ash::Device,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: usize,
}

impl TerrainData {
    pub fn new(env: Arc<RenderEnv>, height_map: HeightMap) -> TerrainData {
        let w = height_map.w;
        let h = height_map.h;

        let mut vertices = Vec::with_capacity((h * w) as usize);
        let mut indices = Vec::with_capacity((h * (w - 1) * 6) as usize);

        let get_pos = |x: i32, y: i32| -> Vector3<f32> {
            let height = height_map.get_height(x, y);
            Vector3::new((x as f32) * 0.1, height, -(y as f32) * 0.1)
        };

        for y in 0..(h as i32) {
            for x in 0..(w as i32) {
                let pos = get_pos(x, y);

                // Bottom left, Bottom right, Upper left
                let l = get_pos(x - 1, y) - pos;
                let t = get_pos(x, y + 1) - pos;
                let r = get_pos(x + 1, y) - pos;
                let b = get_pos(x, y - 1) - pos;

                let lb = l.cross(b).normalize();
                let br = b.cross(r).normalize();
                let rt = r.cross(t).normalize();
                let tl = t.cross(l).normalize();

                let normal = -(lb + br + rt + tl).normalize();

                vertices.push(Vertex {
                    position: pos.into(), //[(x as f32) * 0.1, height, -(y as f32) * 0.1],
                    normal: normal.into(),
                    texcoord: [x as f32, y as f32],
                });
            }
        }

        for y in 1..(h) {
            for x in 0..(w - 1) {
                indices.push((y - 1) * w + x);
                indices.push((y - 1) * w + x + 1);
                indices.push((y) * w + x);

                indices.push((y) * w + x);
                indices.push((y - 1) * w + x + 1);
                indices.push((y) * w + x + 1);
            }
        }

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

        TerrainData {
            device: env.device().clone(),
            vertex_buffer,
            vertex_buffer_memory,

            index_buffer,
            index_buffer_memory,

            index_count,
        }
    }
}


impl Drop for TerrainData {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
        }
    }
}
