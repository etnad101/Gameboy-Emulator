mod opcodes;
mod registers;

use chrono::{DateTime, Local};

use crate::emulator::cpu::{
    opcodes::{AddressingMode, Opcode, Register},
    registers::Registers,
};
use crate::emulator::{
    CpuDebugMode,
    memory::MemoryBus
};
use crate::utils::GetBit;
use std::{collections::HashMap, fs, path::Path};

use super::{
    errors::CpuError,
    test::State,
};


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

pub struct Cpu {
    reg: Registers,
    sp: u16,
    pc: u16,
    normal_opcodes: HashMap<u8, opcodes::Opcode>,
    prefixed_opcodes: HashMap<u8, opcodes::Opcode>,
    debug_log: String,
    debug_mode: Option<CpuDebugMode>,
}

impl Cpu {
    pub fn new(debug_mode: Option<CpuDebugMode>) -> Cpu {
        Cpu {
            reg: Registers::new(),
            sp: 0,
            pc: 0,
            normal_opcodes: Opcode::generate_normal_opcode_map(),
            prefixed_opcodes: Opcode::generate_prefixed_opcode_map(),
            debug_log: String::new(),
            debug_mode,
        }
    }

    // Debugging methods

    fn dump_mem(&mut self, memory: &MemoryBus) {
        self.debug_log
            .push_str("\nMEMORY DUMP\n------------------------------------");
        self.debug_log
            .push_str("\n16KiB ROM Bank 00 | BOOT ROM $0000 - $00FF");
        for i in 0..=memory.get_size() {
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
                self.debug_log.push(' ');
            }

            let byte: u8 = memory.read_u8(i as u16);
            self.debug_log.push_str(&format!("{:02x} ", byte));
        }
    }

    fn log_debug_info(&mut self, asm: String) {
        match self.debug_mode {
            Some(CpuDebugMode::Instructions) => {
                self.debug_log.push_str(&asm.to_string());
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
            _ => (),
        }
    }

    pub fn crash(&mut self, memory: &MemoryBus, error: CpuError) -> Result<(), CpuError> {
        match self.debug_mode {
            Some(_) => {
                self.dump_mem(memory);
                let dt = Local::now();

                let native_utc = dt.naive_utc();
                let offset = *dt.offset();

                let now =
                    DateTime::<Local>::from_naive_utc_and_offset(native_utc, offset).to_string();
                let log_name = "crash_log".to_string()
                    + &now.replace(" ", "_").replace(":", "-").replace(".", "_");
                if !Path::new("./logs/").exists() {
                    fs::create_dir("./logs").expect("Unable to create log directory")
                };
                let path = "./logs/".to_string() + &log_name;

                fs::File::create(path.clone()).expect("unable to create file");
                fs::write(path, self.debug_log.clone()).expect("unable to write to file");
            }
            None => (),
        }
        Err(error)
    }

    // Utility methods

    fn get_data(&self, memory: &MemoryBus, addressing_mode: &AddressingMode) -> DataType {
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
            AddressingMode::ImmediateU8 => DataType::ValueU8(memory.read_u8(self.pc + 1)),
            AddressingMode::AddressHRAM => {
                let hi: u16 = 0xFF << 8;
                let lo: u16 = memory.read_u8(self.pc + 1) as u16;
                let addr = hi | lo;
                DataType::Address(addr)
            }
            AddressingMode::ImmediateI8 => DataType::ValueI8(memory.read_u8(self.pc + 1) as i8),
            AddressingMode::ImmediateU16 => DataType::ValueU16(memory.read_u16(self.pc + 1)),
            AddressingMode::AddressU16 => DataType::Address(memory.read_u16(self.pc + 1)),
            AddressingMode::IoAdressOffset => DataType::Address(0xFF00 + self.reg.c as u16),
            AddressingMode::None => DataType::None,
        }
    }

    pub fn push_stack(&mut self, memory: &mut MemoryBus, value: u16) {
        let hi = ((value & 0xFF00) >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        memory.write_u8(self.sp, hi);
        self.sp -= 1;
        memory.write_u8(self.sp, lo);
        self.sp -= 1;
    }

    pub fn pop_stack(&mut self, memory: &mut MemoryBus) -> u16 {
        self.sp += 1;
        let lo = memory.read_u8(self.sp);
        self.sp += 1;
        let hi = memory.read_u8(self.sp);
        ((hi as u16) << 8) | lo as u16
    }

    // Opcode methods

    fn load_or_store_value(
        &mut self,
        memory: &mut MemoryBus,
        lhs: &AddressingMode,
        rhs: &AddressingMode,
        modifier: StoreLoadModifier,
    ) -> Result<(), CpuError> {
        let data: DataType = self.get_data(memory, rhs);

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
                    _ => self.crash(memory, CpuError::OpcodeError("Must store u8 value in u8 register".to_string()))?,
                },
                DataType::ValueU16(value) => match reg {
                    Register::BC => self.reg.set_bc(value),
                    Register::DE => self.reg.set_de(value),
                    Register::HL => self.reg.set_hl(value),
                    Register::SP => self.sp = value,
                    _ => self.crash(memory, CpuError::OpcodeError("Must store u16 value in u16 register".to_string()))?,
                },
                DataType::Address(addr) => {
                    let value = memory.read_u8(addr);
                    match reg {
                        Register::A => self.reg.a = value,
                        Register::B => self.reg.b = value,
                        Register::C => self.reg.c = value,
                        Register::D => self.reg.d = value,
                        Register::E => self.reg.e = value,
                        Register::H => self.reg.h = value,
                        Register::L => self.reg.l = value,
                        _ => {
                            self.crash(memory, CpuError::OpcodeError("Must store u8 value in u8 register".to_string()))?
                        }
                    }
                }
                _ => self.crash(memory, CpuError::OpcodeError("Should not have None here".to_string()))?,
            },
            AddressingMode::AddressRegister(reg) => {
                let addr = match reg {
                    Register::BC => self.reg.bc(),
                    Register::DE => self.reg.de(),
                    Register::HL => self.reg.hl(),
                    _ => {
                        return self.crash(
                            memory,
                            CpuError::OpcodeError("Address can't come from 8 bit registere".to_string()),
                        )
                    }
                };

                match data {
                    DataType::ValueU8(value) => memory.write_u8(addr, value),
                    _ => self.crash(
                        memory,
                        CpuError::OpcodeError("Should only write u8 to mem / not implemented - check docs".to_string()),
                    )?,
                }
            }
            AddressingMode::AddressU16
            | AddressingMode::IoAdressOffset
            | AddressingMode::AddressHRAM => {
                let addr: u16 = match self.get_data(memory, lhs) {
                    DataType::Address(addr) => addr,
                    _ => return self.crash(memory, CpuError::OpcodeError("Should only have address here".to_string())),
                };

                match data {
                    DataType::ValueU8(value) => memory.write_u8(addr, value),
                    _ => self.crash(memory, CpuError::OpcodeError("Should only have u8 value here".to_string()))?,
                }
            }
            _ => self.crash(memory, CpuError::OpcodeError("Should only be an address or value".to_string()))?,
        }

        match modifier {
            StoreLoadModifier::DecHL => self.reg.set_hl(self.reg.hl() - 1),
            StoreLoadModifier::IncHL => self.reg.set_hl(self.reg.hl() + 1),
            StoreLoadModifier::None => (),
        }

        Ok(())
    }

    fn half_carry_add_u8(&self, lhs: u8, rhs: u8) -> (u8, bool) {
        let sum = lhs.wrapping_add(rhs);
        let carry = (((lhs & 0xF) + (rhs & 0xF)) & 0x10) == 0x10;
        (sum, carry)
    }

    fn half_carry_sub_u8(&self, lhs: u8, rhs: u8) -> (u8, bool) {
        let diff = lhs.wrapping_sub(rhs);
        let borrow = lhs & 0xF < (rhs) & 0xF;
        return (diff, borrow)
    }

    fn increment_u8(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let (sum, carry) = match self.get_data(memory, addressing_mode) {
            DataType::ValueU8(val) => self.half_carry_add_u8(val, 1),
            _ => return self.crash(memory, CpuError::OpcodeError("Expected u8 here".to_string())),
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
                _ => self.crash(memory, CpuError::OpcodeError("expected 8 bit register".to_string()))?,
            },
            AddressingMode::AddressRegister(reg) => match reg {
                Register::HL => memory.write_u8(self.reg.hl(), sum),
                _ => self.crash(memory, CpuError::OpcodeError("Should only have [HL] here".to_string()))?,
            },
            _ => self.crash(memory, CpuError::OpcodeError("Only use this fucntion for u8 values".to_string()))?,
        };

        if sum == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.clear_n_flag();

        if carry {
            self.reg.set_h_flag()
        } else {
            self.reg.clear_h_flag()
        }

        Ok(())
    }

    fn increment_u16(
        &mut self,
        memory: &MemoryBus,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let sum = match self.get_data(memory, addressing_mode) {
            DataType::ValueU16(val) => val.wrapping_add(1),
            _ => return self.crash(memory, CpuError::OpcodeError("Expected u16 here".to_string())),
        };

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::BC => self.reg.set_bc(sum),
                Register::DE => self.reg.set_de(sum),
                Register::HL => self.reg.set_hl(sum),
                _ => self.crash(memory, CpuError::OpcodeError("Expected 16 bit register".to_string()))?,
            },
            _ => self.crash(memory, CpuError::OpcodeError("Expected 16 bit register".to_string()))?,
        }

        Ok(())
    }

    fn decrement_u8(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let (diff, borrow) = match self.get_data(memory, addressing_mode) {
            DataType::ValueU8(val) => self.half_carry_sub_u8(val, 1),
            _ => return self.crash(memory, CpuError::OpcodeError("Expected u8 here".to_string())),
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
                Register::HL => memory.write_u8(self.reg.hl(), diff),
                _ => self.crash(memory, CpuError::OpcodeError("Should only have [HL] here".to_string()))?,
            },
            _ => self.crash(memory, CpuError::OpcodeError("Only use this fucntion for u8 values".to_string()))?,
        }

        if diff == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.set_n_flag();

        if borrow {
            self.reg.set_h_flag()
        } else {
            self.reg.clear_h_flag()
        }

        Ok(())
    }

    fn xor_with_a(&mut self, memory: &MemoryBus, rhs: &AddressingMode) -> Result<(), CpuError> {
        let res = match self.get_data(memory, rhs) {
            DataType::ValueU8(val) => self.reg.a ^ val,
            DataType::Address(addr) => {
                let val = memory.read_u8(addr);
                val ^ self.reg.a
            }
            _ => {
                return self.crash(
                    memory,
                    CpuError::OpcodeError("Should only xor with 8 bit register or HL address".to_string()),
                )
            }
        };

        self.reg.a = res;

        if res == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.clear_n_flag();
        self.reg.clear_h_flag();
        self.reg.clear_c_flag();

        Ok(())
    }

    fn reljump(
        &mut self,
        memory: &MemoryBus,
        addressing_mode: &AddressingMode,
        condition: JumpCondition,
    ) -> Result<u32, CpuError> {
        let offset = match self.get_data(memory, addressing_mode) {
            DataType::ValueI8(val) => val,
            _ => match self.crash(memory, CpuError::OpcodeError("Should only be i8".to_string())) {
                Ok(_) => panic!("This panic should not be possible to reach, if it is something went very wrong"),
                Err(e) => return Err(e)
            },
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

        Ok(extra_cycles)
    }

    fn call(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let addr = match self.get_data(memory, addressing_mode) {
            DataType::Address(addr) => addr,
            _ => return self.crash(memory, CpuError::OpcodeError("Should only have an address here".to_string())),
        };

        self.push_stack(memory, self.pc);
        self.pc = addr;

        Ok(())
    }

    fn ret(&mut self, memory: &mut MemoryBus) {
        // add 3 to account for call instruction size
        self.pc = self.pop_stack(memory) + 3;
    }

    fn push_stack_instr(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let value = match self.get_data(memory, addressing_mode) {
            DataType::ValueU16(value) => value,
            _ => return self.crash(memory, CpuError::OpcodeError("Only expected u16 value here".to_string())),
        };

        self.push_stack(memory, value);

        Ok(())
    }

    fn pop_stack_instr(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let value = self.pop_stack(memory);

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::AF => self.reg.set_af(value),
                Register::BC => self.reg.set_bc(value),
                Register::DE => self.reg.set_bc(value),
                Register::HL => self.reg.set_bc(value),
                _ => self.crash(memory, CpuError::OpcodeError("Can only pop stack to 16 bit register".to_string()))?,
            },
            _ => self.crash(memory, CpuError::OpcodeError("Can only pop stack to 16 bit register".to_string()))?,
        }

        Ok(())
    }

    fn bit_check(
        &mut self,
        memory: &MemoryBus,
        bit: u8,
        addressing_mode: &AddressingMode,
    ) -> Result<(), CpuError> {
        let byte = match self.get_data(memory, addressing_mode) {
            DataType::ValueU8(val) => val,
            _ => {
                return self.crash(
                    memory,
                    CpuError::OpcodeError("bit check not yet implemented or dosent exist".to_string()),
                )
            }
        };

        if byte.get_bit(bit) == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }
        self.reg.clear_n_flag();
        self.reg.set_h_flag();

        Ok(())
    }

    fn rotate_left_through_carry(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
        prefixed: bool,
    ) -> Result<(), CpuError> {
        let data = match self.get_data(memory, addressing_mode) {
            DataType::ValueU8(value) => value,
            DataType::Address(addr) => memory.read_u8(addr),
            _ => return self.crash(memory, CpuError::OpcodeError("Expected u8 value here".to_string())),
        };

        let new_bit_0 = self.reg.get_c_flag();
        let shifted_out_bit = (data & (1 << 7)) >> 7;

        if prefixed && shifted_out_bit == 0{
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.clear_n_flag();
        self.reg.clear_h_flag();

        if shifted_out_bit == 1 {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
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
                _ => self.crash(memory, CpuError::OpcodeError("Should only rotate 8 bit values".to_string()))?,
            },
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(memory, addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => return self.crash(memory, CpuError::OpcodeError("Expected addr value here".to_string())),
                };

                memory.write_u8(addr, new_val);
            }
            _ => self.crash(
                memory,
                CpuError::OpcodeError("Should only have r8 or address register".to_string()),
            )?,
        }

        Ok(())
    }

    fn sub_a(
        &mut self,
        memory: &mut MemoryBus,
        addressing_mode: &AddressingMode,
        store_result: bool,
    ) -> Result<(), CpuError> {
        let value = match self.get_data(memory, addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => memory.read_u8(addr),
            _ => return self.crash(memory, CpuError::OpcodeError("Should only have u8 value".to_string())),
        };

        let (diff, borrow) = self.half_carry_sub_u8(self.reg.a, value);

        if diff == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.set_n_flag();

        if borrow {
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

        Ok(())
    }

    // Execution methods
    pub fn execute_next_opcode(&mut self, memory: &mut MemoryBus) -> Result<u32, CpuError> {
        // Get next instruction
        let mut code = memory.read_u8(self.pc);
        let prefixed = code == 0xcb;

        let (opcode_asm, opcode_bytes, opcode_cycles, lhs, rhs) = {
            let opcode_set = if prefixed {
                code = memory.read_u8(self.pc + 1);
                &self.prefixed_opcodes
            } else {
                &self.normal_opcodes
            };

            let opcode = match opcode_set.get(&code) {
                Some(op) => op,
                None => {
                    if prefixed {
                        match self.crash(
                            memory,
                            CpuError::UnrecognizedOpcode(code, true),
                        ) {
                            Ok(_) => panic!("This panic should not be possible to reach, if it is something went very wrong"),
                            Err(e) => return Err(e)
                        }
                    } else {
                        match self.crash(
                            memory,
                            CpuError::UnrecognizedOpcode(code, false),
                        ) {
                            Ok(_) => panic!("This panic should not be possible to reach, if it is something went very wrong"),
                            Err(e) => return Err(e)
                        }
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
        let mut extra_cycles: u32 = 0;

        if prefixed {
            code = memory.read_u8(self.pc + 1);
            match code {
                0x11 => self.rotate_left_through_carry(memory, &lhs, true)?,
                0x7c => self.bit_check(memory, 7, &rhs)?,
                _ => self.crash(
                    memory,
                    CpuError::OpcodeNotImplemented(code, true),
                )?,
            };
        } else {
            match code {
                0x00 => (),
                0x05 | 0x0d | 0x15 | 0x1d | 0x3d => self.decrement_u8(memory, &lhs)?,
                0x04 | 0x0c | 0x24 => self.increment_u8(memory, &lhs)?,
                0x13 | 0x23 => self.increment_u16(memory, &lhs)?,
                0x06 | 0x0e | 0x11 | 0x1a | 0x1e | 0x21 | 0x2e | 0x31 | 0x3e | 0x4f | 0x57
                | 0x67 | 0x77 | 0x7b | 0x7c | 0xe0 | 0xe2 | 0xea | 0xf0 => {
                    self.load_or_store_value(memory, &lhs, &rhs, StoreLoadModifier::None)?
                }
                0x22 => self.load_or_store_value(memory, &lhs, &rhs, StoreLoadModifier::IncHL)?,
                0x32 => self.load_or_store_value(memory, &lhs, &rhs, StoreLoadModifier::DecHL)?,
                0x17 => self.rotate_left_through_carry(memory, &lhs, false)?,
                0x18 => extra_cycles = self.reljump(memory, &rhs, JumpCondition::None)?,
                0x20 => extra_cycles = self.reljump(memory, &rhs, JumpCondition::NZ)?,
                0x28 => extra_cycles = self.reljump(memory, &rhs, JumpCondition::Z)?,
                0xc1 => self.pop_stack_instr(memory, &lhs)?,
                0xc5 => self.push_stack_instr(memory, &lhs)?,
                0xc9 => {
                    skip_pc_increase = true;
                    self.ret(memory);
                }
                0xcd => {
                    skip_pc_increase = true;
                    self.call(memory, &lhs)?;
                }
                0x90 => self.sub_a(memory, &rhs, true)?,
                0xaf => self.xor_with_a(memory, &rhs)?,
                0xbe | 0xfe => self.sub_a(memory, &rhs, false)?,
                _ => self.crash(memory, CpuError::OpcodeNotImplemented(code, false))? 
            };
        };

        if !skip_pc_increase {
            self.pc += opcode_bytes;
        }

        self.log_debug_info(opcode_asm);

        Ok(opcode_cycles + extra_cycles)
    }

    pub fn load_state(&mut self, state: &State) {
        self.reg.a = state.a;
        self.reg.b = state.b;
        self.reg.c = state.c;
        self.reg.d = state.d;
        self.reg.e = state.e;
        self.reg.f = state.f;
        self.reg.h = state.h;
        self.reg.l = state.l;
        self.sp = state.sp;
        self.pc = state.pc - 1;
    }

    pub fn get_state(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8, u16, u16) {
        (
            self.reg.a, self.reg.b, self.reg.c, self.reg.d, self.reg.e, self.reg.f, self.reg.h,
            self.reg.l, self.sp, self.pc,
        )
    }
}

