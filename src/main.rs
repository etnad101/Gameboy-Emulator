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

use drivers::display::{Color, Display, WHITE};
use emulator::{debugger::DebugFlags, rom::Rom, Emulator};

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

// Palette Colors
const COLOR_0: Color = 0x009BBC0F;
const COLOR_1: Color = 0x008BAC0F;
const COLOR_2: Color = 0x00306230;
const COLOR_3: Color = 0x000F380F;

fn main() -> Result<(), Box<dyn Error>> {
    // Init windows
    let mut emulator_window = Display::new("Game Boy Emulator", SCREEN_WIDTH, SCREEN_HEIGHT, true)?;
    let mut debug_window = Display::new("Tile Map", 128, 192, true)?;

    let test_rom = Rom::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?;

    let mut emulator = Emulator::new(vec![DebugFlags::ShowTileMap], Some(&mut debug_window));

    emulator.load_rom(test_rom)?;

    // Game Boy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    emulator_window.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    emulator_window.set_background(WHITE);
    while emulator_window.is_open() {
        let frame = match emulator.update() {
            Ok(frame) => {
                frame
            }
            Err(e) => {
                println!("{}", e);
                return Ok(());
            }
        };
        emulator.update_debug_view();
        emulator_window.clear();
        emulator_window.set_buffer(frame);
        emulator_window.render()?;

    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcodes() {
        let mut emulator = Emulator::new(vec![], None);
        assert!(emulator._run_opcode_tests().unwrap());
    }
}
