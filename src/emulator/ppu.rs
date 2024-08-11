use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::memory::MemoryBus;
use super::{Debugger, LCDRegister};
use crate::drivers::display::Color;
use crate::{COLOR_0, COLOR_1, COLOR_2, COLOR_3};
use crate::{drivers::display::WHITE, utils::GetBit, SCREEN_HEIGHT, SCREEN_WIDTH};

const CYCLES_PER_SCANLINE: usize = 456;

enum PpuMode {
    HBlank,
    VBlank,
    OAMScan,
    DrawingPixels,
}

enum FetcherMode {
    GetTile,
    TileDataLow,
    TileDataHigh,
    Sleep,
    Push,
}

struct Fifo {
    pixels: VecDeque<Color>,
    max_size: usize,
}

impl Fifo {
    pub fn new() -> Self {
        Self {
           pixels: VecDeque::new(), 
           max_size: 16,
        }
    }

    pub fn push(&mut self, pixel: Color) {
        if self.pixels.len() < self.max_size {
            self.pixels.push_front(pixel);
        }
    }

    pub fn pop(&mut self) -> Color {
        self.pixels.pop_back().unwrap()
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }
}

pub struct Ppu<'a> {
    memory: Rc<RefCell<MemoryBus>>,
    debugger: Rc<RefCell<Debugger<'a>>>,
    buffer: Vec<Color>,
    mode: PpuMode,
    current_scanline_dots: usize,
    fetcher_mode: FetcherMode,
    fetcher_x: u8,
    fetcher_y: u8,
    scanline_x: u8,
    tile_addr: u16,
    lo_byte: u8,
    hi_byte: u8,
    background_fifo: Fifo,
    object_fifo: Fifo,
}

