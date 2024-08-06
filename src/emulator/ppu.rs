use std::cell::RefCell;
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

struct Fetcher {
    x: u8,
}

impl Fetcher {
    pub fn new() -> Self {
        Self {
            x: 0,
        }
    }
}

pub struct Ppu<'a> {
    memory: Rc<RefCell<MemoryBus>>,
    debugger: Rc<RefCell<Debugger<'a>>>,
    buffer: Vec<Color>,
    mode: PpuMode,
    current_scanline_cycles: usize,
    fetcher_x: u8,
    hx: usize,
}

impl<'a> Ppu<'a> {
    pub fn new(memory: Rc<RefCell<MemoryBus>>, debugger: Rc<RefCell<Debugger<'a>>>) -> Self {
        Self {
            memory,
            debugger,
            buffer: vec![WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
            mode: PpuMode::OAMScan,
            current_scanline_cycles: 0,
            fetcher_x: 0,
            hx: 0,
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

    pub fn update_graphics(&mut self, cycles: usize) {
        let mut ly = self.read_mem_u8(LCDRegister::LY as u16);
        let lcdc = self.read_mem_u8(LCDRegister::LCDC as u16);
        let scy = self.read_mem_u8(LCDRegister::SCY as u16);
        let scx = self.read_mem_u8(LCDRegister::SCX as u16);

        self.current_scanline_cycles += cycles;

        if self.current_scanline_cycles >= CYCLES_PER_SCANLINE {
                self.current_scanline_cycles = 0;
                ly = ly.wrapping_add(1);
            if ly > 153 {
                    ly = 0;
                }
                self.write_mem_u8(LCDRegister::LY as u16, ly);
            }

        for _ in 0..(cycles / 2) {
            self.mode = if ly >= 144 {
                PpuMode::VBlank
            } else if self.current_scanline_cycles <= 80 {
                PpuMode::OAMScan
            } else if self.current_scanline_cycles <= 289 {
                PpuMode::DrawingPixels
            } else {
                PpuMode::HBlank
            };

            match self.mode {
                PpuMode::DrawingPixels => {
                    let tile_number_base: u16 = if lcdc.get_bit(3) == 1 { 0x9C00 } else { 0x9800 };

                    let mut tile_number_offset: u16 = self.fetcher_x as u16;
                    if lcdc.get_bit(5) == 0 {
                        tile_number_offset += ((scx / 8) & 0x1F) as u16;
                        tile_number_offset += 32 * ((((ly as u16 + scy as u16) & 0xFF) / 8) & 0x1F);
                    }

                    let tile_number = self.read_mem_u8(tile_number_base + tile_number_offset);

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

                    let lo_byte = self.read_mem_u8(tile_addr);
                    let hi_byte = self.read_mem_u8(tile_addr + 1);

                    for bit in (0..8).rev() {
                        let lo = ((lo_byte & (1 << bit)) >> bit) as u16;
                        let hi = ((hi_byte & (1 << bit)) >> bit) as u16;
                        let data: u8 = ((hi << 1) | lo) as u8;
                        let color: Color = match data {
                            0 => COLOR_0,
                            1 => COLOR_1,
                            2 => COLOR_2,
                            3 => COLOR_3,
                            _ => panic!("Should not have any other color here"),
                        };
                        self.set_pixel(self.hx, ly as usize, color);
                        self.hx += 1;
                        if self.hx >= SCREEN_WIDTH {
                            self.hx = 0;
                        }
                    }

                    self.fetcher_x += 1;
                    if self.fetcher_x > 31 {
                        self.fetcher_x = 0;
                    }
                }
                _ => (),
            }
        }
    }

    pub fn get_frame(&self) -> Vec<Color> {
        self.buffer.clone()
    }
}
