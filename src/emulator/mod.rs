pub mod cartridge;
mod cpu;
pub mod debug;
mod errors;
mod memory;
mod ppu;
mod test;

use std::{cell::RefCell, error::Error, fs, io::Write, rc::Rc};

use cartridge::Cartridge;
use cpu::Cpu;
use debug::{DebugCtx, DebugFlag};
use errors::{CpuError, EmulatorError};
use memory::Bus;
use ppu::Ppu;
use test::TestData;

use crate::{utils::frame_buffer::FrameBuffer, Palette};

pub use memory::{DMGBus, RawBus};
pub use ppu::{SCREEN_WIDTH, SCREEN_HEIGHT};

const CPU_FREQ: usize = 4_194_304; // T-cycles
const DIV_FREQ: usize = 16_384;
const MAX_CYCLES_PER_FRAME: usize = 70_224; // CPU_FREQ / FRAME_RATE
const DIV_UPDATE_FREQ: usize = CPU_FREQ / DIV_FREQ;

pub enum LCDRegister {
    Lcdc,
    Stat,
    Scy,
    Scx,
    Ly,
    Lyc,
    Dma,
    Bgp,
    Obp0,
    Obp1,
}

impl From<LCDRegister> for u16 {
    fn from(val: LCDRegister) -> Self {
        match val {
            LCDRegister::Lcdc => 0xFF40,
            LCDRegister::Stat => 0xff41,
            LCDRegister::Scy => 0xff42,
            LCDRegister::Scx => 0xff43,
            LCDRegister::Ly => 0xff44,
            LCDRegister::Lyc => 0xff45,
            LCDRegister::Dma => 0xff46,
            LCDRegister::Bgp => 0xff47,
            LCDRegister::Obp0 => 0xff48,
            LCDRegister::Obp1 => 0xff49,
        }
    }
}

enum Timer {
    Div,
    Tima,
    Tma,
    Tac,
}

impl From<Timer> for u16 {
    fn from(val: Timer) -> Self {
        match val {
            Timer::Div => 0xFF04,
            Timer::Tima => 0xFF05,
            Timer::Tma => 0xFF06,
            Timer::Tac => 0xFF07,
        }
    }
}

pub struct Emulator<B: Bus> {
    cpu: Cpu<B>,
    ppu: Ppu<B>,
    memory: Rc<RefCell<B>>,
    pub debug_ctx: Rc<RefCell<DebugCtx<B>>>,
    timer_cycles: usize,
    frames: usize,
    running: bool,
}

impl Emulator<DMGBus> {
    /// Creates a new emulator instance with a DMGBus
    pub fn new() -> Self {
        let memory_bus = DMGBus::new().unwrap();
        let memory_bus = Rc::new(RefCell::new(memory_bus));

        let palette: Palette = (0xFFFFFF, 0xa9a9a9, 0x545454, 0x000000);

        let debug_ctx = Rc::new(RefCell::new(DebugCtx::new(Rc::clone(&memory_bus), palette)));

        Self {
            cpu: Cpu::new(Rc::clone(&memory_bus), Rc::clone(&debug_ctx)),
            ppu: Ppu::new(Rc::clone(&memory_bus), Rc::clone(&debug_ctx), palette),
            memory: Rc::clone(&memory_bus),
            debug_ctx,
            timer_cycles: 0,
            frames: 0,
            running: false,
        }
    }
}

impl Emulator<RawBus> {
    pub fn new() -> Self {
        let memory_bus = Rc::new(RefCell::new(RawBus::new()));

        let palette: Palette = (0xFFFFFF, 0xa9a9a9, 0x545454, 0x000000);

        let debug_ctx = Rc::new(RefCell::new(DebugCtx::new(Rc::clone(&memory_bus), palette)));

        Self {
            cpu: Cpu::new(Rc::clone(&memory_bus), Rc::clone(&debug_ctx)),
            ppu: Ppu::new(Rc::clone(&memory_bus), Rc::clone(&debug_ctx), palette),
            memory: Rc::clone(&memory_bus),
            debug_ctx,
            timer_cycles: 0,
            frames: 0,
            running: false,
        }
    }
}

impl<B: Bus> Emulator<B> {
    pub fn with_debug_flags(self, debug_flags: Vec<DebugFlag>) -> Self {
        self.debug_ctx.borrow_mut().set_flags(debug_flags);
        self
    }

    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.debug_ctx.borrow_mut().set_palette(palette);
        self.ppu.set_palette(palette);
        self
    }

    pub fn with_rom(self, rom: Cartridge) -> Result<Self, Box<dyn Error>> {
        self.load_rom(rom)?;
        Ok(self)
    }

    pub fn load_rom(&self, rom: Cartridge) -> Result<(), Box<dyn Error>> {
        println!("Loading rom: {}", rom.title());
        if rom.gb_compatible() {
            self.memory.borrow_mut().load_cartridge(rom);
            Ok(())
        } else {
            Err(Box::new(EmulatorError::IncompatibleRom))
        }
    }

    fn update_timers(&mut self, cycles: usize) {
        self.timer_cycles += cycles;
        if self.timer_cycles >= DIV_UPDATE_FREQ {
            let addr = Timer::Div.into();
            let div = self.memory.borrow().read_u8(addr);
            self.memory.borrow_mut().write_u8(addr, div.wrapping_add(1));
            self.timer_cycles = 0;
        }
    }

    /// Ticks emulator one frame
    pub fn tick(&mut self) -> Result<&FrameBuffer, Box<dyn Error>> {
        self.frames += 1;
        if self.frames >= 60 {
            self.frames = 0;
        }

        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES_PER_FRAME {
            let cycles = self.cpu.execute_next_opcode()?;
            cycles_this_frame += cycles;

            self.update_timers(cycles);
            self.ppu.update_graphics(cycles);

            if let Some(interrupt_cycles) = self.cpu.handle_interrupts() {
                cycles_this_frame += interrupt_cycles;
                self.update_timers(cycles);
                self.ppu.update_graphics(cycles);
            }
        }

        Ok(self.ppu.get_frame())
    }

    fn load_state(&mut self, test: &TestData) {
        self.cpu.load_state(&test.initial);
        self.memory.borrow_mut().clear();
        for mem_state in test.initial.ram.iter().cloned() {
            let addr = mem_state[0];
            let value = mem_state[1] as u8;
            self.memory.borrow_mut().write_u8(addr, value)
        }
    }

    fn check_state(&self, test: &TestData) -> bool {
        let (a, b, c, d, e, f, h, l, sp, pc) = self.cpu.get_state();
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
    pub fn run_opcode_tests(&mut self) -> Result<bool, Box<dyn Error>> {
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
                self.load_state(&test);
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

                if self.check_state(&test) {
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
