use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::Matrix4;

use ash_render_env::{descriptor_set, pipeline_builder, shader};
use ash_render_env::descriptor_set::{DescriptorSet, DescriptorSetBuilder};
use ash_render_env::env::RenderEnv;
use ash_render_env::frame_buffer::Framebuffer;
use ash_render_env::pipeline_builder::{Pipeline, PipelineBuilder};

use crate::shadow_map::uniform_buffer::UniformBuffer;

#[repr(C)]
struct Uniforms {
    light_vp: Matrix4<f32>,
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

        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .min_filter(vk::Filter::LINEAR)
            .mag_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
            .anisotropy_enable(false);

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

    pub fn update_shadows(&mut self, light_vp: Matrix4<f32>) {
        self.uniform_buffer.write_data(Uniforms {
            light_vp,
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
        }
    }
}
