pub mod cartridge;
mod cpu;
pub mod debug;
mod errors;
mod memory;
mod ppu;
mod test;

#[cfg(test)]
use errors::CpuError;
#[cfg(test)]
pub use memory::RawBus;
#[cfg(test)]
use std::{fs, io::Write};
#[cfg(test)]
use test::TestCase;

use std::{cell::RefCell, error::Error, rc::Rc};

use crate::{utils::frame_buffer::FrameBuffer, Palette};
use cartridge::Cartridge;
use cpu::Cpu;
use debug::{DebugCtx, DebugFlag};
use errors::EmulatorError;
use memory::Bus;
use ppu::Ppu;

pub use memory::DMGBus;
pub use ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};

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

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum RunType {
    Paused,
    Instr,
    Frame,
}

impl std::fmt::Display for RunType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunType::Paused => write!(f, "Paused"),
            RunType::Instr => write!(f, "Instr"),
            RunType::Frame => write!(f, "Frame"),
        }
    }
}

pub struct Emulator<B: Bus> {
    cpu: Cpu<B>,
    ppu: Ppu<B>,
    memory: Rc<RefCell<B>>,
    debug_ctx: DebugCtx<B>,
    timer_cycles: usize,
    frames: usize,
    running: RunType,
    cycles_this_frame: usize,
}

impl Emulator<DMGBus> {
    /// Creates a new emulator instance with a `DMGBus`
    pub fn new() -> Self {
        let memory_bus = DMGBus::new().unwrap();
        let memory_bus = Rc::new(RefCell::new(memory_bus));

        let palette: Palette = (0xFFFFFF, 0xa9a9a9, 0x545454, 0x000000);

        let debug_ctx = DebugCtx::new(Rc::clone(&memory_bus), palette);
        let cpu = Cpu::new(Rc::clone(&memory_bus));

        Self {
            cpu,
            ppu: Ppu::new(Rc::clone(&memory_bus), palette),
            memory: Rc::clone(&memory_bus),
            debug_ctx,
            timer_cycles: 0,
            frames: 0,
            running: RunType::Paused,
            cycles_this_frame: 0,
        }
    }
}

#[cfg(test)]
impl Emulator<RawBus> {
    pub fn new() -> Self {
        let memory_bus = Rc::new(RefCell::new(RawBus::new()));

        let palette: Palette = (0xFFFFFF, 0xa9a9a9, 0x545454, 0x000000);

        let debug_ctx = DebugCtx::new(Rc::clone(&memory_bus), palette);

        Self {
            cpu: Cpu::new(Rc::clone(&memory_bus)),
            ppu: Ppu::new(Rc::clone(&memory_bus), palette),
            memory: Rc::clone(&memory_bus),
            debug_ctx,
            timer_cycles: 0,
            frames: 0,
            running: RunType::Paused,
            cycles_this_frame: 0,
        }
    }
}

