use egui::{Context, Image, TextureOptions, Vec2};
pub struct EmuScreen {
    size: [usize; 2],
    texture: Option<egui::TextureHandle>,
    texture_options: TextureOptions,
}

impl EmuScreen {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            size: [width, height],
            texture: None,
            texture_options: TextureOptions {
                magnification: egui::TextureFilter::Nearest,
                minification: egui::TextureFilter::Nearest,
                wrap_mode: egui::TextureWrapMode::ClampToEdge,
                mipmap_mode: None,
            },
        }
    }
    pub fn update_texture(&mut self, buff: &[u8], ctx: &Context) {
        let image = egui::ColorImage::from_rgb(self.size, buff);
        if let Some(tex) = &mut self.texture {
            tex.set(image, self.texture_options)
        } else {
            self.texture = Some(ctx.load_texture("gb_display", image, self.texture_options))
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(tex) = &self.texture {
            ui.add(Image::new(tex).fit_to_exact_size(Vec2::new(
                self.size[0] as f32 * 2.0,
                self.size[1] as f32 * 2.0,
            )));
        } else {
            ui.label("Please Start Emulator");
        }
    }
}
