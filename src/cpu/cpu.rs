use chrono::{DateTime, Local};

use crate::cpu::opcodes::Register;
use crate::cpu::registers::Registers;
use std::{
    collections::HashMap,
    fs,
};

use super::opcodes::{self, AddressingMode, Opcode};

const MEM_SIZE: usize = 0xFFFF;

const MAX_CYCLES: usize = 69905;

const DEBUG: bool = true;

enum AlterHL {
    Inc,
    Dec,
    None,
}

enum DataType {
    Address(u16),
    ValueU8(u8),
    ValueU16(u16),
}

struct MemoryBus {
    memory: [u8; MEM_SIZE],
}

impl MemoryBus {
    pub fn new() -> Self {
        let path = "./DMG_ROM.bin";
        let boot_rom = std::fs::read(path).unwrap();

        let mut memory = [0; MEM_SIZE];
        memory[0..boot_rom.len()].copy_from_slice(&boot_rom);
        MemoryBus { memory }
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        self.memory[addr as usize] = value;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.memory[(addr) as usize] as u16;
        let hi = self.memory[(addr + 1) as usize] as u16;
        (hi << 8) | lo
    }
}

pub struct CPU {
    memory: MemoryBus,
    reg: Registers,
    sp: u16,
    pc: u16,
    normal_opcodes: HashMap<u8, opcodes::Opcode>,
    prefixed_opcodes: HashMap<u8, opcodes::Opcode>,
    debug_log: String,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            memory: MemoryBus::new(),
            reg: Registers::new(),
            sp: 0,
            pc: 0,
            normal_opcodes: Opcode::generate_normal_opcode_map(),
            prefixed_opcodes: Opcode::generate_prefixed_opcode_map(),
            debug_log: String::new(),
        }
    }

    fn dump_mem(&mut self) {
        self.debug_log
            .push_str("\nMEMORY DUMP\n------------------------------------");
        self.debug_log.push_str("\nBOOT ROM");
        for i in 0..=MEM_SIZE {
            if i == 0x8000 {
                self.debug_log.push_str("\nVRAM");
            }
            if i == 0xFE00 {
                self.debug_log.push_str("\nObject attribute memory (OAM)")
            }
            if i == 0xFF00 {
                self.debug_log.push_str("\nI/O Registers");
            }

            if i % 32 == 0 {
                self.debug_log.push_str(&format!("\n|{:#06x}| ", i));
            } else if i % 8 == 0 {
                self.debug_log.push_str(" ");
            }

            let byte = self.memory.read_u8(i as u16);
            self.debug_log.push_str(&format!("{:02x} ", byte));
            
        }
    }

    fn log_debug_info(&mut self) {
        self.debug_log
            .push_str(&format!("\nStack Pointer: {:#04x}", self.sp));
        self.debug_log.push_str(&format!(
            "\nA: {:#04x}, F: {:#010b}",
            self.reg.a, self.reg.f
        ));
        self.debug_log
            .push_str(&format!("\nB: {:#04x}, C: {:#04x}", self.reg.b, self.reg.c));
        self.debug_log
            .push_str(&format!("\nD: {:#04x}, E: {:#04x}", self.reg.d, self.reg.e));
        self.debug_log
            .push_str(&format!("\nH: {:#04x}, L: {:#04x}", self.reg.h, self.reg.l));
        self.debug_log.push_str("\n");
        self.debug_log
            .push_str(&format!("\nProgram Coutner: {:#04x}", self.pc));
    }

    fn crash(&mut self, msg: String) -> ! {
        if DEBUG {
            self.dump_mem();
            let dt = Local::now();

            let native_utc = dt.naive_utc();
            let offset = dt.offset().clone();

            let now = DateTime::<Local>::from_naive_utc_and_offset(native_utc, offset).to_string();
            let log_name = "crash_log".to_string()
                + &now.replace(" ", "_").replace(":", "-").replace(".", "_");
            let path = "./logs/".to_string() + &log_name;

            fs::File::create(path.clone()).expect("unable to create file");
            fs::write(path, self.debug_log.clone()).expect("unable to write to file");
        }
        panic!("{}", String::from(msg));
    }

    fn get_data(&self, addressing_mode: &AddressingMode) -> DataType {
        match addressing_mode {
            AddressingMode::ImmediateRegister(register) => match register {
                Register::A => DataType::ValueU8(self.reg.a),
                Register::B => DataType::ValueU8(self.reg.b),
                Register::C => DataType::ValueU8(self.reg.c),
                Register::D => DataType::ValueU8(self.reg.d),
                Register::E => DataType::ValueU8(self.reg.e),
                Register::H => DataType::ValueU8(self.reg.h),
                Register::L => DataType::ValueU8(self.reg.l),
                Register::BC => DataType::ValueU16(self.reg.bc()),
                Register::DE => DataType::ValueU16(self.reg.de()),
                Register::HL => DataType::ValueU16(self.reg.hl()),
                Register::SP => DataType::ValueU16(self.sp),
            },
            AddressingMode::AddressRegister(register) => match register {
                Register::HL => DataType::Address(self.reg.hl()),
                _ => todo!("Address_Register not implemented"),
            },
            AddressingMode::ImmediateU8 => DataType::ValueU8(self.memory.read_u8(self.pc + 1)),
            AddressingMode::JoypadU8 => {
                todo!("a8 adressing mode not implemented")
            }
            AddressingMode::ImmediateI8 => todo!("Immediate_i8 not implemented"),
            AddressingMode::ImmediateU16 => DataType::ValueU16(self.memory.read_u16(self.pc + 1)),
            AddressingMode::AdressU16 => DataType::Address(self.memory.read_u16(self.pc + 1)),
            AddressingMode::IoAdressOffset => DataType::Address(0xFF00 + self.reg.c as u16),
        }
    }

    fn load_register(&mut self, addressing_mode: &AddressingMode, register: Register) {
        match self.get_data(addressing_mode) {
            DataType::ValueU16(value) => match register {
                Register::BC => self.reg.set_bc(value),
                Register::DE => self.reg.set_de(value),
                Register::HL => self.reg.set_hl(value),
                Register::SP => self.sp = value,
                _ => self.crash(format!("Must be 16 bit register for this function")),
            },
            DataType::ValueU8(value) => match register {
                Register::A => self.reg.a = value,
                Register::B => self.reg.b = value,
                Register::C => self.reg.c = value,
                Register::D => self.reg.d = value,
                Register::E => self.reg.e = value,
                Register::H => self.reg.h = value,
                Register::L => self.reg.l = value,
                _ => self.crash(format!("Must be 8 bit register for this function")),
            },
            _ => self.crash(format!("Load type not implemented")),
        }
    }

    fn store_val_from_a(&mut self, addressing_mode: &AddressingMode, alter_hl: AlterHL) {
        match self.get_data(addressing_mode) {
            DataType::Address(addr) => self.memory.write_u8(addr, self.reg.a),
            _ => self.crash("Must be address".to_string()),
        }

        match alter_hl {
            AlterHL::Dec => self.reg.set_hl(self.reg.hl() - 1),
            AlterHL::Inc => self.reg.set_hl(self.reg.hl() + 1),
            AlterHL::None => {}
        }
    }

    fn xor_with_a(&mut self, addressing_mode: &AddressingMode) {
        let res = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val ^ self.reg.a,
            DataType::Address(addr) => {
                let val = self.memory.read_u8(addr);
                val ^ self.reg.a
            }
            DataType::ValueU16(_) => self.crash(format!("Should not have u16 value")),
        };

        if res == 0 {
            self.reg.set_z_flag()
        }
    }

    fn bit_check(&mut self, addressing_mode: &AddressingMode, bit: u8) {
        let byte = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            _ => self.crash(format!("bit check not yet implemented or dosent exist")),
        };

        if (byte & (1 << bit)) == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }
        self.reg.set_h_flag();
    }

    fn reljump_zero_not_set(&mut self, addressing_mode: &AddressingMode) {
        let data = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            _ => self.crash(format!("Should only be u8")),
        };

        if !self.reg.check_z_flag() {
            let offset = data as i8;
            let temp_pc = self.pc as i16;
            let res = temp_pc + offset as i16;
            self.pc = res as u16
        }
    }

    fn execute_next_opcode(&mut self) -> u32 {
        let mut code = self.memory.read_u8(self.pc);
        let prefixed = code == 0xcb;

        let (opcode_bytes, opcode_cycles, addressing_mode) = {
            let opcode_set = if prefixed {
                code = self.memory.read_u8(self.pc + 1);
                &self.prefixed_opcodes
            } else {
                &self.normal_opcodes
            };

            let opcode = match opcode_set.get(&code) {
                Some(op) => op,
                None => {
                    if prefixed {
                        self.crash(format!("Prefixed Opocde {:#04x} not recognized", code))
                    } else {
                        self.crash(format!("Normal Opocde {:#04x} not recognized", code))
                    }
                }
            };
            (
                opcode.bytes as u16,
                opcode.cycles as u32,
                opcode.addressing_mode.clone(),
            )
        };

        if prefixed {
            code = self.memory.read_u8(self.pc + 1);
            match code {
                0x7c => self.bit_check(&addressing_mode, 7),
                _ => self.crash(format!("Prefixed code {:#04x} not implemented", code)),
            }
        } else {
            match code {
                0x0e => self.load_register(&addressing_mode, Register::C),
                0x3e => self.load_register(&addressing_mode, Register::A),
                0x21 => self.load_register(&addressing_mode, Register::HL),
                0x31 => self.load_register(&addressing_mode, Register::SP),
                0xe2 => self.store_val_from_a(&addressing_mode, AlterHL::None),
                0x20 => self.reljump_zero_not_set(&addressing_mode),
                0x32 => self.store_val_from_a(&addressing_mode, AlterHL::Dec),
                0xaf => self.xor_with_a(&addressing_mode),
                _ => {
                    println!("Unknown opcode: {:#04x}", code);
                    println!("PC: {:#06x}", self.pc);
                    println!("SP: {:#06x}", self.sp);
                    self.crash(String::new())
                }
            };
        }

        self.pc += opcode_bytes;

        if DEBUG {
            self.log_debug_info();
        }

        opcode_cycles as u32
    }

    fn update_timers(&self, cycles: u32) {
        todo!()
    }

    fn update_graphics(&self, cycles: u32) {
        todo!()
    }

    fn do_interupts(&self) {
        todo!()
    }

    fn render_screen(&self) {
        todo!()
    }

    pub fn update(&mut self) {
        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES as u32 {
            let cycles = self.execute_next_opcode();

            cycles_this_frame += cycles;

            // self.update_timers(cycles);

            // self.update_graphics(cycles);

            // self.do_interupts();
        }

        // self.render_screen();
    }
}