impl<B: Bus> Emulator<B> {
    pub fn with_debug_flags(mut self, debug_flags: Vec<DebugFlag>) -> Self {
        self.debug_ctx.set_flags(debug_flags);
        self
    }

    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.debug_ctx.set_palette(palette);
        self.ppu.set_palette(palette);
        self
    }

    pub fn with_rom(self, rom: Cartridge) -> Result<Self, Box<dyn Error>> {
        self.load_rom(rom)?;
        Ok(self)
    }

    pub fn set_run_type(&mut self, run_type: RunType) {
        self.running = run_type;
    }

    pub fn run_type(&self) -> RunType {
        self.running
    }

    pub fn pause(&mut self) {
        self.running = RunType::Paused;
    }

    pub fn run(&mut self) {
        self.running = RunType::Frame;
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

    pub fn update_frame_count(&mut self) {
        self.frames += 1;
        if self.frames >= 60 {
            self.frames = 0;
        }
    }

    pub fn tick_to_next_frame(&mut self) -> Result<&FrameBuffer, Box<dyn Error>> {
        while self.cycles_this_frame < MAX_CYCLES_PER_FRAME {
            self.tick_instr()?;
        }
        self.cycles_this_frame = 0;

        Ok(self.ppu.get_frame())
    }

    pub fn tick_instr(&mut self) -> Result<(), Box<dyn Error>> {
        let cycles = self.cpu.execute_next_opcode(&mut self.debug_ctx)?;
        self.cycles_this_frame += cycles;

        if self.cycles_this_frame >= MAX_CYCLES_PER_FRAME {
            self.update_frame_count();
        }

        self.update_timers(cycles);
        self.ppu.update_graphics(cycles);

        if let Some(interrupt_cycles) = self.cpu.handle_interrupts(&mut self.debug_ctx) {
            self.cycles_this_frame += interrupt_cycles;
            self.update_timers(cycles);
            self.ppu.update_graphics(cycles);
        }

        Ok(())
    }

    pub fn tick(&mut self) -> Result<&FrameBuffer, Box<dyn Error>> {
        match self.running {
            RunType::Paused => Ok(self.ppu.get_frame()),
            RunType::Frame => self.tick_to_next_frame(),
            RunType::Instr => {
                // Add logic for ticking only once
                self.tick_instr()?;
                self.running = RunType::Paused;
                Ok(self.ppu.get_frame())
            }
        }
    }

    pub fn debug_ctx(&self) -> &DebugCtx<B> {
        &self.debug_ctx
    }

    pub fn debug_ctx_mut(&mut self) -> &mut DebugCtx<B> {
        &mut self.debug_ctx
    }

    #[cfg(test)]
    fn load_test_case(&mut self, test: &TestCase) {
        use crate::emulator::cpu::state::CpuState;
        let cpu_state = CpuState {
            a: test.initial.a,
            b: test.initial.b,
            c: test.initial.c,
            d: test.initial.d,
            e: test.initial.e,
            f: test.initial.f,
            h: test.initial.h,
            l: test.initial.l,
            sp: test.initial.sp,
            pc: test.initial.pc,
            ime: false,
        };
        self.cpu.load_state(cpu_state);
        self.memory.borrow_mut().clear();
        for mem_state in test.initial.ram.iter().cloned() {
            let addr = mem_state[0];
            let value = mem_state[1] as u8;
            self.memory.borrow_mut().write_u8(addr, value);
        }
    }

    #[cfg(test)]
    fn check_test_case(&self, test: &TestCase) -> bool {
        let cpu_state = self.cpu.get_state();
        let equal = cpu_state.a == test.final_name.a
            && cpu_state.b == test.final_name.b
            && cpu_state.c == test.final_name.c
            && cpu_state.d == test.final_name.d
            && cpu_state.e == test.final_name.e
            && cpu_state.f == test.final_name.f
            && cpu_state.h == test.final_name.h
            && cpu_state.l == test.final_name.l
            && cpu_state.sp == test.final_name.sp
            && cpu_state.pc == test.final_name.pc;
        for mem_state in test.final_name.ram.iter().cloned() {
            let addr = mem_state[0];
            let correct_value = mem_state[1] as u8;
            let mem_value = self.memory.borrow().read_u8(addr);

            if mem_value != correct_value && addr != 0xff04 {
                print!("addr: {addr:#06x}, val: {mem_value:#04x}, expected: {correct_value:#04x}");
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
                "  Result: a: {:#04x}, b: {:#04x}, c: {:#04x}, d: {:#04x}, e: {:#04x}, h: {:#04x}, l: {:#04x}, f: {:#010b}, sp: {:#06x}, pc: {:#06x}", cpu_state.a, cpu_state.b, cpu_state.c, cpu_state.d, cpu_state.e, cpu_state.h, cpu_state.l, cpu_state.f, cpu_state.sp, cpu_state.pc
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

    #[cfg(test)]
    pub fn run_opcode_tests(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut all_passed = true;
        let test_dir = fs::read_dir("./tests")?;
        for file in test_dir {
            let path = file?.path();
            // TODO: add check to make sure file is valid test
            let data = fs::read_to_string(path).unwrap();

            let test_data: Vec<TestCase> = serde_json::from_str(&data).unwrap();
            let total_tests = test_data.len();
            let name = test_data[0].name.clone();

            let mut current_test = 0;
            let mut passed = 0;
            println!("----------");
            println!("Testing {name}");
            'inner: for test in test_data {
                current_test += 1;
                std::io::stdout().flush().unwrap();
                self.load_test_case(&test);
                match self.cpu.execute_next_opcode(&mut self.debug_ctx) {
                    Ok(_) => (),
                    Err(CpuError::OpcodeError(e)) => {
                        println!("{e}");
                    }
                    Err(e) => {
                        println!("{e}");
                        break 'inner;
                    }
                }

                if self.check_test_case(&test) {
                    passed += 1;
                } else {
                    all_passed = false;
                    println!(" -> test {current_test}");
                    std::io::stdout().flush()?;
                }
            }
            println!("\n{passed}/{total_tests} tests passed\n");
        }
        Ok(all_passed)
    }
}
