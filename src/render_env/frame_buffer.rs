use core::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::render_env::attachment_texture::AttachmentImage;
use crate::render_env::env;
use crate::render_env::utils::format_has_depth;


#[derive(Clone)]
pub struct AttachmentDesciption {
    pub format: vk::Format,
    pub samples_count: vk::SampleCountFlags,
}

pub struct Framebuffer {
    attachment_desc: Vec<AttachmentDesciption>,
    pub render_pass: vk::RenderPass,

    pub framebuffer: Option<vk::Framebuffer>,
    pub attachments: Vec<AttachmentImage>,
    dimensions: [u32; 2],

    env: Arc<env::RenderEnv>,
}

impl Framebuffer {
    pub fn new(env: Arc<env::RenderEnv>, attachment_desc: Vec<AttachmentDesciption>) -> Framebuffer {
        let render_pass = Framebuffer::_create_render_pass(env.device(), &attachment_desc);

        Framebuffer {
            env,
            attachment_desc,
            render_pass,
            framebuffer: None,
            attachments: vec![],
            dimensions: [0, 0],
        }
    }

    fn _create_render_pass(
        device: &ash::Device,
        descriptions: &Vec<AttachmentDesciption>,
    ) -> vk::RenderPass
    {
        let mut attachments: Vec<vk::AttachmentDescription> = vec![];

        let mut color_attachments_refs: Vec<vk::AttachmentReference> = vec![];
        let mut depth_attachment_ref: Vec<vk::AttachmentReference> = vec![];

        for (attachment_idx, attachment_info) in descriptions.iter().enumerate() {
            let final_layout = if format_has_depth(attachment_info.format) {
                vk::ImageLayout::DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL
            } else {
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            };

            attachments.push(vk::AttachmentDescription {
                flags: Default::default(),
                format: attachment_info.format,
                samples: attachment_info.samples_count,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout,
            });


            let attachment_ref = vk::AttachmentReference {
                attachment: attachment_idx as u32,
                layout: if format_has_depth(attachment_info.format) {
                    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
                } else {
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
                },
            };

            if format_has_depth(attachment_info.format) {
                depth_attachment_ref.push(attachment_ref);
            } else {
                color_attachments_refs.push(attachment_ref);
            }
        }

        let subpass = vec!(
            vk::SubpassDescription {
                flags: Default::default(),
                pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
                input_attachment_count: 0,
                p_input_attachments: ptr::null(),
                color_attachment_count: color_attachments_refs.len() as u32,
                p_color_attachments: color_attachments_refs.as_ptr(),
                p_resolve_attachments: ptr::null(),
                p_depth_stencil_attachment: depth_attachment_ref.as_ptr(),
                preserve_attachment_count: 0,
                p_preserve_attachments: ptr::null(),
            }
        );

        let subpass_deps = vec!(
            vk::SubpassDependency {
                src_subpass: vk::SUBPASS_EXTERNAL,
                dst_subpass: 0,
                src_stage_mask: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: vk::AccessFlags::MEMORY_READ,
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dependency_flags: vk::DependencyFlags::BY_REGION,
            },
            vk::SubpassDependency {
                src_subpass: 0,
                dst_subpass: vk::SUBPASS_EXTERNAL,
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                dst_stage_mask: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_access_mask: vk::AccessFlags::MEMORY_READ,
                dependency_flags: vk::DependencyFlags::BY_REGION,
            }
        );

        let render_pass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: ptr::null(),
            flags: Default::default(),
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: subpass.len() as u32,
            p_subpasses: subpass.as_ptr(),
            dependency_count: subpass_deps.len() as u32,
            p_dependencies: subpass_deps.as_ptr(),
        };

        unsafe {
            device.create_render_pass(&render_pass_create_info, None).unwrap()
        }
    }

    pub fn resize_swapchain(&mut self, dimensions: [u32; 2]) {
        self.attachments.clear();
        if self.framebuffer.is_some() {
            unsafe {
                self.env.device().destroy_framebuffer(self.framebuffer.unwrap(), None)
            };
        };

        let mut images = vec!();
        let mut views = vec!();

        for desc in self.attachment_desc.iter() {
            let mut usage = vk::ImageUsageFlags::INPUT_ATTACHMENT | vk::ImageUsageFlags::SAMPLED;

            if format_has_depth(desc.format) {
                usage |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
            } else {
                usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
            }

            let img = AttachmentImage::new(
                &self.env,
                dimensions,
                desc.format,
                1,
                desc.samples_count,
                usage,
            );

            views.push(img.view);
            images.push(img);
        }
        self.attachments = images;

        let framebuffer_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: Default::default(),
            render_pass: self.render_pass,
            attachment_count: views.len() as u32,
            p_attachments: views.as_ptr(),
            width: dimensions[0],
            height: dimensions[1],
            layers: 1,
        };

        let framebuffer = unsafe {
            self.env.device().create_framebuffer(&framebuffer_info, None).unwrap()
        };

        self.framebuffer = Some(framebuffer);
        self.dimensions = dimensions;
    }

    pub fn destroy(&self) {
        unsafe {
            if self.framebuffer.is_some() {
                self.env.device().destroy_framebuffer(self.framebuffer.unwrap(), None);
            };

            self.env.device().destroy_render_pass(self.render_pass, None);
        }
    }

    #[inline]
    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }
}
