use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::memory::MemoryBus;
use super::{Debugger, LCDRegister};
use crate::drivers::display::Color;
use crate::Palette;
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
        } else {
            panic!("PPU::Not sure if I should panic here::Fifo can't hold more pixels")
        }
    }

    pub fn pop(&mut self) -> Color {
        self.pixels.pop_back().unwrap()
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    pub fn clear(&mut self) {
        self.pixels.clear();
    }
}

pub struct Ppu<'a> {
    memory: Rc<RefCell<MemoryBus>>,
    debugger: Rc<RefCell<Debugger<'a>>>,
    buffer: Vec<Color>,
    mode: PpuMode,
    current_scanline_cycles: usize,
    fetcher_mode: FetcherMode,
    fetcher_x: u8,
    fetcher_y: u8,
    scanline_x: u8,
    tile_number: u8,
    tile_addr: u16,
    lo_byte: u8,
    hi_byte: u8,
    background_fifo: Fifo,
    object_fifo: Fifo,
    scanline_has_reset: bool,
    palette: Palette,
}

impl<'a> Ppu<'a> {
    pub fn new(
        memory: Rc<RefCell<MemoryBus>>,
        debugger: Rc<RefCell<Debugger<'a>>>,
        palette: Palette,
    ) -> Self {
        memory.borrow_mut().write_u8(LCDRegister::LY as u16, 0);
        Self {
            memory,
            debugger,
            buffer: vec![WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
            mode: PpuMode::OAMScan,
            current_scanline_cycles: 0,
            fetcher_mode: FetcherMode::GetTile,
            fetcher_x: 0,
            fetcher_y: 0,
            scanline_x: 0,
            tile_number: 0,
            tile_addr: 0,
            lo_byte: 0,
            hi_byte: 0,
            background_fifo: Fifo::new(),
            object_fifo: Fifo::new(),
            scanline_has_reset: false,
            palette,
        }
    }

    fn write_mem_u8(&self, addr: u16, value: u8) {
        self.memory.borrow_mut().write_u8(addr, value);
    }

    fn read_mem_u8(&self, addr: u16) -> u8 {
        self.memory.borrow().read_u8(addr)
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

    fn get_tile_number(&mut self) -> u8 {
        let lcdc = self.read_mem_u8(LCDRegister::LCDC as u16);
        let ly = self.read_mem_u8(LCDRegister::LY as u16) as u16;
        let scx = self.read_mem_u8(LCDRegister::SCX as u16) as u16;
        let scy = self.read_mem_u8(LCDRegister::SCY as u16) as u16;

        let mut tile_num_addr: u16 = if lcdc.get_bit(3) == 0 { 0x9800 } else { 0x9C00 };
        tile_num_addr += self.fetcher_x as u16;
        tile_num_addr += (scx / 8) & 0x1f;
        tile_num_addr += 32 * (((ly + scy) & 0xFF) / 8);

        self.read_mem_u8(tile_num_addr)
    }

    fn get_tile_data_low(&mut self) -> u8 {
        let ly = self.read_mem_u8(LCDRegister::LY as u16) as u16;
        let scy = self.read_mem_u8(LCDRegister::SCY as u16) as u16;

        self.tile_addr = 0x8000 + (16 * self.tile_number as u16);
        self.tile_addr += 2 * ((ly + scy) % 8);
        self.read_mem_u8(self.tile_addr)
    }

    fn get_tile_data_high(&mut self) -> u8 {
        self.read_mem_u8(self.tile_addr + 1)
    }

    fn push_to_fifo(&mut self) {
        for bit in (0..8).rev() {
            let lo = ((self.lo_byte & (1 << bit)) >> bit) as u16;
            let hi = ((self.hi_byte & (1 << bit)) >> bit) as u16;
            let data: u8 = ((hi << 1) | lo) as u8;
            let color: Color = match data {
                0 => self.palette.c0,
                1 => self.palette.c1,
                2 => self.palette.c2,
                3 => self.palette.c3,
                _ => panic!("Should not have any other color here"),
            };
            self.background_fifo.push(color);
        }
        self.fetcher_x += 1;
        if self.fetcher_x >= 32 {
            self.fetcher_x = 0;
        }
    }

    pub fn update_graphics(&mut self, cycles: usize) {
        let lcdc = self.read_mem_u8(LCDRegister::LCDC as u16);
        if lcdc.get_bit(7) == 0 {
            return;
        }

        for i in 0..cycles {
            self.current_scanline_cycles += 1;
            match self.mode {
                PpuMode::OAMScan => {
                    if self.current_scanline_cycles >= 80 {
                        self.mode = PpuMode::DrawingPixels;
                    }
                }
                PpuMode::DrawingPixels => {
                    if i % 2 == 1 {
                        match self.fetcher_mode {
                            FetcherMode::GetTile => {
                                self.tile_number = self.get_tile_number();
                                self.fetcher_mode = FetcherMode::TileDataLow;
                            }
                            FetcherMode::TileDataLow => {
                                self.lo_byte = self.get_tile_data_low();
                                self.fetcher_mode = FetcherMode::TileDataHigh
                            }
                            FetcherMode::TileDataHigh => {
                                self.hi_byte = self.get_tile_data_high();
                                if !self.scanline_has_reset {
                                    self.fetcher_mode = FetcherMode::GetTile;
                                    self.scanline_has_reset = true;
                                } else {
                                    self.fetcher_mode = FetcherMode::Push
                                }
                            }
                            FetcherMode::Push => {
                                self.push_to_fifo();
                                self.fetcher_mode = FetcherMode::GetTile;
                            }
                            FetcherMode::Sleep => (),
                        }
                    }

                    if self.background_fifo.len() > 8 {
                        let color = self.background_fifo.pop();
                        let ly = self.read_mem_u8(LCDRegister::LY as u16);
                        self.set_pixel(self.scanline_x as usize, ly as usize, color);
                        self.scanline_x += 1;
                    }

                    if self.scanline_x >= 160 {
                        self.mode = PpuMode::HBlank;
                    }
                }
                PpuMode::HBlank => {
                    if self.current_scanline_cycles >= 456 {
                        self.scanline_has_reset = false;
                        self.scanline_x = 0;
                        self.current_scanline_cycles = 0;
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
                    if self.current_scanline_cycles >= 456 {
                        self.current_scanline_cycles = 0;
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
