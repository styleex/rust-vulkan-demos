use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::descriptor_set::{DescriptorSet, DescriptorSetBuilder};
use crate::render_env::egui::cpu_buffer::CpuBuffer;
use crate::render_env::env::RenderEnv;
use crate::render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use crate::render_env::shader::Shader;
use crate::utils::texture::Texture;

struct FontTexture(Texture, u64);

#[allow(dead_code)]
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

    texture: FontTexture,
    pipeline: Pipeline,
    render_pass: vk::RenderPass,
    env: Arc<RenderEnv>,
    descriptor_set: DescriptorSet,
    sampler: vk::Sampler,
}

impl EguiRenderer {
    pub fn new(env: Arc<RenderEnv>, ctx: egui::CtxRef, output_format: vk::Format) -> EguiRenderer {
        ctx.set_fonts(egui::FontDefinitions::default());
        ctx.set_style(egui::Style::default());

        let texture = Self::upload_font_texture(&env, ctx);

        let vs = Shader::load(env.device(), "shaders/spv/egui/egui.vert.spv");
        let ps = Shader::load(env.device(), "shaders/spv/egui/egui.frag.spv");

        let vertex_bindings = vec![
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: 4 * std::mem::size_of::<f32>() as u32 + 4 * std::mem::size_of::<u8>() as u32,
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
                format: vk::Format::R8G8B8A8_UNORM,
                offset: 16,
            },
        ];

