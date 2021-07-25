use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;


pub struct SyncObjects {
    device: ash::Device,

    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub inflight_fences: Vec<vk::Fence>,
    pub render_quad_semaphore: vk::Semaphore,
    pub render_gui_semaphore: vk::Semaphore,
}

impl SyncObjects {
    pub fn destroy(&mut self) {
        unsafe {
            for semphore in self.image_available_semaphores.drain(0..) {
                self.device.destroy_semaphore(semphore, None);
            }

            for semphore in self.render_finished_semaphores.drain(0..) {
                self.device.destroy_semaphore(semphore, None);
            }

            for fence in self.inflight_fences.drain(0..) {
                self.device.destroy_fence(fence, None);
            }

            self.device.destroy_semaphore(self.render_quad_semaphore, None);
            self.device.destroy_semaphore(self.render_gui_semaphore, None);
        }
    }
}


pub fn create_sync_objects(device: &ash::Device) -> SyncObjects {
    let mut sync_objects = SyncObjects {
        device: device.clone(),

        image_available_semaphores: vec![],
        render_finished_semaphores: vec![],
        inflight_fences: vec![],
        render_quad_semaphore: vk::Semaphore::null(),
        render_gui_semaphore: vk::Semaphore::null(),
    };

    let semaphore_create_info = vk::SemaphoreCreateInfo {
        s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::SemaphoreCreateFlags::empty(),
    };

    let fence_create_info = vk::FenceCreateInfo {
        s_type: vk::StructureType::FENCE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::FenceCreateFlags::SIGNALED,
    };


    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        unsafe {
            let image_available_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create Semaphore Object!");
            let render_finished_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create Semaphore Object!");
            let inflight_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Failed to create Fence Object!");

            sync_objects
                .image_available_semaphores
                .push(image_available_semaphore);
            sync_objects
                .render_finished_semaphores
                .push(render_finished_semaphore);
            sync_objects.inflight_fences.push(inflight_fence);
        }
    }

    unsafe {
        sync_objects.render_quad_semaphore = device
            .create_semaphore(&semaphore_create_info, None)
            .expect("Failed to create Semaphore Object!");

        sync_objects.render_gui_semaphore = device
            .create_semaphore(&semaphore_create_info, None)
            .expect("Failed to create Semaphore Object!");
    };

    sync_objects
}
