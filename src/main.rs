use std::error::Error;

use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Gameboy Emulator",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut x: usize = 0;
        for pixel in buffer.iter_mut() {
            if x < (WIDTH / 3) {
                *pixel = rgb(255, 0, 0);
            } else if x < (2 * WIDTH / 3) {
                *pixel = rgb(0, 255, 0);
            } else {
                *pixel = rgb(0, 0, 255);
            }
            x += 1;
            if x >= WIDTH {
                x = 0;
            }
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }

    Ok(())
}