        let sampler_create_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 16.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,

            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            min_lod: 0.0,
            max_lod: vk::LOD_CLAMP_NONE,
            mip_lod_bias: 0.0,

            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
        };

        let sampler = unsafe {
            env.device()
                .create_sampler(&sampler_create_info, None)
                .expect("Failed to create Sampler!")
        };

        let render_pass = create_render_pass(env.device(), output_format);

        let pipeline = PipelineBuilder::new(env.device().clone(), render_pass, 0)
            .vertex_input(vertex_bindings, vert_attrs)
            .vertex_shader(vs)
            .fragment_shader(ps)
            .disable_culling()
            .blend()
            .build();

        let descriptor_set = DescriptorSetBuilder::new(env.device(), &pipeline.descriptor_set_layouts[0])
            .add_image(texture.0.texture_image_view, sampler)
            .build();

        EguiRenderer {
            env,
            texture,
            pipeline,
            render_pass,
            render_ops: vec![],
            descriptor_set,
            sampler,
        }
    }

    pub fn render(&mut self, ctx: egui::CtxRef, meshes: Vec<egui::ClippedMesh>, dimensions: [u32; 2], frames: usize) -> vk::CommandBuffer {
        if self.render_ops.len() > frames {
            self.render_ops.remove(0);
        }

        if ctx.texture().version != self.texture.1 {
            println!("egui: upload new texture version");
            self.texture = Self::upload_font_texture(&self.env, ctx);
            self.descriptor_set = DescriptorSetBuilder::new(self.env.device(), &self.pipeline.descriptor_set_layouts[0])
                .add_image(self.texture.0.texture_image_view, self.sampler)
                .build();
        }

        let mut vertices: Vec<egui::epaint::Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for mesh in meshes.iter() {
            vertices.extend(&mesh.1.vertices);
            indices.extend(&mesh.1.indices);
        }

        let vb = CpuBuffer::from_vec(&self.env, vk::BufferUsageFlags::VERTEX_BUFFER, &vertices);
        let ib = CpuBuffer::from_vec(&self.env, vk::BufferUsageFlags::INDEX_BUFFER, &indices);

        let cmd_buf = self.env.create_secondary_command_buffer();
        let device = self.env.device().clone();

        let inheritance_info = vk::CommandBufferInheritanceInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_INHERITANCE_INFO,
            p_next: ptr::null(),
            render_pass: self.render_pass,
            subpass: 0,
            framebuffer: vk::Framebuffer::null(),
            occlusion_query_enable: 0,
            query_flags: Default::default(),
            pipeline_statistics: Default::default()
        };
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            flags: vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE | vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            p_inheritance_info: &inheritance_info,
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: dimensions[0] as f32,
            height: dimensions[1] as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let mut data = Vec::new();
        data.extend((dimensions[0] as f32).to_le_bytes());
        data.extend((dimensions[1] as f32).to_le_bytes());

        unsafe {
            device.begin_command_buffer(cmd_buf, &begin_info).unwrap();

            device.cmd_set_viewport(cmd_buf, 0, viewports.as_ref());
            let vertex_buffers = [vb.buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(cmd_buf, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(cmd_buf, ib.buffer, 0, vk::IndexType::UINT32);
            device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, self.pipeline.graphics_pipeline);
            device.cmd_push_constants(cmd_buf, self.pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0,
                                      &data);

            let bind_descriptors = [self.descriptor_set.set];
            device.cmd_bind_descriptor_sets(cmd_buf, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline_layout,
                                            0, &bind_descriptors, &[]);

            let mut index_base = 0;
            let mut vertex_base = 0;
            for egui::ClippedMesh(rect, mesh) in meshes.iter() {
                let min = rect.min;
                let scale_factor = 1.0;

                let min = egui::Pos2 {
                    x: min.x * scale_factor as f32,
                    y: min.y * scale_factor as f32,
                };
                let min = egui::Pos2 {
                    x: f32::clamp(min.x, 0.0, dimensions[0] as f32),
                    y: f32::clamp(min.y, 0.0, dimensions[1] as f32),
                };
                let max = rect.max;
                let max = egui::Pos2 {
                    x: max.x * scale_factor as f32,
                    y: max.y * scale_factor as f32,
                };
                let max = egui::Pos2 {
                    x: f32::clamp(max.x, min.x, dimensions[0] as f32),
                    y: f32::clamp(max.y, min.y, dimensions[1] as f32),
                };

                device.cmd_set_scissor(
                    cmd_buf,
                    0,
                    &[vk::Rect2D::builder()
                        .offset(
                            vk::Offset2D::builder()
                                .x(min.x.round() as i32)
                                .y(min.y.round() as i32)
                                .build(),
                        )
                        .extent(
                            vk::Extent2D::builder()
                                .width((max.x.round() - min.x) as u32)
                                .height((max.y.round() - min.y) as u32)
                                .build(),
                        )
                        .build()],
                );

                device.cmd_draw_indexed(
                    cmd_buf,
                    mesh.indices.len() as u32,
                    1,
                    index_base,
                    vertex_base,
                    0,
                );

                index_base += mesh.indices.len() as u32;
                vertex_base += mesh.vertices.len() as i32;
            }

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

    fn upload_font_texture(env: &RenderEnv, ctx: egui::CtxRef) -> FontTexture {
        let font_tx = ctx.texture();
        let data = font_tx
            .pixels
            .iter()
            .flat_map(|&r| vec![r, r, r, r])
            .collect::<Vec<_>>();

        let texture = Texture::from_pixels(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &env.mem_properties,
            vk::Format::R8G8B8A8_UNORM,
            &data,
            font_tx.width as u32,
            font_tx.height as u32,
            false,
        );

        FontTexture(texture, font_tx.version)
    }
}

impl Drop for EguiRenderer {
    fn drop(&mut self) {
        unsafe {
            self.env.device().destroy_render_pass(self.render_pass, None);
            self.env.device().destroy_sampler(self.sampler, None);
        }
    }
}


fn create_render_pass(device: &ash::Device, surface_format: vk::Format) -> vk::RenderPass {
    let color_attachment = vk::AttachmentDescription {
        format: surface_format,
        flags: vk::AttachmentDescriptionFlags::empty(),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::LOAD,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
    };

    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let subpasses = [vk::SubpassDescription {
        color_attachment_count: 1,
        p_color_attachments: &color_attachment_ref,
        p_depth_stencil_attachment: ptr::null(),
        flags: vk::SubpassDescriptionFlags::empty(),
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        input_attachment_count: 0,
        p_input_attachments: ptr::null(),
        p_resolve_attachments: ptr::null(),
        preserve_attachment_count: 0,
        p_preserve_attachments: ptr::null(),
    }];

    let render_pass_attachments = [color_attachment];

    let subpass_dependencies = [
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::MEMORY_READ,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::BY_REGION,
        },
        vk::SubpassDependency {
            src_subpass: 0,
            dst_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags::MEMORY_READ,
            dependency_flags: vk::DependencyFlags::BY_REGION,
        }
    ];

    let renderpass_create_info = vk::RenderPassCreateInfo {
        s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
        flags: vk::RenderPassCreateFlags::empty(),
        p_next: ptr::null(),
        attachment_count: render_pass_attachments.len() as u32,
        p_attachments: render_pass_attachments.as_ptr(),
        subpass_count: subpasses.len() as u32,
        p_subpasses: subpasses.as_ptr(),
        dependency_count: subpass_dependencies.len() as u32,
        p_dependencies: subpass_dependencies.as_ptr(),
    };

    unsafe {
        device
            .create_render_pass(&renderpass_create_info, None)
            .expect("Failed to create render pass!")
    }
}
