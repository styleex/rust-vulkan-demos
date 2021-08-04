use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use ash_render_env::env::RenderEnv;

const CASCADE_COUNT: usize = 4;

struct Cascade {
    view: vk::ImageView,
    framebuffer: vk::Framebuffer,

    device: ash::Device,
}

impl Drop for Cascade {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_framebuffer(self.framebuffer, None);
            self.device.destroy_image_view(self.view, None);
        }
    }
}

pub struct ShadowMapFramebuffer {
    width: u32,
    height: u32,

    cascades: Vec<Cascade>,

    pub view: vk::ImageView,
    image: vk::Image,
    memory: vk::DeviceMemory,

    render_pass: vk::RenderPass,
    device: ash::Device,
}

impl ShadowMapFramebuffer {
    pub fn new(env: Arc<RenderEnv>) -> ShadowMapFramebuffer {
        let (cascade_width, cascade_height) = (4096 as u32, 4096 as u32);
        let depth_format = vk::Format::D16_UNORM;
        let render_pass = create_render_pass(env.device(), depth_format);


        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::empty(),
            image_type: vk::ImageType::TYPE_2D,
            format: depth_format,
            extent: vk::Extent3D {
                width: cascade_width,
                height: cascade_height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: CASCADE_COUNT as u32,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
        };

        let shadow_map_image = unsafe {
            env.device()
                .create_image(&image_create_info, None)
                .expect("Failed to create Texture Image!")
        };

        let image_memory_requirement =
            unsafe { env.device().get_image_memory_requirements(shadow_map_image) };

        let memory_allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: ptr::null(),
            allocation_size: image_memory_requirement.size,
            memory_type_index: env.find_memory_type(
                image_memory_requirement.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
        };

        let shadow_map_memory = unsafe {
            env.device()
                .allocate_memory(&memory_allocate_info, None)
                .expect("Failed to allocate Texture Image memory!")
        };

        unsafe {
            env
                .device()
                .bind_image_memory(shadow_map_image, shadow_map_memory, 0)
                .expect("Failed to bind Image Memmory!");
        }

        let imageview_create_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            view_type: vk::ImageViewType::TYPE_2D_ARRAY,
            format: depth_format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: CASCADE_COUNT as u32,
            },
            image: shadow_map_image,
        };

        let view = unsafe {
            env.device()
                .create_image_view(&imageview_create_info, None)
                .expect("Failed to create Image View!")
        };


        // CREATE CASCADES VIEWS AND FRAMEBUFFERS
        let mut cascades = Vec::with_capacity(CASCADE_COUNT);
        for i in 0..CASCADE_COUNT - 1 {
            let imageview_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                view_type: vk::ImageViewType::TYPE_2D_ARRAY,
                format: depth_format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: i as u32,
                    layer_count: 1,
                },
                image: shadow_map_image,
            };

            let cascade_image_view = unsafe {
                env.device()
                    .create_image_view(&imageview_create_info, None)
                    .expect("Failed to create Image View!")
            };

            let cascade_image_view_list = [cascade_image_view];
            let framebuffer_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                render_pass: render_pass.clone(),
                attachment_count: cascade_image_view_list.len() as u32,
                p_attachments: cascade_image_view_list.as_ptr(),
                width: cascade_width,
                height: cascade_height,
                layers: 1,
            };

            let cascade_framebuffer = unsafe {
                env.device().create_framebuffer(&framebuffer_info, None).unwrap()
            };

            cascades.push(Cascade {
                device: env.device().clone(),
                view: cascade_image_view,
                framebuffer: cascade_framebuffer,
            })
        }


        ShadowMapFramebuffer {
            device: env.device().clone(),
            width: cascade_width,
            height: cascade_height,
            render_pass,
            image: shadow_map_image,
            memory: shadow_map_memory,
            cascades,
            view,
        }
    }

    pub fn get_cascade_view(&self, index: usize) -> vk::ImageView {
        self.cascades[index].view.clone()
    }

    pub fn frambuffer(&self, index: usize) -> vk::Framebuffer {
        self.cascades[index].framebuffer.clone()
    }

    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass.clone()
    }
}

impl Drop for ShadowMapFramebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
            self.device.destroy_render_pass(self.render_pass, None);
        }
    }
}

fn create_render_pass(device: &ash::Device, depth_format: vk::Format) -> vk::RenderPass {
    let attachments = [vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: depth_format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
    }];

    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let subpass = vec!(
        vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: ptr::null(),
            color_attachment_count: 0,
            p_color_attachments: ptr::null(),
            p_resolve_attachments: ptr::null(),
            p_depth_stencil_attachment: &depth_attachment_ref,
            preserve_attachment_count: 0,
            p_preserve_attachments: ptr::null(),
        }
    );

    let subpass_deps = vec!(
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
            dst_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::SHADER_READ,
            dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::BY_REGION,
        },
        vk::SubpassDependency {
            src_subpass: 0,
            dst_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
            src_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            dependency_flags: vk::DependencyFlags::BY_REGION,
        }
    );

    let render_pass_create_info = vk::RenderPassCreateInfo {
        s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::RenderPassCreateFlags::empty(),
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