impl<'a> Ppu<'a> {
    pub fn new(memory: Rc<RefCell<MemoryBus>>, debugger: Rc<RefCell<Debugger<'a>>>) -> Self {
        memory.borrow_mut().write_u8(LCDRegister::LY as u16, 0);
        Self {
            memory,
            debugger,
            buffer: vec![WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
            mode: PpuMode::OAMScan,
            current_scanline_dots: 0,
            fetcher_mode: FetcherMode::GetTile,
            fetcher_x: 0,
            fetcher_y: 0,
            scanline_x: 0,
            tile_addr: 0,
            lo_byte: 0,
            hi_byte: 0,
            background_fifo: Fifo::new(),
            object_fifo: Fifo::new(),
        }
    }

    fn write_mem_u8(&self, addr: u16, value: u8) {
        self.memory.borrow_mut().write_u8(addr, value);
    }

    fn read_mem_u8(&self, addr: u16) -> u8 {
        self.memory.borrow().read_u8(addr)
    }

    fn read_mem_u16(&self, addr: u16) -> u16 {
        self.memory.borrow().read_u16(addr)
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x > SCREEN_WIDTH {
            panic!("ERROR::PPU attempting to draw outside of buffer (width)")
        }

        if y > SCREEN_HEIGHT {
            panic!("ERROR::PPU attempting to draw outside of buffer (height)")
        }
        
        let index = (y * SCREEN_WIDTH) + x;
        self.buffer[index] = color;
    }

    fn get_tile(&mut self) -> u16 {
        let lcdc = self.read_mem_u8(LCDRegister::LCDC as u16);
        let ly = self.read_mem_u8(LCDRegister::LY as u16);
        let scx = self.read_mem_u8(LCDRegister::SCX as u16);
        let scy = self.read_mem_u8(LCDRegister::SCY as u16);

        // Check to see if current tile is a window tile or not
        if lcdc.get_bit(5) == 0 {
            self.fetcher_x = ((scx / 8) + self.scanline_x) & 0x1F;
            self.fetcher_y = (ly + scy) & 0xFF;
        } else {
            // If window tile, use x and y coord for window tile
        }

        let tile_map_base: u16 = if lcdc.get_bit(3) == 0 { 0x9800 } else { 0x9C00 };
        let tile_map_addr = tile_map_base + self.fetcher_x as u16 + ((self.fetcher_y as u16 / 8) * 32);
        let tile_number = self.read_mem_u8(tile_map_addr) as u16;

        let tile_addr = if lcdc.get_bit(4) == 1 {
            let base: u16 = 0x8000;
            base + (tile_number as u16 * 16)
        } else {
            let base: isize = 0x9000;
            let offset: isize = tile_number as isize * 16;
            (base + offset) as u16
        };

        // Tile address
        tile_addr
    }
    
    fn get_tile_data_low(&mut self) -> u8 {
        self.tile_addr += 2 * (self.fetcher_y as u16 % 8);
        self.read_mem_u8(self.tile_addr)
    }

    fn get_tile_data_high(&mut self) -> u8 {
        self.tile_addr += 2 * (self.fetcher_y as u16 % 8);
        self.read_mem_u8(self.tile_addr + 1)
    }

    fn push_to_fifo(&mut self) {
        for bit in (0..8).rev() {
            let lo = ((self.lo_byte & (1 << bit)) >> bit) as u16;
            let hi = ((self.hi_byte & (1 << bit)) >> bit) as u16;
            let data: u8 = ((hi << 1) | lo) as u8;
            let color: Color = match data {
                0 => COLOR_0,
                1 => COLOR_1,
                2 => COLOR_2,
                3 => COLOR_3,
                _ => panic!("Should not have any other color here"),
            };
            self.background_fifo.push(color);
        }
    }

    pub fn update_graphics(&mut self, cycles: usize) {
        let lcdc = self.read_mem_u8(LCDRegister::LCDC as u16);
        if lcdc.get_bit(7) == 0 {
            return
        }

        self.current_scanline_dots += cycles;

        for i in 0..cycles {
            let ly = self.read_mem_u8(LCDRegister::LY as u16);
            match self.mode {
                PpuMode::OAMScan => {
                    if self.current_scanline_dots >= 80 {
                        self.mode = PpuMode::DrawingPixels;
                    }
                }
                PpuMode::DrawingPixels => {
                    if i % 2 == 0 {
                        match self.fetcher_mode {
                            FetcherMode::GetTile => {
                                self.tile_addr = self.get_tile();
                                self.fetcher_mode = FetcherMode::TileDataLow;
                            }
                            FetcherMode::TileDataLow => {
                                self.lo_byte = self.get_tile_data_low();
                                self.fetcher_mode = FetcherMode::TileDataHigh
                            }
                            FetcherMode::TileDataHigh => {
                                self.hi_byte = self.get_tile_data_high();
                                self.fetcher_mode = FetcherMode::Push
                            }
                            FetcherMode::Push => {
                                self.push_to_fifo();
                                self.fetcher_mode = FetcherMode::GetTile;
                            }
                            FetcherMode::Sleep => ()
                        }
                    }

                    if self.background_fifo.len() > 8 {
                        let color = self.background_fifo.pop();
                        let ly = self.read_mem_u8(LCDRegister::LY as u16);
                        self.set_pixel(self.scanline_x as usize, ly as usize, color);
                        self.scanline_x += 1;
                    }

                    if self.scanline_x >= 160 {
                        self.scanline_x = 0;
                        self.mode = PpuMode::HBlank;
                    }
                }
                PpuMode::HBlank => {
                    if self.current_scanline_dots >= 456 {
                        self.current_scanline_dots = 0;
                        let mut ly = self.read_mem_u8(LCDRegister::LY as u16);
                        ly = ly.wrapping_add(1);
                        self.write_mem_u8(LCDRegister::LY as u16, ly);
                        if ly >= 144 {
                            self.mode = PpuMode::VBlank
                        } else {
                            self.mode = PpuMode::OAMScan
                        }
                    }
                }
                PpuMode::VBlank => {
                    if self.current_scanline_dots >= 456 {
                        self.current_scanline_dots = 0;
                        let mut ly = self.read_mem_u8(LCDRegister::LY as u16);
                        ly = ly.wrapping_add(1);
                        self.write_mem_u8(LCDRegister::LY as u16, ly);
                        if ly >= 153 {
                            self.write_mem_u8(LCDRegister::LY as u16, 0);
                            self.mode = PpuMode::OAMScan
                        }
                    }
                }
            }
        }
    }

    pub fn get_frame(&self) -> Vec<Color> {
        self.buffer.clone()
    }
}
