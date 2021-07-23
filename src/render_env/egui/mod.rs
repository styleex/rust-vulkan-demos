use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::egui::cpu_buffer::CpuBuffer;
use crate::render_env::env::RenderEnv;
use crate::render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use crate::render_env::shader::Shader;
use crate::utils::texture::Texture;

mod cpu_buffer;

pub struct EguiRenderer {
    texture: Texture,
    pipeline: Pipeline,
}

impl EguiRenderer {
    pub fn new(env: &RenderEnv, ctx: egui::CtxRef, render_pass: vk::RenderPass) -> EguiRenderer {
        let font_tx = ctx.texture();
        let texture = Texture::from_pixels(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &env.mem_properties,
            &font_tx.pixels,
            font_tx.width as u32,
            font_tx.height as u32,
        );

        let vs = Shader::load(env.device(), "shaders/spv/egui/egui.vert.spv");
        let ps = Shader::load(env.device(), "shaders/spv/egui/egui.frag.spv");

        let vertex_bindings = vec![
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: 0,
                input_rate: vk::VertexInputRate::VERTEX,
            }
        ];

        let vert_attrs = vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R8G8B8A8_SINT,
                offset: 16,
            },
        ];

        let pipeline = PipelineBuilder::new(env.device().clone(), render_pass, 0)
            .vertex_input(vertex_bindings, vert_attrs)
            .vertex_shader(vs)
            .fragment_shader(ps)
            .build();

        EguiRenderer {
            texture,
            pipeline,
        }
    }

    pub fn render(&mut self, meshes: Vec<egui::ClippedMesh>) {
        for mesh in meshes.iter() {

        }
    }
}

impl Drop for EguiRenderer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
