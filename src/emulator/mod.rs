mod cpu;
mod memory;

use cpu::CPU;
use memory::{LCDRegister, MemoryBus};
use crate::utils::GetBit;

const MEM_SIZE: usize = 0xFFFF;
const MAX_CYCLES: usize = 69905;
const CYCLES_PER_SCANLINE: usize = 456 / 4;

pub struct Emulator {
    cpu: CPU,
    memory: MemoryBus,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator {
            cpu: CPU::new(),
            memory: MemoryBus::new(MEM_SIZE)
        }
    }


    fn update_timers(&self, cycles: u32) {
        todo!()
    }

    fn update_graphics(&mut self, cycles: u32) {
        if (cycles as usize % CYCLES_PER_SCANLINE) == 0 {
            let mut LY = self.memory.read_u8(LCDRegister::LY as u16).wrapping_add(1);
            if LY == 154 {
                LY = 0;
            }
            self.memory.write_u8(LCDRegister::LY as u16, LY);
        }
    }

    fn do_interupts(&self) {
        todo!()
    }

    fn render_screen(&self) -> Vec<u32> {
        let mut buff = vec![0; 160 * 144];

        // get tile
        let fetcher_x = 0;
        let fetcher_y = 0;
        let lcdc = self.memory.read_u8(LCDRegister::LCDC as u16);
        // change false to chekc if x coordinate of current scanline is in window
        let tilemap_base = if (lcdc.get_bit(3) == 1) && (false) {
            0x9c00 
        } else if (lcdc.get_bit(6) == 1) && (false) {
            0x9c00 
        } else {
            0x9800 
        };

        let tilemap_addr = tilemap_base + fetcher_x;
        let tile_offset = self.memory.read_u8(tilemap_addr) as u16;

        let tile_addr = if lcdc.get_bit(4) == 1 {
           0x8000 + (tile_offset * 16)
        } else {
            let offset = (tile_offset as i8) as i32 * 16;
            (0x9000 + offset) as u16
        };

        buff
    }

    pub fn update(&mut self) -> Vec<u32> {
        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES as u32 {
            let cycles = self.cpu.execute_next_opcode(&mut self.memory);
            
            cycles_this_frame += cycles;

            // self.update_timers(cycles);

            self.update_graphics(cycles_this_frame);

            // self.do_interupts();
        }

        // Temporarily render at the end of every frame for simplicity, implement pixel FIFO later
        // Move code out of this function and into update_graphics later
        self.render_screen()
    }
    
}