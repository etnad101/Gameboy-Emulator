use std::{cell::RefCell, collections::VecDeque, fs, path::Path, rc::Rc};

use chrono::{DateTime, Local};

use crate::Palette;
use simple_graphics::display::{Color, Display};

use super::memory::MemoryBus;

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
    DumpMem,
    DumpCallLog,
}

pub struct Debugger<'a> {
    active: bool,
    flags: Vec<DebugFlags>,
    tile_window: Option<&'a mut Display>,
    memory: Rc<RefCell<MemoryBus>>,
    palette: Palette,
    call_log: VecDeque<String>,
}

impl<'a> Debugger<'a> {
    pub fn new(
        flags: Vec<DebugFlags>,
        memory: Rc<RefCell<MemoryBus>>,
        tile_window: Option<&'a mut Display>,
        palette: Palette,
    ) -> Self {
        let active = !flags.is_empty();
        Self {
            active,
            flags,
            tile_window,
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
        if (!self.active) || (!self.flags.contains(&DebugFlags::DumpCallLog)) {
            return;
        }
        let msg = format!("{:#06x}: '{}' ({:#04x})", pc, asm, code);
        self.call_log.push_front(msg);
        if self.call_log.len() > CALL_LOG_HISTORY_LENGTH {
            self.call_log.pop_back();
        }
    }

    pub fn create_call_log_dump(&self) -> Option<String> {
        if (!self.active) || (!self.flags.contains(&DebugFlags::DumpCallLog)) {
            return None;
        }
        let mut log = String::from("CALL LOG\n------------------------------------\n");
        for intruction in &self.call_log {
            log.push_str(intruction);
            log.push_str("\n");
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
        for i in 0..self.memory.borrow().get_size() + 1 {
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

            let byte: u8 = self.memory.borrow().read_u8(i as u16);
            mem_log.push_str(&format!("{:02x} ", byte));
        }
        Some(mem_log)
    }

    pub fn dump_logs(&mut self) {
        let mut log = String::new();
        match self.create_call_log_dump() {
            Some(l) => log.push_str(&l),
            None => (),
        }
        match self.create_mem_dump() {
            Some(l) => log.push_str(&l),
            None => (),
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

    pub fn render_tiles(&mut self) {
        if (!self.active) || (!self.flags.contains(&DebugFlags::ShowTileMap)) {
            return;
        }

        match self.tile_window {
            Some(ref mut window) => {
                let (length, width) = window.size();
                // Check to see if window can hold tiles evenly without wrapping around or
                // throwing a drawing error. Draw function expects to draw without wrapping
                if (width % 8) != 0 || (length % 8) != 0 {
                    panic!("Width and height must be multiples of 8, 128x192 recomended")
                }
                // Check to see if the window has enough pixels to hold all the tiles
                // 384 tiles * 64 pixels each = 24576 pixels
                if (length * width) < 24576 {
                    panic!("Window not big enough to display all tiles, 128x192 recomended")
                }

                window.clear();
                let block_size: usize = 16 * 128 * 3;
                let vram_start: usize = 0x8000;
                let tile_data = self
                    .memory
                    .borrow()
                    .get_range(vram_start..vram_start + block_size)
                    .unwrap();
                let tiles = Tile::parse_tile_data(tile_data);

                let mut tile_x = 0;
                let mut tile_y = 0;

                for tile in tiles {
                    let tile_data = tile.get_data();
                    let mut pixel_x = tile_x * 8;
                    let mut pixel_y = tile_y * 8;
                    for data in tile_data {
                        let color: Color = match data {
                            0 => self.palette.c0,
                            1 => self.palette.c1,
                            2 => self.palette.c2,
                            3 => self.palette.c3,
                            _ => panic!("Should not have any other color here"),
                        };
                        window.draw_pixel(pixel_x, pixel_y, color).unwrap();
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
                window.render().unwrap();
            }
            None => panic!("Must Provide window for tile map to be drawn to"),
        }
    }
}
