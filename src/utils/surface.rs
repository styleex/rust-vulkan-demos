use ash::vk;
use crate::utils::platforms;

pub struct SurfaceStuff {
    pub surface_loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,
}

pub fn create_surface(entry: &ash::Entry, instance: &ash::Instance, window: &winit::window::Window) -> SurfaceStuff {
    let surface_loader = ash::extensions::khr::Surface::new(entry, instance);

    let surface = unsafe { platforms::create_surface(entry, instance, window).unwrap() };

    SurfaceStuff { surface_loader, surface }
}
