use std::path::Path;
use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::{Matrix4, SquareMatrix};

use crate::render_env::descriptor_set::DescriptorSet;
use crate::render_env::env::RenderEnv;
use crate::render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use crate::render_env::shader;
use crate::utils::uniform_buffer::UboBuffers;
use crate::utils::{skybox};
use crate::utils::skybox::SkyboxVertexData;
use crate::utils::cube_texture::CubeTexture;


pub struct SkyboxRenderer {
    cmd_bufs: Vec<vk::CommandBuffer>,

    vertex_buffer: SkyboxVertexData,

    render_pass: vk::RenderPass,
    pipeline: Pipeline,
    texture: CubeTexture,

    descriptor_sets: Vec<DescriptorSet>,
    uniforms: UboBuffers,
    env: Arc<RenderEnv>,

    current_frame: usize,
    max_inflight_frames: usize,
}

impl SkyboxRenderer {
    pub fn new(env: Arc<RenderEnv>, render_pass: vk::RenderPass, color_attachment_count: usize,
               msaa_samples: vk::SampleCountFlags, max_inflight_frames: usize,
               dimensions: [u32; 2]) -> SkyboxRenderer
    {
        let pipeline = {
            let vert_shader_module = shader::Shader::load(env.device(), "shaders/spv/skybox.vert.spv");
            let frag_shader_module = shader::Shader::load(env.device(), "shaders/spv/skybox.frag.spv");

            PipelineBuilder::new(env.device().clone(), render_pass, 0)
                .vertex_shader(vert_shader_module)
                .fragment_shader(frag_shader_module)
                .vertex_input(skybox::SkyboxVertex::get_binding_descriptions(), skybox::SkyboxVertex::get_attribute_descriptions())
                .msaa(msaa_samples)
                .color_attachment_count(color_attachment_count)
                .with_depth_func(vk::CompareOp::LESS_OR_EQUAL)
                .cull_mode(vk::CullModeFlags::BACK)
                .build()
        };

        let texture = CubeTexture::new(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &env.mem_properties,
            Path::new("assets/skybox"),
        );

        let uniforms = UboBuffers::new(
            env.instance(),
            env.device().clone(),
            env.physical_device(),
            max_inflight_frames,
        );

        let vertex_buffer = skybox::SkyboxVertexData::create(env.instance(), env.physical_device(), env.device().clone(), env.command_pool(), env.queue());

        let mut cmd_bufs = vec![];
        let mut descriptor_sets = vec![];
        for i in 0..max_inflight_frames {
            descriptor_sets.push(
                DescriptorSet::builder(env.device(), pipeline.descriptor_set_layouts.get(0).unwrap())
                    .add_buffer(uniforms.uniform_buffers[i])
                    .add_image(texture.texture_image_view, texture.texture_sampler)
                    .build()
            );
            cmd_bufs.push(
                Self::build_cmd_buf(&env, render_pass, &pipeline, &descriptor_sets[i], &vertex_buffer, dimensions)
            );
        }

        SkyboxRenderer {
            env: env.clone(),
            pipeline,
            cmd_bufs,
            texture,
            render_pass,
            uniforms,
            descriptor_sets,
            vertex_buffer,
            current_frame: 0,
            max_inflight_frames,
        }
    }

    fn build_cmd_buf(env: &RenderEnv, render_pass: vk::RenderPass, pipeline: &Pipeline, descriptor_set: &DescriptorSet, vertex_buffer: &SkyboxVertexData, dimensions: [u32; 2]) -> vk::CommandBuffer {
        let command_buffer = env.create_secondary_command_buffer();
        let device = env.device();

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

        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin recording Command Buffer at beginning!");
        }


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
            device.cmd_set_viewport(command_buffer, 0, viewports.as_ref());
            device.cmd_set_scissor(command_buffer, 0, scissors.as_ref());

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.graphics_pipeline,
            );

            let descriptor_sets_to_bind = [descriptor_set.set];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                0,
                &descriptor_sets_to_bind,
                &[],
            );

            let vertex_buffers = [vertex_buffer.vertex_buffer];
            let offsets = [0_u64];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(command_buffer, vertex_buffer.index_buffer, 0, vk::IndexType::UINT32);

            device.cmd_draw_indexed(command_buffer, vertex_buffer.index_count as u32, 1, 0, 0, 0);

            device
                .end_command_buffer(command_buffer)
                .expect("Failed to record Command Buffer at Ending!");
        }

        command_buffer
    }

    pub fn resize_framebuffer(&mut self, dimensions: [u32; 2]) {
        unsafe {
            self.env.device().free_command_buffers(self.env.command_pool(), &self.cmd_bufs);
        }

        let mut cmd_bufs = vec![];

        for i in 0..self.max_inflight_frames {
            cmd_bufs.push(
                Self::build_cmd_buf(&self.env, self.render_pass, &self.pipeline,
                                    &self.descriptor_sets[i], &self.vertex_buffer, dimensions)
            );
        }

        self.cmd_bufs = cmd_bufs;
    }

    pub fn draw(&mut self, view: Matrix4<f32>, proj: Matrix4<f32>) -> vk::CommandBuffer
    {
        self.uniforms.update_uniform_buffer(self.current_frame, Matrix4::identity(), view, proj);

        let current_frame = self.current_frame;
        self.current_frame = (self.current_frame + 1) % self.max_inflight_frames;

        self.cmd_bufs[current_frame]
    }
}

impl Drop for SkyboxRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.cmd_bufs.len() > 0 {
                self.env.device().free_command_buffers(self.env.command_pool(), &self.cmd_bufs);
            }
        }
    }
}
