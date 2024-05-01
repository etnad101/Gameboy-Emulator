use core::fmt;

use minifb::{Key, Window, WindowOptions};

pub type Color = u32;

pub const BLACK: Color = rgb(0, 0, 0);
pub const WHITE: Color = rgb(255, 255, 255);

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
}

pub struct Display {
    window: Window,
    buffer: Vec<Color>,
    width: usize,
    height: usize,
    background: Color,
}

#[derive(Debug)]
pub enum DrawOutOfBoundsError {
    X(usize),
    Y(usize), 
}

impl fmt::Display for DrawOutOfBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawOutOfBoundsError::X(val) => write!(f, "Tried drawing out of bound at x = {}", val),
            DrawOutOfBoundsError::Y(val) => write!(f, "Tried drawing out of bound at y = {}", val),
        }
    }
}

impl std::error::Error for DrawOutOfBoundsError {}

#[allow(dead_code)] 
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
            background: BLACK,
        })
    }

    pub fn render(&mut self) -> Result<(), minifb::Error> {
        self.window
            .update_with_buffer(&self.buffer, self.width, self.height)?;

        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open() && !self.window.is_key_down(Key::Escape)
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<(), DrawOutOfBoundsError>{
        if x > self.width {
            return Err(DrawOutOfBoundsError::X(x))
        }

        if y > self.height {
            return Err(DrawOutOfBoundsError::Y(y))
        }

        let index = (y * self.width) + x;
        self.buffer[index] = color; 

        Ok(())
    }

    pub fn clear(&mut self) {
        for pixel in self.buffer.iter_mut() {
            *pixel = self.background;
        }
    }

    pub fn set_background(&mut self, bg: Color) {
        self.background = bg;
    }

}
