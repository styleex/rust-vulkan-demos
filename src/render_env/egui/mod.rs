use crate::utils::texture::Texture;
use crate::render_env::env::RenderEnv;

pub struct EguiRenderer {
    texture: Texture,
}

impl EguiRenderer {
   pub fn new(env: &RenderEnv, ctx: egui::CtxRef) -> EguiRenderer {
        let font_tx = ctx.texture();
        let texture = Texture::from_pixels(
            env.device().clone(),
            env.command_pool(),
            env.queue(),
            &env.mem_properties,
            &font_tx.pixels,
            font_tx.width as u32,
            font_tx.height as u32,
        );

        EguiRenderer {
            texture,
        }
    }
}

impl Drop for EguiRenderer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
