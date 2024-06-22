pub mod rom;
mod cpu;
mod ppu;
mod memory;
mod errors;

use cpu::Cpu;
use rom::Rom;
use ppu::Ppu;
use memory::MemoryBus;
use errors::EmulatorError;

use crate::drivers::display::Color;

const CPU_FREQ: usize = 4_194_300; // Acutal frequency is 4,194,304 hz
const MEM_SIZE: usize = 0xFFFF;
const MAX_CYCLES_PER_FRAME: usize = CPU_FREQ / 60; // Divide frequency by frame rate
const DIV_FREQ: usize = 16380; // Actual rate is 16,384 hz
const DIV_UPDATE_FREQ: usize = CPU_FREQ / DIV_FREQ;

pub enum LCDRegister {
    LCDC = 0xFF40,
    STAT = 0xff41,
    SCY = 0xff42,
    SCX = 0xff43,
    LY = 0xff44,
    LYC = 0xff45,
    DMA = 0xff46,
    BGP = 0xff47,
    OBP0 = 0xff48,
    OBP1 = 0xff49,
}

enum Timer {
    DIV = 0xFF04,
    TIMA = 0xFF05,
    TMA = 0xFF06,
    TAC = 0xFF07,
}


pub struct Emulator {
    cpu: Cpu,
    ppu: Ppu,
    memory: MemoryBus,
    timer_cycles: u32,
    frames: usize,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            memory: MemoryBus::new(MEM_SIZE),
            timer_cycles: 0,
            frames: 0,
        }
    }

    pub fn load_rom(&mut self, rom: Rom) -> Result<(), EmulatorError>{
        if rom.gb_compatible() {
            self.memory.load_rom(rom.bytes());
            return Ok(())
        } else {
            return Err(EmulatorError::IncompatableRom)
        }
    }

    fn update_timers(&mut self, cycles: u32) {
        self.timer_cycles += cycles;
        if self.timer_cycles as usize >= DIV_UPDATE_FREQ {
            let addr = Timer::DIV as u16;
            let div = self.memory.read_u8(addr as u16);
            self.memory.write_u8(addr, div);
            self.timer_cycles = 0;
        }
    }


    fn do_interupts(&self) {
        todo!()
    }


    pub fn update(&mut self) -> Vec<Color> {
        if self.frames > 120 {
            self.cpu.crash(&self.memory, "Intentional crash after 2 seconds".to_string());
        }
        self.frames += 1;
        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES_PER_FRAME as u32 {
            let cycles = self.cpu.execute_next_opcode(&mut self.memory);
            
            cycles_this_frame += cycles;

            // self.update_timers(cycles);

            self.ppu.update_graphics(&mut self.memory, cycles_this_frame);

            // self.do_interupts();
        }

        // Temporarily render at the end of every frame for simplicity, implement pixel FIFO later
        // Move code out of this function and into update_graphics later
        self.ppu.render_screen(&mut self.memory)
    }
    
}