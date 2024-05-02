use crate::cpu::opcodes::Register;
use crate::cpu::registers::Registers;
use std::collections::HashMap;

use super::opcodes::{self, AddressingMode, Opcode};

const MEM_SIZE: usize = 0xFFFF;

const MAX_CYCLES: usize = 69905;

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
        }
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
        }
    }

    fn load_r16(&mut self, addressing_mode: &AddressingMode, register: Register) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU16(val) => val,
            _ => panic!("should only have ValueU16"),
        };

        match register {
            Register::BC => self.reg.set_bc(value),
            Register::DE => self.reg.set_de(value),
            Register::HL => self.reg.set_hl(value),
            Register::SP => self.sp = value,
            _ => panic!("invalid register / not implemented"),
        }
    }

    fn store_a_dec_hl(&mut self, addressing_mode: &AddressingMode) {
        let addr = match self.get_data(addressing_mode) {
            DataType::Address(addr) => addr,
            _ => panic!("Should not have value here"),
        };

        self.memory.write_u8(addr, self.reg.a)
    }

    fn xor_with_a(&mut self, addressing_mode: &AddressingMode) {
        let res = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val ^ self.reg.a,
            DataType::Address(addr) => {
                let val = self.memory.read_u8(addr);
                val ^ self.reg.a
            }
            DataType::ValueU16(_) => panic!("Should not have u16 value"),
        };

        if res == 0 {
            self.reg.set_z()
        }
    }

    fn bit_check(&mut self, addressing_mode: &AddressingMode, bit: u8) {
        let bit = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            _ => panic!("bit check not yet implemented or dosent exist"),
        };

        if (bit & (1 << 7)) == 0 {}
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

            let opcode = opcode_set.get(&code).unwrap_or_else(|| {
                if prefixed {
                    panic!("Prefixed Opocde {:#04x} not recognized", code)
                } else {
                    panic!("Normal Opocde {:#04x} not recognized", code)
                }
            });
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
                _ => panic!("Prefixed code {:#04x} not implemented", code),
            }
        } else {
            match code {
                0x21 => self.load_r16(&addressing_mode, Register::HL),
                0x31 => self.load_r16(&addressing_mode, Register::SP),
                0x32 => self.store_a_dec_hl(&addressing_mode),
                0xaf => self.xor_with_a(&addressing_mode),
                _ => {
                    println!("Unknown opcode: {:#04x}", code);
                    println!("PC: {:#06x}", self.pc);
                    println!("SP: {:#06x}", self.sp);
                    panic!()
                }
            };
        }
        self.pc += opcode_bytes;
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
