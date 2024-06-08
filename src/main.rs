/*
* TODO
* Create Logger for CPU
* Implement LCD status registers
*/

mod cpu;
mod drivers;

use std::error::Error;

use cpu::CPU;
use drivers::display::{Display, Color, WHITE};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

struct Tile {
    data: [u8; 64],
}

impl Tile {
    pub fn from(tile_data: [u8; 16]) -> Tile {
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut display = Display::new(WIDTH, HEIGHT)?;

    let mut cpu = CPU::new();

    // Gameboy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    display.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    display.set_background(WHITE);

    while display.is_open() {
        display.clear();
        cpu.update();
        display.render()?;
    }

    Ok(())
}
