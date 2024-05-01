mod drivers;
mod cpu;

use std::error::Error;
use cpu::cpu::CPU;

use drivers::display::{Display, WHITE};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() -> Result<(), Box<dyn Error>> {

    let mut display = Display::new(WIDTH, HEIGHT)?;

    let mut cpu = CPU::new();

    display.clear();
    while display.is_open() {
        cpu.update();
        display.render()?;
        display.draw_pixel(0, 0, WHITE)?;
    }

    Ok(())
}
