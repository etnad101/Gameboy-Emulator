/*
* TODO
* Create better debugger
* Need support for palettes, tile data, background tile maps, vertical scrolling (register 0xFF42), and register @ 0xFF44
* Implement timer
*/

mod emulator;
mod drivers;
mod utils;

use std::error::Error;

use emulator::Emulator;
use emulator::rom::Rom;
use drivers::display::{Display, WHITE};

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

fn main() -> Result<(), Box<dyn Error>> {
    let mut display = Display::new(SCREEN_WIDTH, SCREEN_HEIGHT)?;

    let test_rom = Rom::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?;
    let tetris = Rom::from("./roms/games/Tetris (World) (Rev A).gb")?;

    let mut emulator = Emulator::new();

    emulator.load_rom(tetris);

    // Gameboy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    display.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    display.set_background(WHITE);

    while display.is_open() {
        let frame = emulator.update();

        display.clear();
        display.set_buffer(frame);
        display.render()?;
    }

    Ok(())
}
