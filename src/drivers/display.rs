use minifb::{Key, Window, WindowOptions};

enum Color {
    BLACK = 0,
    WHITE = (255 << 16) | (255 << 8) | 255,
}

pub struct Display {
    window: Window,
    buffer: Vec<u32>,
    width: usize,
    height: usize,
}

impl Display {
    pub fn new(width: usize, height: usize) -> Result<Self, minifb::Error> {
        let buffer: Vec<u32> = vec![0; width * height];

        let window_options = WindowOptions {
            borderless: true,
            title: true,
            resize: false,
            scale: minifb::Scale::X2,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            topmost: true,
            transparency: false,
            none: false,
        };

        let window = Window::new("Gameboy Emulator", width, height, window_options)?;

        Ok(Display {
            window,
            buffer,
            width,
            height,
        })
    }

    pub fn update(&mut self) -> Result<(), minifb::Error> {
        for pixel in self.buffer.iter_mut() {
            *pixel = Color::WHITE as u32;
        }

        self.window
            .update_with_buffer(&self.buffer, self.width, self.height)?;

        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open() && !self.window.is_key_down(Key::Escape)
    }
}
