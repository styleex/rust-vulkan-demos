use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;
pub fn create_command_buffers(
    device: &ash::Device,

    // memory management
    command_pool: vk::CommandPool,

    // pipeline
    graphics_pipeline: vk::Pipeline,

    // Render pass
    framebuffers: &Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    surface_extent: vk::Extent2D,

    // Mesh
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: usize,

    // Shader uniforms
    pipeline_layout: vk::PipelineLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
) -> Vec<vk::CommandBuffer>
{
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: ptr::null(),
        command_buffer_count: framebuffers.len() as u32,
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
    };

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .expect("Failed to allocate Command Buffers!")
    };

    for (i, &command_buffer) in command_buffers.iter().enumerate() {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            p_inheritance_info: ptr::null(),
            flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
        };

        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin recording Command Buffer at beginning!");
        }

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass,
            framebuffer: framebuffers[i],
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface_extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        unsafe {
            let viewports = [vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: surface_extent.width as f32,
                height: surface_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface_extent,
            }];


            device.cmd_set_viewport(command_buffer, 0, viewports.as_ref());
            device.cmd_set_scissor(command_buffer, 0, scissors.as_ref());

            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline,
            );

            let descriptor_sets_to_bind = [descriptor_sets[i]];
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

            device.cmd_end_render_pass(command_buffer);

            device
                .end_command_buffer(command_buffer)
                .expect("Failed to record Command Buffer at Ending!");
        }
    }

    command_buffers
}

pub fn create_quad_command_buffers(
    device: &ash::Device,

    // memory management
    command_pool: vk::CommandPool,

    // pipeline
    graphics_pipeline: vk::Pipeline,

    // Render pass
    framebuffers: &Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    surface_extent: vk::Extent2D,

    // Shader uniforms
    pipeline_layout: vk::PipelineLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
) -> Vec<vk::CommandBuffer>
{
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: ptr::null(),
        command_buffer_count: framebuffers.len() as u32,
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
    };

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .expect("Failed to allocate Command Buffers!")
    };

    for (i, &command_buffer) in command_buffers.iter().enumerate() {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            p_inheritance_info: ptr::null(),
            flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
        };

        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin recording Command Buffer at beginning!");
        }

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass,
            framebuffer: framebuffers[i],
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface_extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        unsafe {
            let viewports = [vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: surface_extent.width as f32,
                height: surface_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface_extent,
            }];


            device.cmd_set_viewport(command_buffer, 0, viewports.as_ref());
            device.cmd_set_scissor(command_buffer, 0, scissors.as_ref());

            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline,
            );

            let descriptor_sets_to_bind = [descriptor_sets[i]];
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &descriptor_sets_to_bind,
                &[],
            );


            device.cmd_draw(command_buffer, 3, 1, 0, 0);

            device.cmd_end_render_pass(command_buffer);

            device
                .end_command_buffer(command_buffer)
                .expect("Failed to record Command Buffer at Ending!");
        }
    }

    command_buffers
}

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
        flags: vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE,
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



