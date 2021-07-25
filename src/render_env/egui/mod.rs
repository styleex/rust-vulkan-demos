use crate::render_env::egui::render::EguiRenderer;
use egui::{InputState, RawInput};
use std::sync::Arc;
use crate::render_env::env::RenderEnv;
use ash::vk;
use crate::render_env::egui::input::WinitInput;
use winit::event::Event;

mod cpu_buffer;
mod input;
mod render;
pub use input::egui_to_winit_cursor_icon;

pub struct Egui {
    renderer: EguiRenderer,
    winit_input: WinitInput,
}

impl Egui {
    pub fn new(env: Arc<RenderEnv>, ctx: egui::CtxRef, output_format: vk::Format) -> Egui {
        let renderer = EguiRenderer::new(env, ctx, output_format);
        let winit_input = WinitInput::new();

        Egui {
            winit_input,
            renderer,
        }
    }

    pub fn render(&mut self, meshes: Vec<egui::ClippedMesh>, framebuffer: vk::Framebuffer, dimensions: [u32; 2], frames: usize) -> vk::CommandBuffer {
        self.renderer.render(meshes, framebuffer, dimensions, frames)
    }

    pub fn handle_event<T>(&mut self, context: egui::CtxRef, winit_event: &Event<T>) {
        self.winit_input.handle_event(context, winit_event);
    }

    pub fn raw_input(&self) -> RawInput {
        self.winit_input.raw_input.clone()
    }
}
