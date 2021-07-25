use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::shader::{DescriptorSetLayout, Shader};
use crate::render_env::shader;

pub struct Pipeline {
    pub device: ash::Device,
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
    pub pipeline_layout: vk::PipelineLayout,
    pub graphics_pipeline: vk::Pipeline,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.graphics_pipeline, None);

            for descriptor_set_layout in self.descriptor_set_layouts.iter() {
                self.device.destroy_descriptor_set_layout(descriptor_set_layout.layout, None);
            }

            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}


// Work in progress struct
pub struct PipelineBuilder {
    device: ash::Device,

    render_pass: vk::RenderPass,
    subpass: u32,

    vertex_input: vk::PipelineVertexInputStateCreateInfo,
    vertex_input_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_input_attributes: Vec<vk::VertexInputAttributeDescription>,

    input_assembly: vk::PipelineInputAssemblyStateCreateInfo,
    tesselation: Option<vk::PipelineTessellationStateCreateInfo>,
    viewport: vk::PipelineViewportStateCreateInfo,
    rasterization: vk::PipelineRasterizationStateCreateInfo,
    multisampling: vk::PipelineMultisampleStateCreateInfo,
    depth_stencil: vk::PipelineDepthStencilStateCreateInfo,
    color_blend: vk::PipelineColorBlendStateCreateInfo,
    color_blend_attachment_states: Vec<vk::PipelineColorBlendAttachmentState>,

    vertex_shader: Option<Shader>,
    fragment_shader: Option<Shader>,
}

