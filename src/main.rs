/*
* TODO
* refactor everything to be more modular and represent actual architecture
* use egui
* Clean up code
* Optimize so emulator runs faster
* Add Memory bank switching(change how mem is stored)
* Add object drawing
* Add interrupts
* Add RAM bank switching
* Implement timer
*/

mod drivers;
mod emulator;
mod utils;

use std::error::Error;

use emulator::{cartridge::Cartridge, Emulator};
use simple_graphics::display::{Color, Display, WHITE};

type Palette = (Color, Color, Color, Color);

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

const GREEN_PALETTE: Palette = (0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F);
const GRAY_PALETTE: Palette = (0x00FFFFFF, 0x00a9a9a9, 0x00545454, 0x00000000);

fn main() -> Result<(), Box<dyn Error>> {
    // Init windows
    let mut emulator_window = Display::new("Game Boy Emulator", SCREEN_WIDTH, SCREEN_HEIGHT, true)?;

    let _dmg_acid2_rom = Cartridge::from("./roms/tests/dmg-acid2.gb")?; // fail

    let mut emulator = Emulator::new(
        GRAY_PALETTE,
        vec![
            // DebugFlags::DumpMem,
            // DebugFlags::DumpCallLog,
            // DebugFlags::ShowTileMap,
            // DebugFlags::ShowMemView,
            // DebugFlags::ShowRegisters,
        ],
    );

    emulator.load_rom(_dmg_acid2_rom)?;

    // Game Boy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    emulator_window.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    emulator_window.set_background(WHITE);
    while emulator_window.is_open() {
        let frame_buffer = match emulator.tick() {
            Ok(frame) => frame,
            Err(e) => {
                println!("{}", e);
                return Ok(());
            }
        };
        emulator_window.clear();
        emulator_window.set_buffer(frame_buffer);
        emulator_window.render()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcodes() {
        let mut emulator = Emulator::new(GRAY_PALETTE, vec![]);
        assert!(emulator._run_opcode_tests().unwrap());
    }
}
