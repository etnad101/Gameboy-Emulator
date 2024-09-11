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

use emulator::{debugger::DebugFlags, rom::Rom, Emulator};
use simple_graphics::{
    display::{Color, Display, WHITE},
    fonts::Font,
};

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

const GREEN_PALETTE: Palette = Palette::new(0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F);
const GRAY_PALETTE: Palette = Palette::new(0x00FFFFFF, 0x00a9a9a9, 0x00545454, 0x00000000);

#[derive(Clone, Copy)]
struct Palette {
    c0: Color,
    c1: Color,
    c2: Color,
    c3: Color,
}

impl Palette {
    pub const fn new(c0: Color, c1: Color, c2: Color, c3: Color) -> Self {
        Self { c0, c1, c2, c3 }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Init windows
    let mut register_window = Display::new("Register View", 300, 300, true)?;
    let mut background_map_window = Display::new("BackgroundMap", 32 * 8, 32 * 8, true)?;
    let mut tile_window = Display::new("Tile Map", 128, 192, true)?;
    let mut emulator_window = Display::new("Game Boy Emulator", SCREEN_WIDTH, SCREEN_HEIGHT, true)?;
    let font = Font::new("./fonts/retro-pixel-cute-mono.bdf").unwrap();
    register_window.set_font(font);

    let dmg_acid2_rom = Rom::from("./roms/tests/dmg-acid2.gb")?;
    let cpu_instrs_test_rom = Rom::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?;

    let mut emulator = Emulator::new(
        GRAY_PALETTE,
        vec![
            DebugFlags::DumpMem,
            DebugFlags::DumpCallLog,
            DebugFlags::ShowTileMap,
            DebugFlags::ShowRegisters,
        ],
        Some(&mut tile_window),
        Some(&mut register_window),
        Some(&mut background_map_window),
    );

    emulator.load_rom(dmg_acid2_rom)?;

    // Game Boy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    emulator_window.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    emulator_window.set_background(WHITE);
    while emulator_window.is_open() {
        let frame = match emulator.update() {
            Ok(frame) => frame,
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
        let mut emulator = Emulator::new(GRAY_PALETTE, vec![], None, None, None);
        assert!(emulator._run_opcode_tests().unwrap());
    }
}
