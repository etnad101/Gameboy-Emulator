/*
* TODO
* Clean up code
* Optimize so emulator runs faster
* Add interrupts
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
    let font = Font::new("./fonts/retro-pixel-cute-mono.bdf").unwrap();
    register_window.set_font(font);
    let mut background_map_window = Display::new("BackgroundMap", 32 * 8, 32 * 8, true)?;
    let mut tile_window = Display::new("Tile Map", 128, 192, true)?;
    let mut emulator_window = Display::new("Game Boy Emulator", SCREEN_WIDTH, SCREEN_HEIGHT, true)?;

    let _dmg_acid2_rom = Rom::from("./roms/tests/dmg-acid2.gb")?; // fail
    let _cpu_instrs_test_rom = Rom::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?; // fail
    let _cpu_01 = Rom::from("./roms/tests/cpu_instrs/individual/01-special.gb")?; // pass
    let _cpu_02 = Rom::from("./roms/tests/cpu_instrs/individual/02-interrupts.gb")?; // fail
    let _cpu_03 = Rom::from("./roms/tests/cpu_instrs/individual/03-op sp,hl.gb")?; // pass
    let _cpu_04 = Rom::from("./roms/tests/cpu_instrs/individual/04-op r,imm.gb")?; // pass
    let _cpu_05 = Rom::from("./roms/tests/cpu_instrs/individual/05-op rp.gb")?; // pass
    let _cpu_06 = Rom::from("./roms/tests/cpu_instrs/individual/06-ld r,r.gb")?; // pass
    let _cpu_07 = Rom::from("./roms/tests/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb")?; // pass
    let _cpu_08 = Rom::from("./roms/tests/cpu_instrs/individual/08-misc instrs.gb")?; // pass
    let _cpu_09 = Rom::from("./roms/tests/cpu_instrs/individual/09-op r,r.gb")?; // pass
    let _cpu_10 = Rom::from("./roms/tests/cpu_instrs/individual/10-bit ops.gb")?; // pass
    let _cpu_11 = Rom::from("./roms/tests/cpu_instrs/individual/11-op a,(hl).gb")?; // pass
    let _instr_timing = Rom::from("./roms/tests/instr_timing/instr_timing.gb")?;
    let _tetris = Rom::from("./roms/games/tetris.gb")?;


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

    emulator.load_rom(_tetris)?;

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
