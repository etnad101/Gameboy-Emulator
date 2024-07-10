use std::error::Error;

use super::errors::MemError;
use super::memory::MemoryBus;
use super::{LCDRegister, MAX_CYCLES_PER_FRAME};
use crate::drivers::display::Color;
use crate::{drivers::display::WHITE, SCREEN_HEIGHT, SCREEN_WIDTH, utils::GetBit};

const CYCLES_PER_SCANLINE: usize = 456;

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
}

enum PpuMode {
    Hblank,
    Vblank,
    OAMScan,
    DrawingPixels,
}

pub struct Ppu {
    buffer: Vec<Color>,
    mode: PpuMode,
    current_scanline_cycles: usize,
    fetcher_x: u8,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            buffer: vec![WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
            mode: PpuMode::OAMScan,
            current_scanline_cycles: 0,
            fetcher_x: 0,
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x > SCREEN_WIDTH {
            panic!("ERROR::PPU attempting to draw outside of buffer (width)")
        }

        if y > SCREEN_HEIGHT {
            panic!("ERROR::PPU attempting to draw outside of buffer (height)")
        }

        let index = (y * 256) + x;
        self.buffer[index] = color;
    }

    fn draw_tile(
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

            self.set_pixel(pixel_x, pixel_y, color);
            pixel_x += 1;
            if (pixel_x % 8) == 0 {
                pixel_y += 1;
                pixel_x = tile_x * 8;
            }
        }

        Ok(())
    }

    pub fn update_graphics(&mut self, memory: &mut MemoryBus, cycles: usize) {
        let mut ly = memory.read_u8(LCDRegister::LY as u16);
        let lcdc = memory.read_u8(LCDRegister::LCDC as u16);
        let scy = memory.read_u8(LCDRegister::SCY as u16);
        let scx = memory.read_u8(LCDRegister::SCX as u16);

        self.current_scanline_cycles += cycles;

        if self.current_scanline_cycles >= CYCLES_PER_SCANLINE {
            self.current_scanline_cycles = 0;
            ly = ly.wrapping_add(1);
            if ly > 153 {
                ly = 0;
            }
            memory.write_u8(LCDRegister::LY as u16, ly);
        }

        for _ in 0..(cycles / 2) {
            self.mode = if ly >= 144 {
                PpuMode::Vblank
            } else if self.current_scanline_cycles <= 80 {
                PpuMode::OAMScan
            } else {
                PpuMode::DrawingPixels
            };

            // TODO: get tile number
            let tile_number_base: u16 = if lcdc.get_bit(3) == 1 {
                0x9C00
            } else {
                0x9800
            };

            let mut tile_number_offset: u16 = self.fetcher_x as u16;
            if lcdc.get_bit(5) == 0 {
                tile_number_offset += ((scx / 8) & 0x1F) as u16;
                tile_number_offset += 32 * (((ly as u16 + scy as u16) & 0xFF) / 8);
            }

            let tile_number = memory.read_u8(tile_number_base + tile_number_offset);

            // get tile
            let tile_offset = 2 * ((ly as u16 + scy as u16) % 8);
            let tile_base = if lcdc.get_bit(4) == 1 {
                let base: u16 = 0x8000;
                base + (tile_number as u16 * 16)
            } else {
                let base: isize = 0x9000;
                let offset: isize = tile_number as isize * 16;
                (base + offset) as u16
            };

            let tile_addr = tile_base + tile_offset;

            let lo_byte = memory.read_u8(tile_addr);
            let hi_byte = memory.read_u8(tile_addr + 1);

            for bit in (0..8).rev() {
                let lo = ((lo_byte & (1 << bit)) >> bit) as u16;
                let hi = ((hi_byte & (1 << bit)) >> bit) as u16;
                let data: u8 = ((hi << 1) | lo) as u8;
                let color: Color = match data {
                    0 => 0x00FFFFFF,
                    1 => 0x00BBBBBB,
                    2 => 0x00777777,
                    3 => 0x00000000,
                    _ => panic!("Should not have any other color here"),
                };
            }

            self.fetcher_x += 1;
            if self.fetcher_x > 31 {
                self.fetcher_x = 0;
            }
        }
    }

    pub fn get_frame(&self) -> Vec<Color> {
        // let mut buff: Vec<Color> = vec![WHITE; 160 * 144];
        //
        // for y in 0..144 {
        //     for x in 0..160 {
        //         let orig_addr = (y * 256) + x;
        //         let target_addr = (y * 160) + x;
        //         buff[target_addr] = self.buffer[orig_addr];
        //     }
        // }
        // buff
        self.buffer.clone()
    }
}
