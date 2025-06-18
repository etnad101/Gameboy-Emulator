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

mod emulator;
mod gui;
mod utils;

use crate::gui::EmulatorGui;
use std::error::Error;

use emulator::{cartridge::Cartridge, Emulator};

type Color = u32;
type Palette = (u32, u32, u32, u32);

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

const GREEN_PALETTE: (u32, u32, u32, u32) = (0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F);
const GRAY_PALETTE: (u32, u32, u32, u32) = (0x00FFFFFF, 0x00a9a9a9, 0x00545454, 0x00000000);

fn main() -> Result<(), Box<dyn Error>> {
    let _dmg_acid2_rom = Cartridge::from("./roms/tests/dmg-acid2.gb")?; // fail

    let mut emulator = Emulator::new();

    emulator.load_rom(_dmg_acid2_rom)?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Game Boy Emulator",
        options,
        Box::new(|_cc| Ok(Box::new(EmulatorGui::new(emulator)))),
    )
    .unwrap();

    // Game Boy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
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
