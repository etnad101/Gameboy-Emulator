mod drivers;

use std::error::Error;

use drivers::display::Display;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() -> Result<(), Box<dyn Error>> {

    let mut display = Display::new(WIDTH, HEIGHT)?;

    while display.is_open() {
        display.update()?;
    }

    Ok(())
}
