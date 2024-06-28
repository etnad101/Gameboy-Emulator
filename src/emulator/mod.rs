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
use test::TestData;

use crate::drivers::display::Color;

const CPU_FREQ: usize = 4_194_300; // Acutal frequency is 4,194,304 hz
const MEM_SIZE: usize = 0xFFFF;
const MAX_CYCLES_PER_FRAME: usize = CPU_FREQ / 60; // Divide frequency by frame rate
const DIV_FREQ: usize = 16380; // Actual rate is 16,384 hz
const DIV_UPDATE_FREQ: usize = CPU_FREQ / DIV_FREQ;

pub enum CpuDebugMode {
    Memory,
    Instructions,
}

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
    pub fn new(debug_mode: Option<CpuDebugMode>) -> Emulator {
        let mut memory = MemoryBus::new(MEM_SIZE);
        memory.load_rom(true, None).unwrap();

        Emulator {
            cpu: Cpu::new(debug_mode),
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
                CpuError::OpcodeError("Intentional crash after 2 seconds".to_string()),
            )?
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

    fn load_state(&mut self, test: &TestData) {
        self.cpu.load_state(&test.initial);
        self.memory.clear();
        for mem_state in test.initial.ram.to_owned() {
            let addr = mem_state[0];
            let value = mem_state[1] as u8;
            self.memory.write_u8(addr, value)
        }
    }

    fn check_state(&self, test: &TestData) -> bool {
        let (a, b, c, d, e, f, h, l, sp, pc) = self.cpu.get_state();
        let equal = a == test.after.a
            && b == test.after.b
            && c == test.after.c
            && d == test.after.d
            && e == test.after.e
            && f == test.after.f
            && h == test.after.h
            && l == test.after.l
            && sp == test.after.sp
            && pc == test.after.pc - 1;

        for mem_state in test.after.ram.to_owned() {
            let addr = mem_state[0];
            let correct_value = mem_state[1] as u8;
            let mem_value = self.memory.read_u8(addr);

            if mem_value != correct_value {
                if addr != 0xff04 {
                    print!(
                        "addr: {}, val: {}, expected: {}",
                        addr, mem_value, correct_value
                    );
                    return false;
                }
            }
        }

        if !equal {
            println!(
                " Initial: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, f: {:#010b}, h: {:#04x}, l: {:#04x}, sp: {:#06x}, pc: {:#06x}",
                test.initial.a,
                test.initial.b,
                test.initial.c,
                test.initial.d,
                test.initial.e,
                test.initial.f,
                test.initial.h,
                test.initial.l,
                test.initial.sp,
                test.initial.pc - 1
            );
            println!(
                "  Result: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, f: {:#010b}, h: {:#04x}, l: {:#04x}, sp: {:#06x}, pc: {:#06x}",
                a, b, c, d, e, f, h, l, sp, pc
            );
            println!(
                "Expected: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, f: {:#010b}, h: {:#04x}, l: {:#04x}, sp: {:#06x}, pc: {:#06x}",
                test.after.a,
                test.after.b,
                test.after.c,
                test.after.d,
                test.after.e,
                test.after.f,
                test.after.h,
                test.after.l,
                test.after.sp,
                test.after.pc - 1
            );
        }
        equal
    }

    // Test Code
    pub fn _run_opcode_tests(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut all_passed = true;
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
            println!("Testing {}", name);
            for test in test_data {
                current_test += 1;
                std::io::stdout().flush().unwrap();
                self.load_state(&test);
                match self.cpu.execute_next_opcode(&mut self.memory) {
                    Ok(_) => (),
                    Err(e) => {
                        println!("{e}");
                        break;
                    }
                }

                if self.check_state(&test) {
                    passed += 1;
                } else {
                    all_passed = false;
                    print!(" -> test {}\n", current_test);
                    std::io::stdout().flush()?;
                }
            }
            println!("\n{}/{} tests passed\n", passed, total_tests);
        }
        Ok(all_passed)
    }
}
