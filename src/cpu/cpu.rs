use chrono::{DateTime, Local};

use crate::cpu::opcodes::Register;
use crate::cpu::registers::Registers;
use std::{collections::HashMap, fs, ops::Add};

use super::opcodes::{self, AddressingMode, Opcode};

const MEM_SIZE: usize = 0xFFFF;

const MAX_CYCLES: usize = 69905;

const DEBUG: bool = true;

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

struct MemoryBus {
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
            .push_str("\n16KiB ROM Bank 00 | BOOT ROM $0000 - $0100");
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
            AddressingMode::ImmediateI8 => DataType::ValueI8(self.memory.read_u8(self.pc + 1) as i8),
            AddressingMode::ImmediateU16 => DataType::ValueU16(self.memory.read_u16(self.pc + 1)),
            AddressingMode::AdressU16 => DataType::Address(self.memory.read_u16(self.pc + 1)),
            AddressingMode::IoAdressOffset => DataType::Address(0xFF00 + self.reg.c as u16),
            AddressingMode::None => DataType::None,
        }
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
            AddressingMode::AdressU16 => {
                let addr = match self.get_data(lhs) {
                    DataType::Address(addr) => addr,
                    _ => self.crash("Should only have address here".to_string()),
                };

                match data {
                    DataType::ValueU8(value) => self.memory.write_u8(addr, value),
                    _ => self.crash("Should only have u8 value here".to_string()),
                }
            }
            AddressingMode::IoAdressOffset => {
                let addr = match self.get_data(lhs) {
                    DataType::Address(addr) => addr,
                    _ => self.crash("Should only have IO offset address here".to_string()),
                };

                match data {
                    DataType::ValueU8(value) => self.memory.write_u8(addr, value),
                    _ => self.crash("Should only be writing u8 value to memory".to_string()),
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

    fn bit_track_add_u8(&self, lhs: u8, rhs: u8) -> (u8, [bool; 8]) {
        let sum = lhs.wrapping_add(rhs);
        let bit_check = lhs & rhs;
        let mut tracked_bits = [false; 8];
        for i in 0..8 {
            let bit = bit_check & (1 << i);
            if bit > 0 {
                tracked_bits[i] = true;
            }
        }
        (sum, [false; 8])
    }

    fn increment_r8(&mut self, register: Register) {
        let (sum, overflow) = match register {
            Register::A => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.a, 1);
                self.reg.a = sum;
                (sum, overflow)
            }
            Register::B => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.b, 1);
                self.reg.b = sum;
                (sum, overflow)
            }
            Register::C => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.c, 1);
                self.reg.c = sum;
                (sum, overflow)
            }
            Register::D => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.d, 1);
                self.reg.d = sum;
                (sum, overflow)
            }
            Register::E => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.e, 1);
                self.reg.e = sum;
                (sum, overflow)
            }
            Register::H => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.h, 1);
                self.reg.h = sum;
                (sum, overflow)
            }
            Register::L => {
                let (sum, overflow) = self.bit_track_add_u8(self.reg.l, 1);
                self.reg.l = sum;
                (sum, overflow)
            }
            _ => self.crash("expected 8 bit register".to_string()),
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

    fn bit_check(&mut self, bit: u8, addressing_mode: &AddressingMode) {
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
        let offset = match self.get_data(addressing_mode) {
            DataType::ValueI8(val) => val,
            _ => self.crash(format!("Should only be i8")),
        };

        if !self.reg.check_z_flag() {
            let temp_pc = self.pc as i16;
            let res = temp_pc + offset as i16;
            self.pc = res as u16
        }
    }

    // Execution methods

    fn execute_next_opcode(&mut self) -> u32 {
        // Get next instruction
        let mut code = self.memory.read_u8(self.pc);
        let prefixed = code == 0xcb;

        let (opcode_bytes, opcode_cycles, lhs, rhs) = {
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
                opcode.lhs.clone(),
                opcode.rhs.clone(),
            )
        };

        // Execute instruction
        if prefixed {
            code = self.memory.read_u8(self.pc + 1);
            match code {
                0x7c => self.bit_check(7, &rhs),
                _ => self.crash(format!("Prefixed code {:#04x} not implemented", code)),
            }
        } else {
            match code {
                0x0c => self.increment_r8(Register::C),
                0x0e | 0x21 | 0x31 | 0x3e | 0xe2 | 0x77 => self.load_or_store_value(&lhs, &rhs, StoreLoadModifier::None),
                0x32 => self.load_or_store_value(&lhs, &rhs, StoreLoadModifier::DecHL),
                0x20 => self.reljump_zero_not_set(&rhs),
                0xaf => self.xor_with_a(&rhs),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_track_add_u8() {}
}
