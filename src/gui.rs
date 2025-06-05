use eframe::Frame;
use egui::{Context, TextureOptions};

use crate::{emulator::Emulator, utils::frame_buffer::FrameBuffer, SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct EmulatorGui {
    emulator: Emulator,
    frame_buffer: Vec<u8>,
}

impl EmulatorGui {
    pub fn new(emulator: Emulator) -> Self {
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
                ui.image(&texture);
            }
        });

        ctx.request_repaint();
    }
}
