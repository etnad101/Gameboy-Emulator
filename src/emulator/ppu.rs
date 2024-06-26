use std::error::Error;

use super::memory::MemoryBus;
use super::LCDRegister;
use crate::drivers::display::Color;
use crate::{drivers::display::WHITE, utils::GetBit};
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

const CYCLES_PER_SCANLINE: usize = 456 / 4;

struct Tile {
    data: [u8; 64],
}

impl Tile {
    pub fn from(tile_data: &[u8]) -> Tile {
        let mut i = 0;
        let mut ptr: usize = 0;
        let mut tile: [u8; 64] = [0; 64];

        while i < 16 {
            let lo_byte = tile_data[i];
            let hi_byte = tile_data[i + 1];

            for bit in (0..8).rev() {
                let lo = ((lo_byte & (1 << bit)) >> bit) as u16;
                let hi = ((hi_byte & (1 << bit)) >> bit) as u16;
                let data: u8 = ((hi << 1) | lo) as u8;

                tile[ptr] = data;
                ptr += 1;
            }
            i += 2;
        }

        Tile { data: tile }
    }

    pub fn get_data(&self) -> [u8; 64] {
        self.data
    }
}

pub struct Ppu {
    buffer: Vec<Color>,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            buffer: vec![WHITE; 256 * 256],
        }
    }

    fn parse_tile_data(data: Vec<u8>) -> Vec<Tile> {
        let mut tiles: Vec<Tile> = Vec::new();

        let mut i = 0;
        while i < data.len() {
            let tile_end = i + 16;
            let tile = Tile::from(&data[i..tile_end]);
            tiles.push(tile);
            i += 16;
        }

        tiles
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x > SCREEN_WIDTH {
            panic!("ERROR::PPU attempting to draw outside of buffer (width)")
        }

        if y > SCREEN_HEIGHT {
            panic!("ERROR::PPU attempting to draw outside of buffer (height)")
        }

        let index = (y * SCREEN_WIDTH) + x;
        self.buffer[index] = color;
    }

    pub fn draw_tile(
        &mut self,
        tile_x: usize,
        tile_y: usize,
        tile: &Tile,
    ) -> Result<(), Box<dyn Error>> {
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

            self.draw_pixel(pixel_x, pixel_y, color);
            pixel_x += 1;
            if (pixel_x % 8) == 0 {
                pixel_y += 1;
                pixel_x = tile_x * 8;
            }
        }

        Ok(())
    }

    pub fn update_graphics(&mut self, memory: &mut MemoryBus, cycles: u32) {
        if (cycles as usize % CYCLES_PER_SCANLINE) == 0 {
            let mut ly = memory.read_u8(LCDRegister::LY as u16).wrapping_add(1);
            if ly == 154 {
                ly = 0;
            }
            memory.write_u8(LCDRegister::LY as u16, ly);
        }
    }

    pub fn render_screen(&self, memory: &mut MemoryBus) -> Vec<u32> {
        let mut buff = vec![WHITE; 160 * 144];

        // get tile
        let fetcher_x = 0;
        let fetcher_y = 0;
        let lcdc = memory.read_u8(LCDRegister::LCDC as u16);
        // change false to chekc if x coordinate of current scanline is in window
        let tilemap_base = if (lcdc.get_bit(3) == 1) && (false) {
            0x9c00
        } else if (lcdc.get_bit(6) == 1) && (false) {
            0x9c00
        } else {
            0x9800
        };

        let tilemap_addr = tilemap_base + fetcher_x;
        let tile_offset = memory.read_u8(tilemap_addr) as u16;

        let tile_addr = if lcdc.get_bit(4) == 1 {
            0x8000 + (tile_offset * 16)
        } else {
            let offset = (tile_offset as i8) as i32 * 16;
            (0x9000 + offset) as u16
        };

        // additional calculation needed to find the 160x144 area from the 256x256 tile map

        buff
    }
}
