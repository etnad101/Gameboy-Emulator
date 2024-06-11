mod opcodes;
mod registers;

use chrono::{DateTime, Local};

use crate::cpu::opcodes::Register;
use crate::cpu::registers::Registers;
use core::borrow;
use std::{collections::HashMap, fs, ops::Add};
use opcodes::{AddressingMode, Opcode};

const MEM_SIZE: usize = 0xFFFF;

const MAX_CYCLES: usize = 69905;
const CYCLES_PER_SCANLINE: usize = 456 / 4;

const DEBUG: bool = true;

trait GetBit {
    fn get_bit(&self, bit: u8) -> u8;
}

impl GetBit for u8 {
    fn get_bit(&self, bit: u8) -> u8 {
        (self & (1 << bit)) >> bit
    }
}

enum LCDRegister {
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

enum JumpCondition {
    Z,
    NZ,
    C,
    NC,
    None,
}

enum StoreLoadModifier {
    IncHL,
    DecHL,
    None,
}

enum DataType {
    Address(u16),
    ValueU8(u8),
    ValueU16(u16),
    ValueI8(i8),
    None,
}

pub struct MemoryBus {
    memory: [u8; MEM_SIZE + 1],
}

impl MemoryBus {
    pub fn new() -> Self {
        let path = "./DMG_ROM.bin";
        let boot_rom = std::fs::read(path).unwrap();

        let mut memory = [0; MEM_SIZE + 1];
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

    // Debugging methods

    fn dump_mem(&mut self) {
        self.debug_log
            .push_str("\nMEMORY DUMP\n------------------------------------");
        self.debug_log
            .push_str("\n16KiB ROM Bank 00 | BOOT ROM $0000 - $00FF");
        for i in 0..=MEM_SIZE {
            if i == 0x4000 {
                self.debug_log.push_str("\n16 KiB ROM Bank 01-NN");
            }
            if i == 0x8000 {
                self.debug_log.push_str("\nVRAM");
            }
            if i == 0xA000 {
                self.debug_log.push_str("\n8 KiB external RAM")
            }
            if i == 0xC000 {
                self.debug_log.push_str("\n4 KiB WRAM")
            }
            if i == 0xD000 {
                self.debug_log.push_str("\n4 KiB WRAM")
            }
            if i == 0xE000 {
                self.debug_log.push_str("\nEcho RAM")
            }
            if i == 0xFE00 {
                self.debug_log.push_str("\nObject attribute memory (OAM)")
            }
            if i == 0xFEA0 {
                self.debug_log.push_str("\n NOT USEABLE")
            }
            if i == 0xFF00 {
                self.debug_log.push_str("\nI/O Registers");
            }
            if i == 0xFF80 {
                self.debug_log.push_str("\nHigh RAM / HRAM")
            }

            if i % 32 == 0 {
                self.debug_log.push_str(&format!("\n|{:#06x}| ", i));
            } else if i % 16 == 0 {
                self.debug_log.push_str(&format!("|{:#06x}| ", i));
            } else if i % 8 == 0 {
                self.debug_log.push_str(" ");
            }

            let byte = self.memory.read_u8(i as u16);
            self.debug_log.push_str(&format!("{:02x} ", byte));
        }
    }

    fn log_debug_info(&mut self, asm: String) {
        self.debug_log.push_str(&format!("{asm}"));
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
            .push_str(&format!("\nProgram Counter: {:#04x}; ", self.pc));
    }

    pub fn crash(&mut self, msg: String) -> ! {
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
        panic!("CRASHING: {}", String::from(msg));
    }

    // Utility methods

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
                Register::AF => DataType::ValueU16(self.reg.af()),
                Register::BC => DataType::ValueU16(self.reg.bc()),
                Register::DE => DataType::ValueU16(self.reg.de()),
                Register::HL => DataType::ValueU16(self.reg.hl()),
                Register::SP => DataType::ValueU16(self.sp),
            },
            AddressingMode::AddressRegister(register) => match register {
                Register::BC => DataType::Address(self.reg.bc()),
                Register::DE => DataType::Address(self.reg.de()),
                Register::HL => DataType::Address(self.reg.hl()),
                _ => todo!("Address_Register not implemented"),
            },
            AddressingMode::ImmediateU8 => DataType::ValueU8(self.memory.read_u8(self.pc + 1)),
            AddressingMode::AddressHRAM => {
                let hi: u16 = 0xFF << 8;
                let lo: u16 = self.memory.read_u8(self.pc + 1) as u16;
                let addr = hi | lo;
                DataType::Address(addr)
            }
            AddressingMode::ImmediateI8 => {
                DataType::ValueI8(self.memory.read_u8(self.pc + 1) as i8)
            }
            AddressingMode::ImmediateU16 => DataType::ValueU16(self.memory.read_u16(self.pc + 1)),
            AddressingMode::AddressU16 => DataType::Address(self.memory.read_u16(self.pc + 1)),
            AddressingMode::IoAdressOffset => DataType::Address(0xFF00 + self.reg.c as u16),
            AddressingMode::None => DataType::None,
        }
    }

    pub fn push_stack(&mut self, value: u16) {
        let hi = ((value & 0xFF00) >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.memory.write_u8(self.sp, hi);
        self.sp -= 1;
        self.memory.write_u8(self.sp, lo);
        self.sp -= 1;
    }

    pub fn pop_stack(&mut self) -> u16 {
        self.sp += 1;
        let lo = self.memory.read_u8(self.sp);
        self.memory.write_u8(self.sp, 0);
        self.sp += 1;
        let hi = self.memory.read_u8(self.sp);
        self.memory.write_u8(self.sp, 0);
        ((hi as u16) << 8) | lo as u16
    }

    // Opcode methods

    fn load_or_store_value(
        &mut self,
        lhs: &AddressingMode,
        rhs: &AddressingMode,
        modifier: StoreLoadModifier,
    ) {
        let data = self.get_data(rhs);

        match lhs {
            AddressingMode::ImmediateRegister(reg) => match data {
                DataType::ValueU8(value) => match reg {
                    Register::A => self.reg.a = value,
                    Register::B => self.reg.b = value,
                    Register::C => self.reg.c = value,
                    Register::D => self.reg.d = value,
                    Register::E => self.reg.e = value,
                    Register::H => self.reg.h = value,
                    Register::L => self.reg.l = value,
                    _ => self.crash("Must store u8 value in u8 register".to_string()),
                },
                DataType::ValueU16(value) => match reg {
                    Register::BC => self.reg.set_bc(value),
                    Register::DE => self.reg.set_de(value),
                    Register::HL => self.reg.set_hl(value),
                    Register::SP => self.sp = value,
                    _ => self.crash("Must store u16 value in u16 register".to_string()),
                },
                DataType::Address(addr) => {
                    let value = self.memory.read_u8(addr);
                    match reg {
                        Register::A => self.reg.a = value,
                        Register::B => self.reg.b = value,
                        Register::C => self.reg.c = value,
                        Register::D => self.reg.d = value,
                        Register::E => self.reg.e = value,
                        Register::H => self.reg.h = value,
                        Register::L => self.reg.l = value,
                        _ => self.crash("Must store u8 value in u8 register".to_string()),
                    }
                }
                _ => self.crash("Should not have None here".to_string()),
            },
            AddressingMode::AddressRegister(reg) => {
                let addr = match reg {
                    Register::BC => self.reg.bc(),
                    Register::DE => self.reg.de(),
                    Register::HL => self.reg.hl(),
                    _ => self.crash("Address can't come from 8 bit registere".to_string()),
                };

                match data {
                    DataType::ValueU8(value) => self.memory.write_u8(addr, value),
                    _ => self.crash(
                        "Should only write u8 to mem / not implemented - check docs".to_string(),
                    ),
                }
            }
            AddressingMode::AddressU16
            | AddressingMode::IoAdressOffset
            | AddressingMode::AddressHRAM => {
                let addr = match self.get_data(lhs) {
                    DataType::Address(addr) => addr,
                    _ => self.crash("Should only have address here".to_string()),
                };

                match data {
                    DataType::ValueU8(value) => self.memory.write_u8(addr, value),
                    _ => self.crash("Should only have u8 value here".to_string()),
                }
            }
            _ => self.crash("Should only be an address or value".to_string()),
        }

        match modifier {
            StoreLoadModifier::DecHL => self.reg.set_hl(self.reg.hl() - 1),
            StoreLoadModifier::IncHL => self.reg.set_hl(self.reg.hl() + 1),
            StoreLoadModifier::None => (),
        }
    }

    fn carry_track_add_u8(&self, lhs: u8, rhs: u8) -> (u8, [bool; 8]) {
        let sum = lhs.wrapping_add(rhs);
        let bit_check = lhs & rhs;
        let mut tracked_bits = [false; 8];
        for i in 0..8 {
            let bit = bit_check & (1 << i);
            if bit > 0 {
                tracked_bits[i] = true;
            }
        }
        (sum, tracked_bits)
    }

    fn borrow_track_sub_u8(&self, lhs: u8, rhs: u8) -> (u8, [bool; 8]) {
        let sum = lhs.wrapping_sub(rhs);
        let mut tracked_bits = [false; 8];
        for i in 0..7 {
            let bit = lhs & (1 << i);
            if bit == 0 {
                tracked_bits[i + 1] = true;
            }
        }
        (sum, tracked_bits)
    }

    fn increment_u8(&mut self, addressing_mode: &AddressingMode) {
        let (sum, overflow) = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => self.carry_track_add_u8(val, 1),
            _ => self.crash("Expected u8 here".to_string()),
        };

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::A => self.reg.a = sum,
                Register::B => self.reg.b = sum,
                Register::C => self.reg.c = sum,
                Register::D => self.reg.d = sum,
                Register::E => self.reg.e = sum,
                Register::H => self.reg.h = sum,
                Register::L => self.reg.l = sum,
                _ => self.crash("expected 8 bit register".to_string()),
            },
            AddressingMode::AddressRegister(reg) => match reg {
                Register::HL => self.memory.write_u8(self.reg.hl(), sum),
                _ => self.crash("Should only have [HL] here".to_string()),
            },
            _ => self.crash("Only use this fucntion for u8 values".to_string()),
        };

        if sum == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        if overflow[3] {
            self.reg.set_h_flag()
        } else {
            self.reg.clear_h_flag()
        }
    }

    fn increment_u16(&mut self, addressing_mode: &AddressingMode) {
        let sum = match self.get_data(addressing_mode) {
            DataType::ValueU16(val) => val.wrapping_add(1),
            _ => self.crash("Expected u16 here".to_string()),
        };

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::BC => self.reg.set_bc(sum),
                Register::DE => self.reg.set_de(sum),
                Register::HL => self.reg.set_hl(sum),
                _ => self.crash("Expected 16 bit register".to_string()),
            },
            _ => self.crash("Expected 16 bit register".to_string()),
        }
    }

    fn decrement_u8(&mut self, addressing_mode: &AddressingMode) {
        let (diff, borrow) = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => self.borrow_track_sub_u8(val, 1),
            _ => self.crash("Expected u8 here".to_string()),
        };

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::A => self.reg.a = diff,
                Register::B => self.reg.b = diff,
                Register::C => self.reg.c = diff,
                Register::D => self.reg.d = diff,
                Register::E => self.reg.e = diff,
                Register::H => self.reg.h = diff,
                Register::L => self.reg.l = diff,
                _ => todo!(),
            },

            AddressingMode::AddressRegister(reg) => match reg {
                Register::HL => self.memory.write_u8(self.reg.hl(), diff),
                _ => self.crash("Should only have [HL] here".to_string()),
            },
            _ => self.crash("Only use this fucntion for u8 values".to_string()),
        }

        if diff == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        if borrow[4] {
            self.reg.set_h_flag()
        } else {
            self.reg.clear_h_flag()
        }
    }

    fn xor_with_a(&mut self, rhs: &AddressingMode) {
        let res = match self.get_data(rhs) {
            DataType::ValueU8(val) => val ^ self.reg.a,
            DataType::Address(addr) => {
                let val = self.memory.read_u8(addr);
                val ^ self.reg.a
            }
            _ => self.crash(format!("Should only xor with 8 bit register or HL address")),
        };

        if res == 0 {
            self.reg.set_z_flag()
        }
    }

    fn reljump(&mut self, addressing_mode: &AddressingMode, condition: JumpCondition) -> u32 {
        let offset = match self.get_data(addressing_mode) {
            DataType::ValueI8(val) => val,
            _ => self.crash(format!("Should only be i8")),
        };

        let mut jump = false;
        let extra_cycles = match condition {
            JumpCondition::Z => {
                if self.reg.get_z_flag() != 0 {
                    jump = true
                };
                1
            }
            JumpCondition::NZ => {
                if self.reg.get_z_flag() == 0 {
                    jump = true
                };
                1
            }
            JumpCondition::C => {
                if self.reg.get_c_flag() != 0 {
                    jump = true
                };
                1
            }
            JumpCondition::NC => {
                if self.reg.get_c_flag() == 0 {
                    jump = true
                };
                1
            }
            JumpCondition::None => {
                jump = true;
                0
            }
        };

        if jump {
            let res: i16 = (self.pc as i16) + offset as i16;
            self.pc = res as u16;
        }

        extra_cycles
    }

    fn call(&mut self, addressing_mode: &AddressingMode) {
        let addr = match self.get_data(addressing_mode) {
            DataType::Address(addr) => addr,
            _ => self.crash("Should only have an address here".to_string()),
        };

        self.push_stack(self.pc);
        self.pc = addr;
    }

    fn ret(&mut self) {
        // add 3 to account for call instruction size
        self.pc = self.pop_stack() + 3;
    }

    fn push_stack_instr(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU16(value) => value,
            _ => self.crash("Only expected u16 value here".to_string()),
        };

        self.push_stack(value);
    }

    fn pop_stack_instr(&mut self, addressing_mode: &AddressingMode) {
        let value = self.pop_stack();

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::AF => self.reg.set_af(value),
                Register::BC => self.reg.set_af(value),
                Register::DE => self.reg.set_af(value),
                Register::HL => self.reg.set_af(value),
                _ => self.crash("Can only pop stack to 16 bit register".to_string()),
            },
            _ => self.crash("Can only pop stack to 16 bit register".to_string()),
        }
    }

    fn bit_check(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let byte = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            _ => self.crash(format!("bit check not yet implemented or dosent exist")),
        };

        if byte.get_bit(bit) == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }
        self.reg.clear_n_flag();
        self.reg.set_h_flag();
    }

    fn rotate_left_through_carry(&mut self, addressing_mode: &AddressingMode, update_z_flag: bool) {
        let data = match self.get_data(addressing_mode) {
            DataType::ValueU8(value) => value,
            DataType::Address(addr) => self.memory.read_u8(addr),
            _ => self.crash("Expected u8 value here".to_string()),
        };

        let new_bit_0 = self.reg.get_c_flag();
        let shifted_out_bit = (data & (1 << 7)) >> 7;

        if shifted_out_bit == 1 {
            self.reg.set_c_flag();
            self.reg.clear_z_flag();
        } else {
            self.reg.clear_c_flag();
            if update_z_flag {
                self.reg.set_z_flag();
            } else {
                self.reg.clear_z_flag();
            }
        }

        let new_val = (data << 1) | new_bit_0;

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::A => self.reg.a = new_val,
                Register::B => self.reg.a = new_val,
                Register::C => self.reg.a = new_val,
                Register::D => self.reg.a = new_val,
                Register::E => self.reg.a = new_val,
                Register::H => self.reg.a = new_val,
                Register::L => self.reg.a = new_val,
                _ => self.crash("Should only rotate 8 bit values".to_string()),
            },
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => self.crash("Expected addr value here".to_string()),
                };

                self.memory.write_u8(addr, new_val);
            }
            _ => self.crash("Should only have r8 or address register".to_string()),
        }
    }

    fn sub_a(&mut self, addressing_mode: &AddressingMode, store_result: bool) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.memory.read_u8(addr),
            _ => self.crash("Should only have u8 value".to_string()),
        };

        let (diff, borrow) = self.borrow_track_sub_u8(self.reg.a, value);

        if diff == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.set_n_flag();

        if borrow[4] {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }

        if value > self.reg.a {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        if store_result {
            self.reg.a = diff
        }
    }

    // Execution methods
    fn execute_next_opcode(&mut self) -> u32 {
        // Get next instruction
        let mut code = self.memory.read_u8(self.pc);
        let prefixed = code == 0xcb;

        let (opcode_asm, opcode_bytes, opcode_cycles, lhs, rhs) = {
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
                        self.crash(format!("Prefixed Opocde {:#04x} not in opcode map", code))
                    } else {
                        self.crash(format!("Normal Opocde {:#04x} not in opcode map", code))
                    }
                }
            };
            (
                opcode.asm.to_owned(),
                opcode.bytes as u16,
                opcode.cycles as u32,
                opcode.lhs.clone(),
                opcode.rhs.clone(),
            )
        };

        // Execute instruction
        let mut skip_pc_increase = false;
        let mut extra_cycles = 0;

        if prefixed {
            code = self.memory.read_u8(self.pc + 1);
            match code {
                0x11 => self.rotate_left_through_carry(&lhs, true),
                0x7c => self.bit_check(7, &rhs),
                _ => self.crash(format!("Prefixed code {:#04x} not implemented", code)),
            }
        } else {
            match code {
                0x04 | 0x05 | 0x0d | 0x15 | 0x1d | 0x3d => self.decrement_u8(&lhs),
                0x0c | 0x24 => self.increment_u8(&lhs),
                0x13 | 0x23 => self.increment_u16(&lhs),
                0x06 | 0x0e | 0x11 | 0x1a | 0x1e | 0x21 | 0x2e | 0x31 | 0x3e | 0x4f | 0x57
                | 0x67 | 0x77 | 0x7b | 0x7c | 0xe0 | 0xe2 | 0xea | 0xf0 => {
                    self.load_or_store_value(&lhs, &rhs, StoreLoadModifier::None)
                }
                0x22 => self.load_or_store_value(&lhs, &rhs, StoreLoadModifier::IncHL),
                0x32 => self.load_or_store_value(&lhs, &rhs, StoreLoadModifier::DecHL),
                0x17 => self.rotate_left_through_carry(&lhs, false),
                0x18 => extra_cycles = self.reljump(&rhs, JumpCondition::None),
                0x20 => extra_cycles = self.reljump(&rhs, JumpCondition::NZ),
                0x28 => extra_cycles = self.reljump(&rhs, JumpCondition::Z),
                0xc1 => self.pop_stack_instr(&lhs),
                0xc5 => self.push_stack_instr(&lhs),
                0xc9 => {
                    skip_pc_increase = true;
                    self.ret();
                }
                0xcd => {
                    skip_pc_increase = true;
                    self.call(&lhs);
                }
                0x90 => self.sub_a(&rhs, true),
                0xaf => self.xor_with_a(&rhs),
                0xfe => self.sub_a(&rhs, false),
                _ => {
                    println!("Unknown opcode: {:#04x}", code);
                    println!("PC: {:#06x}", self.pc);
                    println!("SP: {:#06x}", self.sp);
                    self.crash(String::new())
                }
            };
        }

        if !skip_pc_increase {
            self.pc += opcode_bytes;
        }

        if DEBUG {
            self.log_debug_info(opcode_asm);
        }

        opcode_cycles + extra_cycles
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
            let cycles = self.execute_next_opcode();
            
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_shift() {
        let num: u8 = 0b1100_0000;
        let c = 0;
        let new_bit_0 = c;
        let c = (num & (1 << 7)) >> 7;

        let new_val = (num << 1) | new_bit_0;

        assert_eq!(new_val, 0b1000_0000);
        assert_eq!(c, 1);
    }

    #[test]
    fn test_get_bit() {
        let num: u8 = 0b1100_0010;
        assert_eq!(num.get_bit(0), 0);
        assert_eq!(num.get_bit(1), 1);
        assert_eq!(num.get_bit(2), 0);
        assert_eq!(num.get_bit(3), 0);
        assert_eq!(num.get_bit(4), 0);
        assert_eq!(num.get_bit(5), 0);
        assert_eq!(num.get_bit(6), 1);
        assert_eq!(num.get_bit(7), 1);

    }
}
