use std::{cell::RefCell, rc::Rc};

use crate::drivers::display::{Color, Display};

use super::memory::MemoryBus;


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
pub enum DebugMode {}

pub struct Debugger<'a> {
    active: bool,
    mode: Option<DebugMode>,
    debug_window: Option<&'a mut Display>,
    memory: Rc<RefCell<MemoryBus>>,
}

impl<'a> Debugger<'a> {
    pub fn new(
        mode: Option<DebugMode>,
        memory: Rc<RefCell<MemoryBus>>,
        debug_window: Option<&'a mut Display>,
    ) -> Self {
        let active = mode.is_some();
        Self {
            active,
            mode,
            debug_window,
            memory,
        }
    }

    pub fn activate(&mut self) {
        self.active = true
    }

    pub fn deactivate(&mut self) {
        self.active = false
    }

    pub fn generate_mem_dump(&mut self) -> Vec<String> {
        let mut mem_log: Vec<String> = Vec::new();
        mem_log.push("\nMEMORY DUMP\n------------------------------------".to_string());
        mem_log.push("\n16KiB ROM Bank 00 | BOOT ROM $0000 - $00FF".to_string());
        for i in 0..self.memory.borrow().get_size() + 1 {
            if i == 0x4000 {
                mem_log.push("\n16 KiB ROM Bank 01-NN".to_string());
            }
            if i == 0x8000 {
                mem_log.push("\nVRAM".to_string());
            }
            if i == 0xA000 {
                mem_log.push("\n8 KiB external RAM".to_string())
            }
            if i == 0xC000 {
                mem_log.push("\n4 KiB WRAM".to_string())
            }
            if i == 0xD000 {
                mem_log.push("\n4 KiB WRAM".to_string())
            }
            if i == 0xE000 {
                mem_log.push("\nEcho RAM".to_string())
            }
            if i == 0xFE00 {
                mem_log.push("\nObject attribute memory (OAM)".to_string())
            }
            if i == 0xFEA0 {
                mem_log.push("\n NOT USEABLE".to_string())
            }
            if i == 0xFF00 {
                mem_log.push("\nI/O Registers".to_string());
            }
            if i == 0xFF80 {
                mem_log.push("\nHigh RAM / HRAM".to_string())
            }

            if i % 32 == 0 {
                mem_log.push(format!("\n|{:#06x}| ", i));
            } else if i % 16 == 0 {
                mem_log.push(format!("|{:#06x}| ", i));
            } else if i % 8 == 0 {
                mem_log.push(' '.to_string());
            }

            let byte: u8 = self.memory.borrow().read_u8(i as u16);
            mem_log.push(format!("{:02x} ", byte));
        }
        mem_log
    }

    pub fn generate_instruction_info(
        &self,
        asm: String,
        pc: u16,
        sp: u16,
        a: u8,
        b: u8,
        c: u8,
        d: u8,
        e: u8,
        f: u8,
        h: u8,
        l: u8,
    ) -> Option<Vec<String>> {
        if self.active {
            let mut instruction_log: Vec<String> = Vec::new();

            instruction_log.push(asm.to_string());
            instruction_log.push(format!("\nStack Pointer: {:#04x}", sp));
            instruction_log.push(format!("\nA: {:#04x}, F: {:#010b}", a, f));
            instruction_log.push(format!("\nB: {:#04x}, C: {:#04x}", b, c));
            instruction_log.push(format!("\nD: {:#04x}, E: {:#04x}", d, e));
            instruction_log.push(format!("\nH: {:#04x}, L: {:#04x}", h, l));
            instruction_log.push("\n".to_string());
            instruction_log.push(format!("\nProgram Counter: {:#04x}; ", pc));

            return Some(instruction_log);
        }
        None
    }

    pub fn render_tiles(&mut self) {
        // TODO: Change to only render when debug mode says to

        match self.debug_window {
            Some(ref mut window) => {
                window.clear();
                let block_size: usize = 16 * 128 * 3;
                let vram_start: usize = 0x8000;
                let tile_data = self.memory.borrow().get_range(vram_start..vram_start + block_size).unwrap();
                let tiles = Tile::parse_tile_data(tile_data);

                let mut tile_x = 0;
                let mut tile_y = 0;


                for tile in tiles {
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
            None => {}
        }
    }
    // TODO: Make function to change tile data based on num key pressed
}