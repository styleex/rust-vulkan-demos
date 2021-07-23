use ash::vk;

use crate::render_env::pipeline_builder::{Pipeline, PipelineBuilder};
use crate::render_env::shader;
use crate::utils::vertex;

pub fn create_graphics_pipeline(
    device: ash::Device,
    render_pass: vk::RenderPass,
    samples: vk::SampleCountFlags,
) -> Pipeline
{
    let vert_shader_module = shader::Shader::load(&device, "shaders/spv/09-shader-base.vert.spv");
    let frag_shader_module = shader::Shader::load(&device, "shaders/spv/09-shader-base.frag.spv");

    PipelineBuilder::new(device, render_pass, 0)
        .vertex_shader(vert_shader_module)
        .fragment_shader(frag_shader_module)
        .vertex_input(vertex::Vertex::get_binding_descriptions(), vertex::Vertex::get_attribute_descriptions())
        .msaa(samples)
        .with_depth_test()
        .build()
}


pub fn create_quad_graphics_pipeline(
    device: ash::Device,
    render_pass: vk::RenderPass,
    input_attachment_samples: vk::SampleCountFlags,
) -> Pipeline
{
    let vert_shader_module = shader::Shader::load(&device, "shaders/spv/compose.vert.spv");
    let frag_shader_module = shader::Shader::load(&device, "shaders/spv/compose.frag.spv")
        .specialize(shader::ConstantsBuilder::new().add_u32(input_attachment_samples.as_raw()));


    PipelineBuilder::new(device.clone(), render_pass, 0)
        .fragment_shader(frag_shader_module)
        .vertex_shader(vert_shader_module)
        .build()
}
