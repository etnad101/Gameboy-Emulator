/*
* TODO
* Add Memory bank switching(change how mem is stored)
* Add object drawing
* Clean up code
* Optimize so emulator runs faster
* Add interrupts
* Add RAM bank switching
* Implement timer
*/

mod drivers;
mod emulator;
mod utils;

use std::{error::Error, mem};

use emulator::{cartridge::Cartridge, debugger::DebugFlags, Emulator};
use simple_graphics::{
    display::{Color, Display, WHITE},
    fonts::Font,
};

type Palette = (Color, Color, Color, Color);

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

const GREEN_PALETTE: Palette = (0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F);
const GRAY_PALETTE: Palette = (0x00FFFFFF, 0x00a9a9a9, 0x00545454, 0x00000000);

fn main() -> Result<(), Box<dyn Error>> {
    // Init windows
    // let mut register_window = Display::new("Register View", 300, 300, true)?;
    // let font = Font::new("./fonts/retro-pixel-cute-mono.bdf").unwrap();
    // register_window.set_font(font);
    let mut emulator_window = Display::new("Game Boy Emulator", SCREEN_WIDTH, SCREEN_HEIGHT, true)?;
    // let mut background_map_window = Display::new("BackgroundMap", 32 * 8, 32 * 8, false)?;
    // let mut tile_window = Display::new("Tile Map", 128, 192, false)?;
    // let mut memory_window = Display::new("Memory Viewer", 256, 256, false)?;

    let _dmg_acid2_rom = Cartridge::from("./roms/tests/dmg-acid2.gb")?; // fail
    let _cpu_instrs_test_rom = Cartridge::from("./roms/tests/cpu_instrs/cpu_instrs.gb")?; // fail
    let _cpu_01 = Cartridge::from("./roms/tests/cpu_instrs/individual/01-special.gb")?; // pass
    let _cpu_02 = Cartridge::from("./roms/tests/cpu_instrs/individual/02-interrupts.gb")?; // fail
    let _cpu_03 = Cartridge::from("./roms/tests/cpu_instrs/individual/03-op sp,hl.gb")?; // pass
    let _cpu_04 = Cartridge::from("./roms/tests/cpu_instrs/individual/04-op r,imm.gb")?; // pass
    let _cpu_05 = Cartridge::from("./roms/tests/cpu_instrs/individual/05-op rp.gb")?; // pass
    let _cpu_06 = Cartridge::from("./roms/tests/cpu_instrs/individual/06-ld r,r.gb")?; // pass
    let _cpu_07 = Cartridge::from("./roms/tests/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb")?; // pass
    let _cpu_08 = Cartridge::from("./roms/tests/cpu_instrs/individual/08-misc instrs.gb")?; // pass
    let _cpu_09 = Cartridge::from("./roms/tests/cpu_instrs/individual/09-op r,r.gb")?; // pass
    let _cpu_10 = Cartridge::from("./roms/tests/cpu_instrs/individual/10-bit ops.gb")?; // pass
    let _cpu_11 = Cartridge::from("./roms/tests/cpu_instrs/individual/11-op a,(hl).gb")?; // pass
    let _instr_timing = Cartridge::from("./roms/tests/instr_timing/instr_timing.gb")?;
    let _tetris = Cartridge::from("./roms/games/tetris.gb")?;
    let _dr_mario = Cartridge::from("./roms/games/Dr. Mario (World).gb")?;
    //let _bubble_bobble = Rom::from("./roms/games/Bubble Bobble (USA, Europe).gb")?;

    let mut emulator = Emulator::new(
        GRAY_PALETTE,
        vec![
            // DebugFlags::DumpMem,
            // DebugFlags::DumpCallLog,
            // DebugFlags::ShowTileMap,
            // DebugFlags::ShowMemView,
            // DebugFlags::ShowRegisters,
        ],
        None,
        None,
        None,
        None,
    );

    emulator.load_rom(_tetris)?;

    // Game Boy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    emulator_window.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    emulator_window.set_background(WHITE);
    while emulator_window.is_open() {
        let frame_buffer = match emulator.update() {
            Ok(frame) => frame,
            Err(e) => {
                println!("{}", e);
                return Ok(());
            }
        };
        emulator.update_debug_view();
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
        let mut emulator = Emulator::new(GRAY_PALETTE, vec![], None, None, None, None);
        assert!(emulator._run_opcode_tests().unwrap());
    }
}
