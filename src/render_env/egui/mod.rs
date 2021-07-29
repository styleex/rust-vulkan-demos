use std::sync::Arc;
use std::time::Instant;

use ash::vk;
use egui::math::vec2;
use winit::event::{WindowEvent};

pub use winit_input::egui_to_winit_cursor_icon;

use crate::render_env::egui::renderer::EguiRenderer;
use crate::render_env::egui::winit_input::WinitInput;
use crate::render_env::env::RenderEnv;

mod cpu_buffer;
mod winit_input;
mod renderer;

pub struct Egui {
    ctx: egui::CtxRef,
    renderer: EguiRenderer,
    winit_input: WinitInput,
    current_cursor_icon: egui::CursorIcon,

    start_time: Option<Instant>,
    dimensions: [u32; 2],
    max_frames_in_flight: usize,
}

impl Egui {
    pub fn new(env: Arc<RenderEnv>, output_format: vk::Format, scale_factor: f64, dimensions: [u32; 2], max_frames_in_flight: usize) -> Egui {
        let mut ctx = egui::CtxRef::default();

        let raw_input = egui::RawInput {
            pixels_per_point: Some(scale_factor as f32),
            screen_rect: Some(egui::Rect::from_min_size(
                Default::default(),
                vec2(dimensions[0] as f32, dimensions[1] as f32) / scale_factor as f32,
            )),
            time: Some(0.0),
            ..Default::default()
        };

        // Egui create internal font texture only after first `begin_frame` call
        ctx.begin_frame(raw_input.clone());
        let (_output, _shapes) = ctx.end_frame();

        let renderer = EguiRenderer::new(env, ctx.clone(), output_format);
        let winit_input = WinitInput::new(raw_input, scale_factor);

        Egui {
            ctx,
            winit_input,
            renderer,
            current_cursor_icon: egui::CursorIcon::None,
            start_time: None,
            dimensions,
            max_frames_in_flight,
        }
    }

    pub fn handle_event(&mut self, window_event: &WindowEvent) {
        self.winit_input.handle_event(self.ctx.clone(), window_event);
    }

    pub fn begin_frame(&mut self) {
        let mut raw_input = self.winit_input.raw_input.take();

        if let Some(time) = self.start_time {
            raw_input.time = Some(time.elapsed().as_secs_f64());
        } else {
            self.start_time = Some(Instant::now());
        }

        self.ctx.begin_frame(raw_input);
    }

    pub fn end_frame(&mut self, wnd: &winit::window::Window) -> vk::CommandBuffer {
        let (output, shapes) = self.ctx.end_frame();
        if self.current_cursor_icon != output.cursor_icon {
            if let Some(cursor_icon) = egui_to_winit_cursor_icon(output.cursor_icon) {
                wnd.set_cursor_visible(true);
                wnd.set_cursor_icon(cursor_icon);
            } else {
                wnd.set_cursor_visible(false);
            }
            self.current_cursor_icon = output.cursor_icon;
        };

        let clipped_meshes = self.ctx.tessellate(shapes);

        let gui_render_op = self.renderer.render(
            self.ctx.clone(),
            clipped_meshes,
            self.dimensions,
            self.max_frames_in_flight,
            self.winit_input.scale_factor as f32,
        );

        gui_render_op
    }

    pub fn set_dimensions(&mut self, dimensions: [u32; 2]) {
        self.dimensions = dimensions;
    }

    pub fn context(&self) -> egui::CtxRef {
        self.ctx.clone()
    }

    pub fn register_texture(&mut self, id: u64, texture: vk::ImageView, multisampled: bool) {
        self.renderer.register_texture(id, texture, multisampled);
    }
}
