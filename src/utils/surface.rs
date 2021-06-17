use ash::vk;
use ash::extensions::khr::{XlibSurface};
use winit::platform::unix::WindowExtUnix;

pub struct SurfaceStuff {
    pub surface_loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,
}

pub fn create_surface(entry: &ash::Entry, instance: &ash::Instance, window: &winit::window::Window) -> SurfaceStuff {
    let x11_display = window.xlib_display().unwrap();
    let x11_window = window.xlib_window().unwrap();
    let x11_ci = vk::XlibSurfaceCreateInfoKHR::builder()
        .window(x11_window as vk::Window)
        .dpy(x11_display as *mut vk::Display);

    let xlib_surface_loader = XlibSurface::new(entry, instance);

    let surface = unsafe {
        xlib_surface_loader.create_xlib_surface(&x11_ci, None).unwrap()
    };
    let surface_loader = ash::extensions::khr::Surface::new(entry, instance);

    SurfaceStuff { surface_loader, surface }
}
