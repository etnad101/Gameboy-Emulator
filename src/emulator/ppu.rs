use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::memory::{Bus, DMGBus};
use super::{debug::DebugCtx, LCDRegister};
use crate::utils::frame_buffer::FrameBuffer;
use crate::Palette;
use crate::{utils::bit_ops::BitOps};
pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

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
    pixels: VecDeque<u32>,
    max_size: usize,
}

impl Fifo {
    pub fn new() -> Self {
        Self {
            pixels: VecDeque::new(),
            max_size: 16,
        }
    }

    pub fn push(&mut self, pixel: u32) {
        if self.pixels.len() < self.max_size {
            self.pixels.push_front(pixel);
        } else {
            panic!("PPU::Not sure if I should panic here::Fifo can't hold more pixels")
        }
    }

    pub fn pop(&mut self) -> u32 {
        self.pixels.pop_back().unwrap()
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    pub fn clear(&mut self) {
        self.pixels.clear();
    }
}

pub struct Ppu<B: Bus> {
    memory: Rc<RefCell<B>>,
    debugger: Rc<RefCell<DebugCtx<B>>>,
    frame: FrameBuffer,
    mode: PpuMode,
    current_scanline_cycles: usize,
    fetcher_mode: FetcherMode,
    fetcher_x: u8,
    scanline_x: u8,
    tile_number: u8,
    tile_addr: u16,
    lo_byte: u8,
    hi_byte: u8,
    background_fifo: Fifo,
    object_fifo: Fifo,
    palette: Palette,
    pixels_to_discard: u8,  // For fine scrolling
    // mapped registers
}

impl<B: Bus> Ppu<B> {
    pub fn new(
        memory: Rc<RefCell<B>>,
        debugger: Rc<RefCell<DebugCtx<B>>>,
        palette: Palette,
    ) -> Self {
        memory.borrow_mut().write_u8(LCDRegister::Ly.into(), 0);
        Self {
            memory,
            debugger,
            frame: FrameBuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            mode: PpuMode::OAMScan,
            current_scanline_cycles: 0,
            fetcher_mode: FetcherMode::GetTile,
            fetcher_x: 0,
            scanline_x: 0,
            tile_number: 0,
            tile_addr: 0,
            lo_byte: 0,
            hi_byte: 0,
            background_fifo: Fifo::new(),
            object_fifo: Fifo::new(),
            palette,
            pixels_to_discard: 0,
        }
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    fn write_mem_u8(&self, addr: u16, value: u8) {
        self.memory.borrow_mut().write_u8(addr, value);
    }

    fn read_mem_u8(&self, addr: u16) -> u8 {
        self.memory.borrow().read_u8(addr)
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x > SCREEN_WIDTH {
            panic!("ERROR::PPU attempting to draw outside of buffer (width)")
        }

        if y > SCREEN_HEIGHT {
            panic!("ERROR::PPU attempting to draw outside of buffer (height)")
        }

        let index = (y * SCREEN_WIDTH) + x;
        self.frame.write(index, color);
    }

    fn get_tile_number(&mut self) -> u8 {
        let lcdc = self.read_mem_u8(LCDRegister::Lcdc.into());
        let ly = self.read_mem_u8(LCDRegister::Ly.into()) as u16;
        let scy = self.read_mem_u8(LCDRegister::Scy.into()) as u16;

        let tile_map_base = ((lcdc >> 3) & 1) as u16;
        let tile_num_addr = 0x9800
            | (tile_map_base << 10)
            | ((((ly + scy) & 0xFF) >> 3) << 5)
            | (self.fetcher_x as u16 & 0x1F); 

        self.read_mem_u8(tile_num_addr)
    }

    fn get_tile_data_low(&mut self) -> u8 {
        let lcdc = self.read_mem_u8(LCDRegister::Lcdc.into()) as u16;
        let ly = self.read_mem_u8(LCDRegister::Ly.into()) as u16;
        let scy = self.read_mem_u8(LCDRegister::Scy.into()) as u16;
        let bit_12 = if !(((lcdc & 0x10) > 0) || (self.tile_number & 0x80) > 0) {
            1
        } else {
            0
        };
        self.tile_addr =
            0x8000 | (bit_12 << 12) | ((self.tile_number as u16) << 4) | (((ly + scy) % 8) << 1);
        self.read_mem_u8(self.tile_addr)
    }

    fn get_tile_data_high(&mut self) -> u8 {
        self.read_mem_u8(self.tile_addr + 1)
    }

    fn push_to_fifo(&mut self) {
        for bit in (0..8).rev() {
            let mask = 1 << bit;
            let lo = ((self.lo_byte & mask) >> bit) as u16;
            let hi = ((self.hi_byte & mask) >> bit) as u16;
            let data: u8 = ((hi << 1) | lo) as u8;
            let color: u32 = match data {
                0 => self.palette.0,
                1 => self.palette.1,
                2 => self.palette.2,
                3 => self.palette.3,
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
        let lcdc = self.read_mem_u8(LCDRegister::Lcdc.into());
        if lcdc.get_bit(7) == 0 {
            return;
        }

        for i in 0..cycles {
            self.current_scanline_cycles += 1;
            match self.mode {
                PpuMode::OAMScan => {
                    if self.current_scanline_cycles >= 80 {
                        // Initialize for drawing pixels
                        let scx = self.read_mem_u8(LCDRegister::Scx.into());
                        self.fetcher_x = scx >> 3;
                        self.pixels_to_discard = scx & 7;
                        self.background_fifo.clear();
                        self.fetcher_mode = FetcherMode::GetTile;
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
                                self.fetcher_mode = FetcherMode::Push
                            }
                            FetcherMode::Push => {
                                self.push_to_fifo();
                                self.fetcher_mode = FetcherMode::GetTile;
                            }
                            FetcherMode::Sleep => (),
                        }
                    }

                    if self.background_fifo.len() > 0 {
                        let color = self.background_fifo.pop();
                        
                        // Handle fine scrolling by discarding pixels
                        if self.pixels_to_discard > 0 {
                            self.pixels_to_discard -= 1;
                        } else {
                            let ly = self.read_mem_u8(LCDRegister::Ly.into());
                            self.set_pixel(self.scanline_x as usize, ly as usize, color);
                            self.scanline_x += 1;
                        }
                    }

                    if self.scanline_x >= 160 {
                        self.mode = PpuMode::HBlank;
                    }
                }
                PpuMode::HBlank => {
                    if self.current_scanline_cycles >= CYCLES_PER_SCANLINE {
                        let scx = self.read_mem_u8(LCDRegister::Scx.into());
                        self.scanline_x = 0;
                        self.fetcher_x = scx >> 3;  // Start fetching from the correct tile
                        self.pixels_to_discard = scx & 7;  // Fine scroll offset
                        self.background_fifo.clear();  // Clear FIFO for new scanline
                        self.fetcher_mode = FetcherMode::GetTile;  // Reset fetcher
                        self.current_scanline_cycles = 0;
                        let mut ly = self.read_mem_u8(LCDRegister::Ly.into());
                        ly = ly.wrapping_add(1);
                        self.write_mem_u8(LCDRegister::Ly.into(), ly);
                        if ly >= 144 {
                            self.mode = PpuMode::VBlank
                        } else {
                            self.mode = PpuMode::OAMScan
                        }
                    }
                }
                PpuMode::VBlank => {
                    if self.current_scanline_cycles >= CYCLES_PER_SCANLINE {
                        self.current_scanline_cycles = 0;
                        let mut ly = self.read_mem_u8(LCDRegister::Ly.into());
                        ly = ly.wrapping_add(1);
                        self.write_mem_u8(LCDRegister::Ly.into(), ly);
                        if ly >= 153 {
                            self.write_mem_u8(LCDRegister::Ly.into(), 0);
                            self.mode = PpuMode::OAMScan
                        }
                    }
                }
            }
        }
    }

    pub fn get_frame(&self) -> &FrameBuffer {
        &self.frame
    }
}
