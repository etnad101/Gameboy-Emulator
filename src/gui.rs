use eframe::Frame;
use egui::{Context, Image, TextureOptions, Vec2};

use crate::{emulator::{Emulator, SCREEN_HEIGHT, SCREEN_WIDTH}, utils::frame_buffer::FrameBuffer};
use crate::emulator::DMGBus;

pub struct EmulatorGui {
    emulator: Emulator<DMGBus>,
    frame_buffer: Vec<u8>,
}

impl EmulatorGui {
    pub fn new(emulator: Emulator<DMGBus>) -> Self {
        Self {
            emulator,
            frame_buffer: vec![],
        }
    }
}

impl eframe::App for EmulatorGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.frame_buffer = self.emulator.tick().unwrap().rgb();

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.frame_buffer.is_empty() {
                let size = [SCREEN_WIDTH, SCREEN_HEIGHT];
                let image = egui::ColorImage::from_rgb(size, &self.frame_buffer);
                let texture = ctx.load_texture(
                    "gb_display",
                    image,
                    TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Nearest,
                        wrap_mode: egui::TextureWrapMode::ClampToEdge,
                        mipmap_mode: None,
                    },
                );
                ui.add(Image::new(&texture).fit_to_exact_size(Vec2::new(
                    SCREEN_WIDTH as f32 * 2.0,
                    SCREEN_HEIGHT as f32 * 2.0,
                )));
            }
        });

        ctx.request_repaint();
    }
}
