mod errors;

use std::error::Error;

use minifb::{Key, Window, WindowOptions};
use errors::*;

use crate::Tile;

pub type Color = u32;

pub const BLACK: Color = 0x00000000;
pub const WHITE: Color = 0x00FFFFFF;

fn rgb(r: u8, g: u8, b: u8) -> u32 {
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

#[allow(dead_code)] 
impl Display {
    pub fn new(width: usize, height: usize) -> Result<Self, minifb::Error> {
        let buffer: Vec<u32> = vec![0; width * height];

        let window_options = WindowOptions {
            borderless: true,
            title: true,
            resize: false,
            scale: minifb::Scale::X4,
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

    pub fn limit_frame_rate(&mut self, rate: Option<std::time::Duration>) {
       self.window.limit_update_rate(rate); 
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open() && !self.window.is_key_down(Key::Escape)
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<(), DrawOutOfBoundsError> {
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

    pub fn set_buffer(&mut self, buff: Vec<u32>) {
        self.buffer = buff;
    }

    pub fn set_background(&mut self, bg: Color) {
        self.background = bg;
    }


    pub fn draw_tile(&mut self, tile_x: usize, tile_y: usize, tile: &Tile) -> Result<(), Box<dyn Error>> {

        let tile_data = tile.get_data();
        let mut pixel_x = tile_x * 8;
        let mut pixel_y = tile_y * 8;
        for data in tile_data {
            let color: Color = match data {
                0 => 0x00FFFFFF,
                1 => 0x00BBBBBB,
                2 => 0x00777777,
                3 => 0x00000000,
                _ => panic!("Should not have any other color here"),
            };

            self.draw_pixel(pixel_x, pixel_y, color)?;
            pixel_x += 1;
            if (pixel_x % 8) == 0 {
                pixel_y += 1;
                pixel_x = tile_x * 8;
            }
        }

        Ok(())
    }
}