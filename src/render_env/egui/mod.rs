use std::{mem, ptr};
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::descriptors::{DescriptorSet, DescriptorSetBuilder};
use crate::render_env::egui::cpu_buffer::CpuBuffer;
use crate::render_env::env::RenderEnv;
use crate::render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use crate::render_env::shader::Shader;
use crate::utils::texture::Texture;

mod cpu_buffer;


pub struct RenderOp {
    vb: CpuBuffer,
    ib: CpuBuffer,
    pub cmd_buf: vk::CommandBuffer,
    env: Arc<RenderEnv>,
}

impl Drop for RenderOp {
    fn drop(&mut self) {
        unsafe {
            self.env.device().free_command_buffers(self.env.command_pool, &[self.cmd_buf]);
        }
    }
}

pub struct EguiRenderer {
    render_ops: Vec<RenderOp>,

    texture: Texture,
    pipeline: Pipeline,
    render_pass: vk::RenderPass,
    env: Arc<RenderEnv>,
    descriptor_set: DescriptorSet,
}

impl EguiRenderer {
    pub fn new(env: Arc<RenderEnv>, ctx: egui::CtxRef, render_pass: vk::RenderPass) -> EguiRenderer {
        ctx.set_fonts(egui::FontDefinitions::default());
        ctx.set_style(egui::Style::default());

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
                stride: mem::size_of::<egui::epaint::Vertex>() as u32,
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
            .disable_culling()
            .build();

        let descriptor_set = DescriptorSetBuilder::new(env.device(), &pipeline.descriptor_set_layouts[0])
            .add_image(texture.texture_image_view, texture.texture_sampler)
            .build();


        EguiRenderer {
            env,
            texture,
            pipeline,
            render_pass,
            render_ops: vec![],
            descriptor_set,
        }
    }

    pub fn render(&mut self, meshes: Vec<egui::ClippedMesh>, framebuffer: vk::Framebuffer, dimensions: [u32; 2], frames: usize) -> vk::CommandBuffer {
        if self.render_ops.len() > frames {
            self.render_ops.remove(0);
        }

        let mut vertices: Vec<egui::epaint::Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for mesh in meshes.iter() {
            vertices.extend(&mesh.1.vertices);
            indices.extend(&mesh.1.indices);
        }

        let vb = CpuBuffer::from_vec(&self.env, vk::BufferUsageFlags::VERTEX_BUFFER, &vertices);
        let ib = CpuBuffer::from_vec(&self.env, vk::BufferUsageFlags::INDEX_BUFFER, &indices);

        // println!("vertices: {:?}", vertices);
        // println!("vertices: {:?}", indices);
        let cmd_buf = self.env.create_primary_command_buffer();
        let device = self.env.device().clone();


        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
        ];

        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            p_inheritance_info: ptr::null(),
        };

        let begin_render_pass = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass: self.render_pass,
            framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: dimensions[0],
                    height: dimensions[1],
                },
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: dimensions[0] as f32,
            height: dimensions[1] as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: dimensions[0],
                height: dimensions[1],
            },
        }];
        unsafe {
            device.begin_command_buffer(cmd_buf, &begin_info);

            device.cmd_begin_render_pass(cmd_buf, &begin_render_pass, vk::SubpassContents::INLINE);

            device.cmd_set_viewport(cmd_buf, 0, viewports.as_ref());
            device.cmd_set_scissor(cmd_buf, 0, scissors.as_ref());

            let vertex_buffers = [vb.buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(cmd_buf, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(cmd_buf, ib.buffer, 0, vk::IndexType::UINT32);
            device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, self.pipeline.graphics_pipeline);

            let bind_descriptors = [self.descriptor_set.set];
            device.cmd_bind_descriptor_sets(cmd_buf, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline_layout,
                                            0, &bind_descriptors, &[]);

            device.cmd_draw_indexed(cmd_buf, indices.len() as u32, 1, 0, 0, 0);
            device.cmd_end_render_pass(cmd_buf);
            device.end_command_buffer(cmd_buf).unwrap();
        }

        self.render_ops.push(RenderOp {
            env: self.env.clone(),
            ib,
            vb,
            cmd_buf,
        });

        cmd_buf
    }
}
