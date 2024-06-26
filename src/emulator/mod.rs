mod cpu;
mod errors;
mod memory;
mod ppu;
pub mod rom;
mod test;

use std::{error::Error, fs, io::Write};

use cpu::Cpu;
use errors::{CpuError, EmulatorError};
use memory::MemoryBus;
use ppu::Ppu;
use rom::Rom;
use test::{State, TestData};

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
        let mut memory = MemoryBus::new(MEM_SIZE);
        memory.load_rom(true, None).unwrap();

        Emulator {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            memory,
            timer_cycles: 0,
            frames: 0,
        }
    }

    pub fn load_rom(&mut self, rom: Rom) -> Result<(), Box<dyn Error>> {
        if rom.gb_compatible() {
            self.memory.load_rom(false, Some(rom.bytes()))?;
            return Ok(());
        } else {
            return Err(Box::new(EmulatorError::IncompatableRom));
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

    pub fn update(&mut self) -> Result<Vec<Color>, Box<dyn Error>> {
        if self.frames > 120 {
            self.cpu.crash(
                &self.memory,
                "Intentional crash after 2 seconds".to_string(),
            );
        }
        self.frames += 1;
        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES_PER_FRAME as u32 {
            let cycles = self.cpu.execute_next_opcode(&mut self.memory)?;

            cycles_this_frame += cycles;

            // self.update_timers(cycles);

            self.ppu
                .update_graphics(&mut self.memory, cycles_this_frame);

            // self.do_interupts();
        }

        // Temporarily render at the end of every frame for simplicity, implement pixel FIFO later
        // Move code out of this function and into update_graphics later
        Ok(self.ppu.render_screen(&mut self.memory))
    }

    fn load_state(&mut self, state: State) {
        /*
        println!(
            "Initial: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, f: {:#04x}, h: {:#04x}, l: {:#04x}, sp: {:#06x}, pc: {:#06x}",
            state.a,
            state.b,
            state.c,
            state.d,
            state.e,
            state.f,
            state.h,
            state.l,
            state.sp,
            state.pc - 1
        );
        */
        self.cpu.load_state(&state);
        self.memory.clear();
        for mem_state in state.ram {
            let addr = mem_state[0];
            let value = mem_state[1] as u8;
            self.memory.write_u8(addr, value)
        }
    }

    fn check_state(&self, state: State) -> bool {
        let (a, b, c, d, e, f, h, l, sp, pc) = self.cpu.get_state();
        /*
        println!(
            "Expexted: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, f: {:#04x}, h: {:#04x}, l: {:#04x}, sp: {:#06x}, pc: {:#06x}",
            state.a,
            state.b,
            state.c,
            state.d,
            state.e,
            state.f,
            state.h,
            state.l,
            state.sp,
            state.pc - 1
        );
        println!(
            "Result: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, f: {:#04x}, h: {:#04x}, l: {:#04x}, sp: {:#06x}, pc: {:#06x}",
            a, b, c, d, e, f, h, l, sp, pc
        );
        */
        let equal = a == state.a
            && b == state.b
            && c == state.c
            && d == state.d
            && e == state.e
            && f == state.f
            && h == state.h
            && l == state.l
            && sp == state.sp
            && pc == state.pc - 1;

        for mem_state in state.ram {
            let addr = mem_state[0];
            let correct_value = mem_state[1] as u8;
            let mem_value = self.memory.read_u8(addr);

            if mem_value != correct_value {
                println!("incorrect memory value");
                return false;
            }
        }
        equal
    }

    // Test Code
    pub fn run_opcode_tests(&mut self) -> Result<(), Box<dyn Error>> {
        let test_dir = fs::read_dir("./tests")?;
        for file in test_dir {
            let path = file?.path();
            // TODO: add check to make sure file is valid test
            
            let data = fs::read_to_string(path).unwrap();

            let test_data: Vec<TestData> = serde_json::from_str(&data).unwrap();
            let total_tests = test_data.len();
            let name = test_data[0].name.clone();

            let mut current_test = 0;
            let mut passed = 0;
            println!("----------");
            for test in test_data {
                current_test += 1;
                print!(
                    "\rTesting {} ({:>3}/{}) ",
                    name, current_test, total_tests
                );
                std::io::stdout().flush().unwrap();
                self.load_state(test.initial);
                match self.cpu.execute_next_opcode(&mut self.memory) {
                    Ok(_) => (),
                    Err(e) => {
                        println!("{e}");
                        break;
                    }
                }

                if self.check_state(test.after) {
                    passed += 1;
                }

            }
            println!("\n{}/{} tests passed", passed, total_tests);
        }
        Ok(())
    }
}
