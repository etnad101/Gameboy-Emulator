/*
* TODO
* Create Logger for CPU
* Implement LCD status registers
*/

mod cpu;
mod drivers;

use cpu::CPU;
use std::error::Error;

use drivers::display::{Display, WHITE};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() -> Result<(), Box<dyn Error>> {
    let mut display = Display::new(WIDTH, HEIGHT)?;

    let mut cpu = CPU::new();

    // Gameboy runs slightly slower than 60 Hz, one frame takes ~16.74ms instead of ~16.67ms
    display.limit_frame_rate(Some(std::time::Duration::from_micros(16740)));
    display.clear();
    let mut frame = 0;

    while display.is_open() {
        if frame == 1 {
            cpu.crash("reached set frame limit".to_string());
        }
        cpu.update();
        display.render()?;
        display.draw_pixel(0, 0, WHITE)?;
        
        frame += 1;
    }

    Ok(())
}