impl PipelineBuilder {
    pub fn new(device: ash::Device, render_pass: vk::RenderPass, subpass: u32) -> PipelineBuilder {
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_attribute_description_count: 0,
            p_vertex_attribute_descriptions: ptr::null(),
            vertex_binding_description_count: 0,
            p_vertex_binding_descriptions: ptr::null(),
        };

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            p_next: ptr::null(),
            primitive_restart_enable: vk::FALSE,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        };

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            scissor_count: 1,
            p_scissors: ptr::null(),
            viewport_count: 1,
            p_viewports: ptr::null(),
        };

        let rasterization_status_create_info = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: vk::FALSE,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            rasterizer_discard_enable: vk::FALSE,
            depth_bias_clamp: 0.0,
            depth_bias_constant_factor: 0.0,
            depth_bias_enable: vk::FALSE,
            depth_bias_slope_factor: 0.0,
        };

        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            p_next: ptr::null(),
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: vk::FALSE,
            min_sample_shading: 0.0,
            p_sample_mask: ptr::null(),
            alpha_to_one_enable: vk::FALSE,
            alpha_to_coverage_enable: vk::FALSE,
        };

        let stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            compare_mask: 0,
            write_mask: 0,
            reference: 0,
        };

        let depth_state_create_info = vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
            depth_test_enable: vk::FALSE,
            depth_write_enable: vk::FALSE,
            depth_compare_op: vk::CompareOp::LESS,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            front: stencil_state,
            back: stencil_state,
            max_depth_bounds: 1.0,
            min_depth_bounds: 0.0,
        };

        let color_blend_attachment_states = vec![
            vk::PipelineColorBlendAttachmentState {
                blend_enable: vk::FALSE,
                color_write_mask: vk::ColorComponentFlags::all(),
                src_color_blend_factor: vk::BlendFactor::ONE,
                dst_color_blend_factor: vk::BlendFactor::ZERO,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ONE,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
            }
        ];

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: 0,
            p_attachments: ptr::null(),
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        };

        PipelineBuilder {
            device,

            render_pass,
            subpass,

            vertex_input: vertex_input_state_create_info,
            vertex_input_bindings: vec!(),
            vertex_input_attributes: vec!(),

            input_assembly: vertex_input_assembly_state_info,
            tesselation: None,
            viewport: viewport_state_create_info,
            rasterization: rasterization_status_create_info,
            multisampling: multisample_state_create_info,
            depth_stencil: depth_state_create_info,
            color_blend: color_blend_state,
            color_blend_attachment_states,

            vertex_shader: None,
            fragment_shader: None,
        }
    }

    pub fn vertex_input(mut self, bindings: Vec<vk::VertexInputBindingDescription>, attrs: Vec<vk::VertexInputAttributeDescription>) -> Self {
        self.vertex_input_bindings = bindings;
        self.vertex_input_attributes = attrs;

        self
    }

    pub fn vertex_shader(mut self, shader: Shader) -> Self {
        self.vertex_shader = Some(shader);

        self
    }

    pub fn fragment_shader(mut self, shader: Shader) -> Self {
        self.fragment_shader = Some(shader);

        self
    }

    pub fn msaa(mut self, sample_count: vk::SampleCountFlags) -> Self {
        self.multisampling.rasterization_samples = sample_count;

        self
    }

    pub fn with_depth_test(mut self) -> Self {
        let stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            compare_mask: 0,
            write_mask: 0,
            reference: 0,
        };

        self.depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
            depth_test_enable: vk::TRUE,
            depth_write_enable: vk::TRUE,
            depth_compare_op: vk::CompareOp::LESS,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            front: stencil_state,
            back: stencil_state,
            max_depth_bounds: 1.0,
            min_depth_bounds: 0.0,
        };

        self
    }

    pub fn disable_culling(mut self) -> Self {
        self.rasterization.cull_mode = vk::CullModeFlags::NONE;

        self
    }

    pub fn build(&mut self) -> Pipeline {
        let shader_stages = [
            self.vertex_shader.as_ref().unwrap().stage(),
            self.fragment_shader.as_ref().unwrap().stage(),
        ];

        let descriptor_set_layouts = shader::create_descriptor_set_layout(
            &self.device,
            vec![
                self.vertex_shader.as_ref().unwrap(),
                self.fragment_shader.as_ref().unwrap(),
            ]);

        let layout_vec: Vec<_> = descriptor_set_layouts
            .iter()
            .map(|x| x.layout)
            .collect();

        let mut push_constant_ranges = Vec::new();
        if self.vertex_shader.as_ref().unwrap().push_constants_range.size > 0 {
            push_constant_ranges.push(self.vertex_shader.as_ref().unwrap().push_constants_range);
        };

        if self.fragment_shader.as_ref().unwrap().push_constants_range.size > 0 {
            push_constant_ranges.push(self.fragment_shader.as_ref().unwrap().push_constants_range);
        };

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(),
            set_layout_count: layout_vec.len() as u32,
            p_set_layouts: layout_vec.as_ptr(),
            push_constant_range_count: push_constant_ranges.len() as u32,
            p_push_constant_ranges: push_constant_ranges.as_ptr(),
        };

        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .expect("Failed to create pipeline layout!")
        };

        // leaving the dynamic statue unconfigurated right now
        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_state_count: dynamic_state.len() as u32,
            p_dynamic_states: dynamic_state.as_ptr(),
        };

        self.color_blend.attachment_count = self.color_blend_attachment_states.len() as u32;
        self.color_blend.p_attachments = self.color_blend_attachment_states.as_ptr();

        if !self.vertex_input_bindings.is_empty() {
            self.vertex_input.vertex_binding_description_count = self.vertex_input_bindings.len() as u32;
            self.vertex_input.p_vertex_binding_descriptions = self.vertex_input_bindings.as_ptr();
        }

        if !self.vertex_input_attributes.is_empty() {
            self.vertex_input.vertex_attribute_description_count = self.vertex_input_attributes.len() as u32;
            self.vertex_input.p_vertex_attribute_descriptions = self.vertex_input_attributes.as_ptr();
        }

        let graphic_pipeline_create_infos = [
            vk::GraphicsPipelineCreateInfo {
                s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineCreateFlags::empty(),

                stage_count: shader_stages.len() as u32,
                p_stages: shader_stages.as_ptr(),

                p_vertex_input_state: &self.vertex_input,
                p_input_assembly_state: &self.input_assembly,
                p_tessellation_state: match self.tesselation.as_ref() {
                    Some(val) => val,
                    None => ptr::null(),
                },
                p_viewport_state: &self.viewport,
                p_rasterization_state: &self.rasterization,
                p_multisample_state: &self.multisampling,
                p_depth_stencil_state: &self.depth_stencil,
                p_color_blend_state: &self.color_blend,
                p_dynamic_state: &dynamic_state_info,
                layout: pipeline_layout,

                render_pass: self.render_pass,
                subpass: self.subpass,

                base_pipeline_handle: vk::Pipeline::null(),
                base_pipeline_index: -1,
            }
        ];

        let graphics_pipelines = unsafe {
            self.device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &graphic_pipeline_create_infos,
                    None,
                )
                .expect("Failed to create Graphics Pipeline!.")
        };

        Pipeline {
            device: self.device.clone(),
            graphics_pipeline: graphics_pipelines[0],
            pipeline_layout,
            descriptor_set_layouts,
        }
    }
}
