/*
* TODO
* Maybe change pc to work like how json tests describe
* Create better debugger
* Need support for palettes, tile data, background tile maps, vertical scrolling (register 0xFF42), and register @ 0xFF44
* Implement timer
*/

mod drivers;
mod emulator;
mod utils;

use std::error::Error;

use drivers::display::{Display, WHITE};
use emulator::rom::Rom;
use emulator::Emulator;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

fn main() -> Result<(), Box<dyn Error>> {
    let mut display = Display::new(SCREEN_WIDTH, SCREEN_HEIGHT)?;

    let test_rom = Rom::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?;

    let mut emulator = Emulator::new();

    emulator.load_rom(test_rom)?;

    // Gameboy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    display.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    display.set_background(WHITE);

    while display.is_open() {
        let frame = emulator.update()?;

        display.clear();
        display.set_buffer(frame);
        display.render()?;
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcodes() {
        let mut emulator = Emulator::new();
        assert!(emulator._run_opcode_tests().unwrap());
        
    }
}
