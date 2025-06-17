use core::panic;
use std::{cell::RefCell, collections::VecDeque, fs, path::Path, rc::Rc};

use chrono::{DateTime, Local};

use crate::{
    utils::{bit_ops::BitOps, frame_buffer::FrameBuffer},
    Palette,
};

use super::{cpu::registers::Registers, memory::MemoryBus, LCDRegister};

const CALL_LOG_HISTORY_LENGTH: usize = 10;

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

#[derive(PartialEq)]
pub enum DebugFlags {
    ShowTileMap,
    ShowRegisters,
    ShowMemView,
    DumpMem,
    DumpCallLog,
}

pub struct DebugCtx {
    active: bool,
    flags: Vec<DebugFlags>,
    memory: Rc<RefCell<MemoryBus>>,
    palette: Palette,
    call_log: VecDeque<String>,
}

impl DebugCtx {
    pub fn new(flags: Vec<DebugFlags>, memory: Rc<RefCell<MemoryBus>>, palette: Palette) -> Self {
        let active = !flags.is_empty();
        Self {
            active,
            flags,
            memory,
            palette,
            call_log: VecDeque::new(),
        }
    }

    pub fn activate(&mut self) {
        self.active = true
    }

    pub fn deactivate(&mut self) {
        self.active = false
    }

    pub fn push_call_log(&mut self, pc: u16, code: u8, asm: &str) {
        if (!self.active)
            || (!self.flags.contains(&DebugFlags::DumpCallLog)
                && !self.flags.contains(&DebugFlags::ShowRegisters))
        {
            return;
        }

        let msg = format!("pc:{:#06x} -> '{}' ({:#04x})", pc, asm, code);
        self.call_log.push_front(msg);
        if self.call_log.len() > CALL_LOG_HISTORY_LENGTH {
            self.call_log.pop_back();
        }
    }

    pub fn create_call_log_dump(&self) -> Option<String> {
        if (!self.active)
            || (!self.flags.contains(&DebugFlags::DumpCallLog)
                && !self.flags.contains(&DebugFlags::ShowRegisters))
        {
            return None;
        }
        let mut log = String::from("CALL LOG\n------------------------------------\n");
        for instruction in &self.call_log {
            log.push_str(instruction);
            log.push('\n');
        }

        Some(log)
    }

    pub fn create_mem_dump(&self) -> Option<String> {
        if (!self.active) || (!self.flags.contains(&DebugFlags::DumpMem)) {
            return None;
        }

        let mut mem_log: String = String::new();

        mem_log.push_str("\nMEMORY DUMP\n------------------------------------");
        mem_log.push_str("\n16KiB ROM Bank 00 | BOOT ROM $0000 - $00FF");
        for i in 0..=0xFFFF {
            if i == 0x4000 {
                mem_log.push_str("\n16 KiB ROM Bank 01-NN");
            }
            if i == 0x8000 {
                mem_log.push_str("\nVRAM");
            }
            if i == 0xA000 {
                mem_log.push_str("\n8 KiB external RAM");
            }
            if i == 0xC000 {
                mem_log.push_str("\n4 KiB WRAM");
            }
            if i == 0xD000 {
                mem_log.push_str("\n4 KiB WRAM");
            }
            if i == 0xE000 {
                mem_log.push_str("\nEcho RAM");
            }
            if i == 0xFE00 {
                mem_log.push_str("\nObject attribute memory (OAM)");
            }
            if i == 0xFEA0 {
                mem_log.push_str("\n NOT USEABLE");
            }
            if i == 0xFF00 {
                mem_log.push_str("\nI/O Registers");
            }
            if i == 0xFF80 {
                mem_log.push_str("\nHigh RAM / HRAM");
            }

            if i % 32 == 0 {
                mem_log.push_str(&format!("\n|{:#06x}| ", i));
            } else if i % 16 == 0 {
                mem_log.push_str(&format!("|{:#06x}| ", i));
            } else if i % 8 == 0 {
                mem_log.push(' ');
            }

            let byte: u8 = self.memory.borrow().read_u8(i);
            mem_log.push_str(&format!("{:02x} ", byte));
        }
        Some(mem_log)
    }

