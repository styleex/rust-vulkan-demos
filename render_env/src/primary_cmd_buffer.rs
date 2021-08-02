use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::env::RenderEnv;
use std::ptr;

pub struct PrimaryCommandBuffer {
    env: Arc<RenderEnv>,
    dimensions: [u32; 2],
    cmd_bufs: Vec<vk::CommandBuffer>,
    max_frame_in_flight: usize,
    current_frame: usize,
}

impl PrimaryCommandBuffer {
    pub fn new(env: Arc<RenderEnv>, max_frame_in_flight: usize) -> PrimaryCommandBuffer {
        let mut cmd_bufs = Vec::with_capacity(max_frame_in_flight);
        for _ in 0..max_frame_in_flight {
            cmd_bufs.push(env.create_primary_command_buffer());
        }

        PrimaryCommandBuffer {
            env: env.clone(),
            dimensions: [0, 0],
            cmd_bufs,
            max_frame_in_flight,
            current_frame: 0,
        }
    }

    pub fn set_dimensions(&mut self, dims: [u32; 2]) {
        self.dimensions = dims;
    }

    pub fn execute_secondary(&mut self, clear_values: Vec<vk::ClearValue>, framebuffer: vk::Framebuffer, render_pass: vk::RenderPass, second_buffers: &[vk::CommandBuffer]) -> vk::CommandBuffer {
        let command_buffer = self.cmd_bufs.get(self.current_frame).unwrap().clone();
        unsafe {
            self.env.device().reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::default()).unwrap()
        }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            p_inheritance_info: ptr::null(),
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        };

        unsafe {
            self.env.device()
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin recording Command Buffer at beginning!");
        }

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass,
            framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: self.dimensions[0],
                    height: self.dimensions[1],
                },
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        unsafe {
            self.env.device().cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::SECONDARY_COMMAND_BUFFERS,
            );

            self.env.device().cmd_execute_commands(command_buffer, second_buffers);

            self.env.device().cmd_end_render_pass(command_buffer);

            self.env.device()
                .end_command_buffer(command_buffer)
                .expect("Failed to record Command Buffer at Ending!");
        }

        self.current_frame = (self.current_frame + 1) % self.max_frame_in_flight;
        command_buffer
    }
}

impl Drop for PrimaryCommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.env.device().free_command_buffers(self.env.command_pool(), &self.cmd_bufs);
        }
    }
}
