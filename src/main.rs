/*
* TODO
* Create Logger for CPU
* Create live debugger
* Need support for palettes, tile data, background tile maps, vertical scrolling (register 0xFF42), and register @ 0xFF44
* Implement timer
*/

mod emulator;
mod drivers;
mod utils;

use std::error::Error;

use emulator::Emulator;
use emulator::rom::Rom;
use drivers::display::{Display, Color, WHITE};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

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

        Tile {
            data: tile,
        }
    }

    pub fn get_data(&self) -> [u8; 64] {
        self.data
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


fn main() -> Result<(), Box<dyn Error>> {
    let mut display = Display::new(WIDTH, HEIGHT)?;

    let test_rom = Rom::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?;
    let tetris = Rom::from("./roms/games/Tetris (World) (Rev A).gb")?;

    let mut emulator = Emulator::new();

    emulator.load_rom(tetris);

    // Gameboy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    display.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    display.set_background(WHITE);

    while display.is_open() {
        display.clear();
        emulator.update();
        display.render()?;
    }

    Ok(())
}