    pub fn dump_logs(&mut self) {
        let mut log = String::new();
        if let Some(l) = self.create_call_log_dump() {
            log.push_str(&l)
        }
        if let Some(l) = self.create_mem_dump() {
            log.push_str(&l)
        }

        if log == String::new() {
            return;
        }

        let dt = Local::now();
        let native_utc = dt.naive_utc();
        let offset = *dt.offset();
        let now = DateTime::<Local>::from_naive_utc_and_offset(native_utc, offset).to_string();
        let log_name =
            "crash_log".to_string() + &now.replace(' ', "_").replace(':', "-").replace('.', "_");
        if !Path::new("./logs/").exists() {
            fs::create_dir("./logs").expect("Unable to create log directory")
        };
        let path = "./logs/".to_string() + &log_name;
        fs::File::create(path.clone()).expect("unable to create file");
        fs::write(path, log).expect("unable to write to file");
    }

    pub fn render_tiles(&mut self) -> FrameBuffer {
        let width = 128;
        let height = 192;
        let mut buff = FrameBuffer::new(width, height);

        let block_size: u16 = 16 * 128 * 3;
        let vram_start: u16 = 0x8000;
        let tile_data = self
            .memory
            .borrow()
            .get_range(vram_start..vram_start + block_size);
        let tiles = Tile::parse_tile_data(tile_data);

        let mut tile_x = 0;
        let mut tile_y = 0;

        for tile in tiles {
            let tile_data = tile.get_data();
            let mut pixel_x = tile_x * 8;
            let mut pixel_y = tile_y * 8;
            for data in tile_data {
                let color: u32 = match data {
                    0 => self.palette.0,
                    1 => self.palette.1,
                    2 => self.palette.2,
                    3 => self.palette.3,
                    _ => panic!("Should not have any other color here"),
                };

                let pos = (pixel_y * width) + pixel_x;
                buff.write(pos, color);

                pixel_x += 1;
                if (pixel_x % 8) == 0 {
                    pixel_y += 1;
                    pixel_x = tile_x * 8;
                }
            }
            tile_x += 1;
            if tile_x >= 16 {
                tile_x = 0;
                tile_y += 1;
            }
        }

        buff
    }

    pub fn render_background_map(&mut self) -> FrameBuffer {
        let width = 32 * 8;
        let height = 32 * 8;
        let mut buff = FrameBuffer::new(width, height);
        let mut tile_x = 0;
        let mut tile_y = 0;
        for tile in 0..32 * 32 {
            let lcdc = self.memory.borrow().read_u8(LCDRegister::Lcdc.into());
            let tile_num_base: u16 = if lcdc.get_bit(3) == 0 { 0x9800 } else { 0x9C00 };
            let tile_number_addr = tile_num_base + tile;
            let tile_number = self.memory.borrow().read_u8(tile_number_addr);
            let tile_data_addr = 0x8000 + (16 * tile_number as u16) as usize;
            let tile_data = self
                .memory
                .borrow()
                .get_range(tile_data_addr as u16..tile_data_addr as u16 + 16);
            let mut pixel_x = tile_x * 8;
            let mut pixel_y = tile_y * 8;
            let mut i = 0;
            while i < 16 {
                let lo_byte = tile_data[i];
                let hi_byte = tile_data[i + 1];
                for bit in (0..8).rev() {
                    let lo = ((lo_byte & (1 << bit)) >> bit) as u16;
                    let hi = ((hi_byte & (1 << bit)) >> bit) as u16;
                    let color_data: u8 = ((hi << 1) | lo) as u8;
                    let color: u32 = match color_data {
                        0 => self.palette.0,
                        1 => self.palette.1,
                        2 => self.palette.2,
                        3 => self.palette.3,
                        _ => panic!("Should not have any other color here"),
                    };
                    let pos = (pixel_y * width) + pixel_x;
                    buff.write(pos, color);
                    pixel_x += 1;
                }
                i += 2;
                if (pixel_x % 8) == 0 {
                    pixel_y += 1;
                    pixel_x = tile_x * 8;
                }
            }
            tile_x += 1;
            if tile_x >= 32 {
                tile_x = 0;
                tile_y += 1;
            }
        }
        buff
    }
}
