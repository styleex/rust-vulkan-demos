use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::{Deg, Matrix4, Point3, Rad, SquareMatrix, Vector3};

use ash_render_env::camera::Camera;
use ash_render_env::descriptor_set::DescriptorSet;
use ash_render_env::env::RenderEnv;
use ash_render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use ash_render_env::shader;

use crate::shadow_map::uniform_buffer::{ShadowMapData, UniformBuffer};
use crate::utils::mesh;
use crate::utils::mesh::Mesh;
use crate::utils::uniform_buffer::UboBuffers;

pub struct MeshShadowMapRenderer {
    render_cmds: Vec<vk::CommandBuffer>,

    render_pass: vk::RenderPass,
    pipeline: Pipeline,
    descriptor_sets: Vec<DescriptorSet>,
    uniforms: Vec<UniformBuffer<ShadowMapData>>,

    mesh: Arc<Mesh>,

    current_frame: usize,
    max_inflight_frames: usize,

    sampler: vk::Sampler,
    env: Arc<RenderEnv>,
}

impl MeshShadowMapRenderer {
    pub fn new(env: Arc<RenderEnv>, render_pass: vk::RenderPass, mesh: Arc<Mesh>, max_inflight_frames: usize,
               dimensions: [u32; 2]) -> MeshShadowMapRenderer
    {
        let pipeline = {
            let vert_shader_module = shader::Shader::load(env.device(), "shaders/spv/mesh/shadow_map.vert.spv");
            let frag_shader_module = shader::Shader::load(env.device(), "shaders/spv/mesh/shadow_map.frag.spv");

            PipelineBuilder::new(env.device().clone(), render_pass, 0)
                .vertex_shader(vert_shader_module)
                .fragment_shader(frag_shader_module)
                .vertex_input(mesh::Vertex::binding_descriptions(), mesh::Vertex::attribute_descriptions())
                .with_depth_test()
                .color_attachment_count(0)
                .build()
        };

        let sampler = create_texture_sampler(env.device(), 1);
        let mut cmd_bufs = Vec::with_capacity(max_inflight_frames);
        let mut descriptor_sets = Vec::with_capacity(max_inflight_frames);
        let mut uniforms = Vec::with_capacity(max_inflight_frames);
        for i in 0..max_inflight_frames {
            let uniform_buffer = UniformBuffer::new(env.clone());
            descriptor_sets.push(
                DescriptorSet::builder(env.device(), pipeline.descriptor_set_layouts.get(0).unwrap())
                    .add_buffer(uniform_buffer.buffer.clone())
                    .build()
            );
            cmd_bufs.push(
                Self::build_cmd_buf(&env, render_pass, &pipeline, &descriptor_sets[i], &mesh, dimensions)
            );
            uniforms.push(uniform_buffer);
        }

        MeshShadowMapRenderer {
            env: env.clone(),
            pipeline,
            render_cmds: cmd_bufs,
            render_pass,
            uniforms,
            descriptor_sets,
            mesh,
            current_frame: 0,
            max_inflight_frames,
            sampler,
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

    pub fn draw(&mut self, camera: &Camera) -> vk::CommandBuffer {
        let current_frame = self.current_frame;
        self.current_frame = (self.current_frame + 1) % self.max_inflight_frames;

        let view = camera.view_matrix();
        let proj = cgmath::perspective(
            Rad::from(Deg(45.0)),
            4096 as f32 / 4096 as f32,
            0.01,
            100.0,
        );
        // let view = Matrix4::<f32>::look_at_rh(
        //     Point3::new(-0.09,-0.39, -9.5),
        //     Point3::new(-0.97, 0.17, 0.17),
        //     Vector3::new(0.0, 1.0, 0.0),
        // );
        let proj = cgmath::ortho(
            -1.0, 1.0,
            -1.0, 1.0,
            -5.0, 5.0
        );
        let w1 = Matrix4::<f32>::from_angle_x(Rad::from(Deg(90.0)));
        let world = Matrix4::<f32>::from_translation(Vector3::new(0.0, 0.01, -10.0)) * w1;

        self.uniforms[current_frame].write_data(ShadowMapData {
            light_wp: proj * view * world,
        });

        self.render_cmds[current_frame]
    }
}

impl Drop for MeshShadowMapRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.render_cmds.len() > 0 {
                self.env.device().destroy_sampler(self.sampler, None);
                self.env.device().free_command_buffers(self.env.command_pool(), &self.render_cmds);
            }
        }
    }
}

pub fn create_texture_sampler(device: &ash::Device, mip_levels: u32) -> vk::Sampler {
    let sampler_create_info = vk::SamplerCreateInfo {
        s_type: vk::StructureType::SAMPLER_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::SamplerCreateFlags::empty(),
        mag_filter: vk::Filter::LINEAR,
        min_filter: vk::Filter::LINEAR,
        address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
        address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
        address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
        anisotropy_enable: vk::TRUE,
        max_anisotropy: 16.0,
        compare_enable: vk::FALSE,
        compare_op: vk::CompareOp::NEVER,

        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
        min_lod: 0.0,
        max_lod: mip_levels as f32,
        mip_lod_bias: 0.0,

        border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
        unnormalized_coordinates: vk::FALSE,
    };

    unsafe {
        device
            .create_sampler(&sampler_create_info, None)
            .expect("Failed to create Sampler!")
    }
}

