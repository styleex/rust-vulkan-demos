use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::{Matrix4, SquareMatrix};

use ash_render_env::{descriptor_set, pipeline_builder, shader};
use ash_render_env::descriptor_set::{DescriptorSet, DescriptorSetBuilder};
use ash_render_env::env::RenderEnv;
use ash_render_env::frame_buffer::Framebuffer;
use ash_render_env::pipeline_builder::{Pipeline, PipelineBuilder};

use crate::shadow_map::uniform_buffer::UniformBuffer;
use crate::shadow_map::{CASCADE_COUNT, CascadeInfo};

#[repr(C)]
struct Uniforms {
    cascade_splits: [f32; CASCADE_COUNT],
    view: Matrix4<f32>,
    cascade_vp: [Matrix4<f32>; CASCADE_COUNT],
}


pub struct QuadRenderer {
    sampler: vk::Sampler,
    shadow_sampler: vk::Sampler,
    descriptor_set: descriptor_set::DescriptorSet,
    pipeline: pipeline_builder::Pipeline,
    pub render_pass: vk::RenderPass,
    pub second_buffer: vk::CommandBuffer,
    uniform_buffer: UniformBuffer<Uniforms>,
    env: Arc<RenderEnv>,
}

impl QuadRenderer {
    pub fn new(env: Arc<RenderEnv>, framebuffer: &Framebuffer, shadow_map_view: vk::ImageView, render_pass: vk::RenderPass, input_samples: vk::SampleCountFlags, dimensions: [u32; 2]) -> QuadRenderer {
        let pipeline = {
            let vert_shader_module = shader::Shader::load(env.device(), "assets/shaders/spv/compose.vert.spv");
            let frag_shader_module = shader::Shader::load(env.device(), "assets/shaders/spv/compose.frag.spv")
                .specialize(shader::ConstantsBuilder::new().add_u32(input_samples.as_raw()));


            PipelineBuilder::new(env.device().clone(), render_pass, 0)
                .fragment_shader(frag_shader_module)
                .vertex_shader(vert_shader_module)
                .build()
        };

        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .min_filter(vk::Filter::LINEAR)
            .mag_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(false);

        let sampler = unsafe {
            env.device().create_sampler(&sampler_create_info, None).unwrap()
        };

        let sampler_create_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            mip_lod_bias: 0.0,
            anisotropy_enable: 0,
            max_anisotropy: 1.0,
            compare_enable: 0,
            compare_op: Default::default(),
            min_lod: 0.0,
            max_lod: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            unnormalized_coordinates: 0
        };

        let shadow_sampler = unsafe {
            env.device().create_sampler(&sampler_create_info, None).unwrap()
        };


        let uniform_buffer = UniformBuffer::new(env.clone());

        let descriptor_set = DescriptorSetBuilder::new(
            env.device(), pipeline.descriptor_set_layouts.get(0).unwrap())
            .add_image(framebuffer.attachments.get(0).unwrap().view, sampler)
            .add_image(framebuffer.attachments.get(1).unwrap().view, sampler)
            .add_image(framebuffer.attachments.get(2).unwrap().view, sampler)
            .add_image_with_layout(shadow_map_view, shadow_sampler.clone(), vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
            .add_buffer(uniform_buffer.buffer)
            .build();

        let second_buffer = Self::render_quad(&env, dimensions, &pipeline, &descriptor_set, render_pass);

        QuadRenderer {
            pipeline,
            render_pass,
            shadow_sampler,

            sampler,
            descriptor_set,
            second_buffer,

            uniform_buffer,
            env: env.clone(),
        }
    }

    pub fn write_shadowmap_ubo(&mut self, view: Matrix4<f32>, cascades: &Vec<CascadeInfo>) {
        let mut cascade_splits = [0.0; CASCADE_COUNT];
        let mut cascade_vp = [Matrix4::<f32>::identity(); CASCADE_COUNT];

        for (idx, cascade) in cascades.iter().enumerate() {
            cascade_splits[idx] = cascade.max_z;
            cascade_vp[idx] = cascade.view_proj_mat;
        }

        self.uniform_buffer.write_data(Uniforms {
            view,
            cascade_vp,
            cascade_splits
        })
    }
    fn render_quad(env: &RenderEnv, dimensions: [u32; 2], pipeline: &Pipeline, descriptor_set: &DescriptorSet, render_pass: vk::RenderPass) -> vk::CommandBuffer {
        let device = env.device();
        let create_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: ptr::null(),
            command_pool: env.command_pool(),
            level: vk::CommandBufferLevel::SECONDARY,
            command_buffer_count: 1,
        };

        let cmd_buf = unsafe {
            device.allocate_command_buffers(&create_info).unwrap().pop().unwrap()
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
            let inheritance_info = vk::CommandBufferInheritanceInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_INHERITANCE_INFO,
                p_next: ptr::null(),
                render_pass,
                subpass: 0,
                framebuffer: vk::Framebuffer::null(),
                occlusion_query_enable: 0,
                query_flags: Default::default(),
                pipeline_statistics: Default::default(),
            };

            let command_buffer_begin_info = vk::CommandBufferBeginInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                p_next: ptr::null(),
                p_inheritance_info: &inheritance_info,
                flags: vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE | vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
            };

            device
                .begin_command_buffer(cmd_buf, &command_buffer_begin_info)
                .expect("Failed to begin recording Command Buffer at beginning!");

            device.cmd_set_viewport(cmd_buf, 0, viewports.as_ref());
            device.cmd_set_scissor(cmd_buf, 0, scissors.as_ref());

            device.cmd_bind_pipeline(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.graphics_pipeline,
            );

            let descriptor_sets_to_bind = [descriptor_set.set];
            device.cmd_bind_descriptor_sets(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                0,
                &descriptor_sets_to_bind,
                &[],
            );

            device.cmd_draw(cmd_buf, 3, 1, 0, 0);

            device.end_command_buffer(cmd_buf).unwrap();
        }

        cmd_buf
    }

    pub fn update_framebuffer(&mut self, framebuffer: &Framebuffer, shadow_map_view: vk::ImageView, dimensions: [u32; 2]) {
        self.descriptor_set = DescriptorSetBuilder::new(
            self.env.device(), self.pipeline.descriptor_set_layouts.get(0).unwrap())
            .add_image(framebuffer.attachments.get(0).unwrap().view, self.sampler)
            .add_image(framebuffer.attachments.get(1).unwrap().view, self.sampler)
            .add_image(framebuffer.attachments.get(2).unwrap().view, self.sampler)
            .add_image_with_layout(shadow_map_view, self.shadow_sampler, vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
            .add_buffer(self.uniform_buffer.buffer)
            .build();

        self.second_buffer = Self::render_quad(&self.env, dimensions, &self.pipeline, &self.descriptor_set, self.render_pass);
    }
}

impl Drop for QuadRenderer {
    fn drop(&mut self) {
        unsafe {
            self.env.device().destroy_sampler(self.sampler, None);
            self.env.device().destroy_sampler(self.shadow_sampler, None);
        }
    }
}
