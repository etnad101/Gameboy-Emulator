mod cpu;
pub mod debugger;
mod errors;
mod memory;
mod ppu;
pub mod rom;
mod test;

use std::{cell::RefCell, error::Error, fs, io::Write, rc::Rc};

use cpu::Cpu;
use debugger::{DebugFlags, Debugger};
use errors::{CpuError, EmulatorError};
use memory::MemoryBus;
use ppu::Ppu;
use rom::Rom;
use test::TestData;

use crate::Palette;
use simple_graphics::display::{Color, Display};

const MEM_SIZE: usize = 0xFFFF;
const CPU_FREQ: usize = 4_194_304; // T-cycles
const DIV_FREQ: usize = 16_384;
const MAX_CYCLES_PER_FRAME: usize = 70_224; // CPU_FREQ / FRAME_RATE
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

pub struct Emulator<'a> {
    cpu: Cpu<'a>,
    ppu: Ppu<'a>,
    memory: Rc<RefCell<MemoryBus>>,
    debugger: Rc<RefCell<Debugger<'a>>>,
    timer_cycles: usize,
    frames: usize,
    uptime_s: usize,
    paused: bool,
}

impl<'a> Emulator<'a> {
    pub fn new(
        palette: Palette,
        debug_flags: Vec<DebugFlags>,
        tile_window: Option<&'a mut Display>,
        register_window: Option<&'a mut Display>,
        background_map_window: Option<&'a mut Display>,
        memory_view_window: Option<&'a mut Display>,
    ) -> Self {
        let memory_bus = Rc::new(RefCell::new(MemoryBus::new(MEM_SIZE)));
        memory_bus.borrow_mut().load_rom(true, None).unwrap();

        let debugger = Rc::new(RefCell::new(Debugger::new(
            debug_flags,
            Rc::clone(&memory_bus),
            tile_window,
            register_window,
            background_map_window,
            memory_view_window,
            palette,
        )));

        Emulator {
            cpu: Cpu::new(Rc::clone(&memory_bus), Rc::clone(&debugger)),
            ppu: Ppu::new(Rc::clone(&memory_bus), Rc::clone(&debugger), palette),
            memory: Rc::clone(&memory_bus),
            debugger,
            timer_cycles: 0,
            frames: 0,
            uptime_s: 0,
            paused: false,
        }
    }

    pub fn load_rom(&mut self, rom: Rom) -> Result<(), Box<dyn Error>> {
        if rom.gb_compatible() {
            self.memory.borrow_mut().load_rom(false, Some(rom))?;
            Ok(())
        } else {
            Err(Box::new(EmulatorError::IncompatibleRom))
        }
    }

    fn update_timers(&mut self, cycles: usize) {
        self.timer_cycles += cycles;
        if self.timer_cycles >= DIV_UPDATE_FREQ {
            let addr = Timer::DIV as u16;
            let div = self.memory.borrow().read_u8(addr);
            self.memory.borrow_mut().write_u8(addr, div);
            self.timer_cycles = 0;
        }
    }

    fn do_interrupts(&self) {
        todo!()
    }

    pub fn update(&mut self) -> Result<Vec<Color>, Box<dyn Error>> {
        self.frames += 1;
        if self.frames >= 60 {
            self.uptime_s += 1;
            self.frames = 0;
        }

        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES_PER_FRAME {
            let cycles = self.cpu.execute_next_opcode()?;

            cycles_this_frame += cycles;

            self.update_timers(cycles);

            self.ppu.update_graphics(cycles);

            // self.do_interrupts();
        }

        Ok(self.ppu.get_frame())
    }

    pub fn update_debug_view(&mut self) {
        self.debugger.borrow_mut().render_tiles();
        self.debugger
            .borrow_mut()
            .render_register_window(self.cpu.get_registers());
        self.debugger.borrow_mut().render_background_map();
        self.debugger.borrow_mut().render_memory_viewer();
    }

    fn _load_state(&mut self, test: &TestData) {
        self.cpu._load_state(&test.initial);
        self.memory.borrow_mut().clear();
        for mem_state in test.initial.ram.iter().cloned() {
            let addr = mem_state[0];
            let value = mem_state[1] as u8;
            self.memory.borrow_mut().write_u8(addr, value)
        }
    }

    fn _check_state(&self, test: &TestData) -> bool {
        let (a, b, c, d, e, f, h, l, sp, pc) = self.cpu._get_state();
        let equal = a == test.final_name.a
            && b == test.final_name.b
            && c == test.final_name.c
            && d == test.final_name.d
            && e == test.final_name.e
            && f == test.final_name.f
            && h == test.final_name.h
            && l == test.final_name.l
            && sp == test.final_name.sp
            && pc == test.final_name.pc;

        for mem_state in test.final_name.ram.iter().cloned() {
            let addr = mem_state[0];
            let correct_value = mem_state[1] as u8;
            let mem_value = self.memory.borrow().read_u8(addr);

            if mem_value != correct_value && addr != 0xff04 {
                print!(
                    "addr: {:#06x}, val: {:#04x}, expected: {:#04x}",
                    addr, mem_value, correct_value
                );
                return false;
            }
        }

        if !equal {
            println!(
                " Initial: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, h: {:#04x}, l: {:#04x}, f: {:#010b}, sp: {:#06x}, pc: {:#06x}",
                test.initial.a,
                test.initial.b,
                test.initial.c,
                test.initial.d,
                test.initial.e,
                test.initial.h,
                test.initial.l,
                test.initial.f,
                test.initial.sp,
                test.initial.pc
            );
            println!(
                "  Result: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, h: {:#04x}, l: {:#04x}, f: {:#010b}, sp: {:#06x}, pc: {:#06x}",
                a, b, c, d, e, h, l, f, sp, pc
            );
            println!(
                "Expected: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, h: {:#04x}, l: {:#04x}, f: {:#010b}, sp: {:#06x}, pc: {:#06x}",
                test.final_name.a,
                test.final_name.b,
                test.final_name.c,
                test.final_name.d,
                test.final_name.e,
                test.final_name.h,
                test.final_name.l,
                test.final_name.f,
                test.final_name.sp,
                test.final_name.pc
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
            'inner: for test in test_data {
                current_test += 1;
                std::io::stdout().flush().unwrap();
                self._load_state(&test);
                match self.cpu.execute_next_opcode() {
                    Ok(_) => (),
                    Err(CpuError::OpcodeError(e)) => {
                        println!("{}", e);
                    }
                    Err(e) => {
                        println!("{}", e);
                        break 'inner;
                    }
                }

                if self._check_state(&test) {
                    passed += 1;
                } else {
                    all_passed = false;
                    println!(" -> test {}", current_test);
                    std::io::stdout().flush()?;
                }
            }
            println!("\n{}/{} tests passed\n", passed, total_tests);
        }
        Ok(all_passed)
    }
}
