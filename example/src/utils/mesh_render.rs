use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::{Matrix4, Deg, Rad, Vector3};

use ash_render_env::descriptor_set::DescriptorSet;
use ash_render_env::env::RenderEnv;
use ash_render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use ash_render_env::shader;
use crate::utils::uniform_buffer::UboBuffers;
use crate::utils::mesh;
use crate::utils::mesh::Mesh;

pub struct MeshRenderer {
    render_cmds: Vec<vk::CommandBuffer>,

    render_pass: vk::RenderPass,
    pipeline: Pipeline,
    descriptor_sets: Vec<DescriptorSet>,
    uniforms: UboBuffers,

    mesh: Arc<Mesh>,

    current_frame: usize,
    max_inflight_frames: usize,

    env: Arc<RenderEnv>,
}

impl MeshRenderer {
    pub fn new(env: Arc<RenderEnv>, render_pass: vk::RenderPass, mesh: Arc<Mesh>, color_attachment_count: usize,
               msaa_samples: vk::SampleCountFlags, max_inflight_frames: usize,
               dimensions: [u32; 2]) -> MeshRenderer
    {
        let pipeline = {
            let vert_shader_module = shader::Shader::load(env.device(), "assets/shaders/spv/mesh/mesh.vert.spv");
            let frag_shader_module = shader::Shader::load(env.device(), "assets/shaders/spv/mesh/mesh.frag.spv");

            PipelineBuilder::new(env.device().clone(), render_pass, 0)
                .vertex_shader(vert_shader_module)
                .fragment_shader(frag_shader_module)
                .vertex_input(mesh::Vertex::binding_descriptions(), mesh::Vertex::attribute_descriptions())
                .msaa(msaa_samples)
                .with_depth_test()
                .color_attachment_count(color_attachment_count)
                .build()
        };

        let uniforms = UboBuffers::new(
            env.instance(),
            env.device().clone(),
            env.physical_device(),
            max_inflight_frames,
        );

        let mut cmd_bufs = vec![];
        let mut descriptor_sets = vec![];
        for i in 0..max_inflight_frames {
            descriptor_sets.push(
                DescriptorSet::builder(env.device(), pipeline.descriptor_set_layouts.get(0).unwrap())
                    .add_buffer(uniforms.uniform_buffers[i])
                    .add_image(mesh.texture.texture_image_view, mesh.texture.texture_sampler)
                    .build()
            );
            cmd_bufs.push(
                Self::build_cmd_buf(&env, render_pass, &pipeline, &descriptor_sets[i], &mesh, dimensions)
            );
        }

        MeshRenderer {
            env: env.clone(),
            pipeline: pipeline,
            render_cmds: cmd_bufs,
            render_pass: render_pass,
            uniforms,
            descriptor_sets: descriptor_sets,
            mesh,
            current_frame: 0,
            max_inflight_frames,
        }
    }

    fn build_cmd_buf(env: &RenderEnv, render_pass: vk::RenderPass, pipeline: &Pipeline, descriptor_set: &DescriptorSet, vertex_buffer: &Mesh, dimensions: [u32; 2]) -> vk::CommandBuffer {
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
            self.env.device().free_command_buffers(self.env.command_pool(), &self.render_cmds);
        }

        let mut cmd_bufs = vec![];

        for i in 0..self.max_inflight_frames {
            cmd_bufs.push(
                Self::build_cmd_buf(&self.env, self.render_pass, &self.pipeline,
                                    &self.descriptor_sets[i], &self.mesh, dimensions)
            );
        }

        self.render_cmds = cmd_bufs;
    }

    pub fn draw(&mut self, view: Matrix4<f32>, proj: Matrix4<f32>) -> vk::CommandBuffer {
        let w1 = Matrix4::<f32>::from_angle_x(Rad::from(Deg(90.0)));
        let world = Matrix4::<f32>::from_translation(Vector3::new(0.0, 0.01, -10.0 )) * w1;

        self.uniforms.update_uniform_buffer(self.current_frame, world, view, proj);

        let current_frame = self.current_frame;
        self.current_frame = (self.current_frame + 1) % self.max_inflight_frames;

        self.render_cmds[current_frame]
    }
}

impl Drop for MeshRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.render_cmds.len() > 0 {
                self.env.device().free_command_buffers(self.env.command_pool(), &self.render_cmds);
            }
        }
    }
}
