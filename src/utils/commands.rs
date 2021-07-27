use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;


pub fn create_second_command_buffers(
    device: &ash::Device,

    // memory management
    command_pool: vk::CommandPool,

    // pipeline
    graphics_pipeline: vk::Pipeline,

    // Render pass
    render_pass: vk::RenderPass,
    viewport: vk::Extent2D,

    // Mesh
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: usize,

    // Shader uniforms
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: vk::DescriptorSet,
) -> vk::CommandBuffer
{
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: ptr::null(),
        command_buffer_count: 1,
        command_pool,
        level: vk::CommandBufferLevel::SECONDARY,
    };

    let command_buffer = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .expect("Failed to allocate Command Buffers!")
            .pop()
            .unwrap()
    };

    let inheritance_info = vk::CommandBufferInheritanceInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_INHERITANCE_INFO,
        p_next: ptr::null(),
        render_pass,
        subpass: 0,
        framebuffer: vk::Framebuffer::null(),
        occlusion_query_enable: 0,
        query_flags: Default::default(),
        pipeline_statistics: Default::default()
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
        width: viewport.width as f32,
        height: viewport.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    }];

    let scissors = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: viewport,
    }];

    unsafe {
        device.cmd_set_viewport(command_buffer, 0, viewports.as_ref());
        device.cmd_set_scissor(command_buffer, 0, scissors.as_ref());

        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            graphics_pipeline,
        );

        let descriptor_sets_to_bind = [descriptor_set];
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            &descriptor_sets_to_bind,
            &[],
        );

        let vertex_buffers = [vertex_buffer];
        let offsets = [0_u64];
        device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
        device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

        device.cmd_draw_indexed(command_buffer, index_count as u32, 1, 0, 0, 0);

        device
            .end_command_buffer(command_buffer)
            .expect("Failed to record Command Buffer at Ending!");
    }

    command_buffer
}
