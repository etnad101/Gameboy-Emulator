mod opcodes;
pub(super) mod registers;

use crate::{
    emulator::{
        cpu::{
            opcodes::{AddressingMode, Opcode, Register},
            registers::Registers,
        },
        memory::MemoryBus,
    },
    utils::BitOps,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::{errors::CpuError, test::State, Debugger};
enum Direction {
    Left,
    Right,
}

enum JumpCondition {
    Z,
    NZ,
    C,
    NC,
}

enum StoreLoadModifier {
    IncHL,
    DecHL,
}

enum DataType {
    Address(u16),
    ValueU8(u8),
    ValueU16(u16),
    ValueI8(i8),
    None,
}

pub struct Cpu<'a> {
    reg: Registers,
    sp: u16,
    pc: u16,
    ime: bool,
    normal_opcodes: HashMap<u8, Opcode>,
    prefixed_opcodes: HashMap<u8, Opcode>,
    memory: Rc<RefCell<MemoryBus>>,
    debugger: Rc<RefCell<Debugger<'a>>>,
}

impl<'a> Cpu<'a> {
    pub fn new(memory: Rc<RefCell<MemoryBus>>, debugger: Rc<RefCell<Debugger<'a>>>) -> Self {
        Self {
            reg: Registers::new(),
            sp: 0,
            pc: 0,
            ime: false,
            normal_opcodes: Opcode::generate_normal_opcode_map(),
            prefixed_opcodes: Opcode::generate_prefixed_opcode_map(),
            memory,
            debugger,
        }
    }

    // Debugging methods

    pub fn get_registers(&self) -> Registers {
        self.reg.clone()
    }

    pub fn crash(&mut self, error: CpuError) -> CpuError {
        self.debugger.borrow_mut().dump_logs();
        eprintln!("{:#06x}", self.pc);
        error
    }

    // Utility methods
    fn write_mem_u8(&self, addr: u16, value: u8) {
        self.memory.borrow_mut().write_u8(addr, value);
    }

    fn read_mem_u8(&self, addr: u16) -> u8 {
        self.memory.borrow().read_u8(addr)
    }

    fn read_mem_u16(&self, addr: u16) -> u16 {
        self.memory.borrow().read_u16(addr)
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
            AddressingMode::ImmediateU8 => DataType::ValueU8(self.read_mem_u8(self.pc.wrapping_add(1))),
            AddressingMode::AddressHRAM => {
                let hi: u16 = 0xFF << 8;
                let lo: u16 = self.read_mem_u8(self.pc.wrapping_add(1)) as u16;
                let addr = hi | lo;
                DataType::Address(addr)
            }
            AddressingMode::ImmediateI8 => DataType::ValueI8(self.read_mem_u8(self.pc.wrapping_add(1)) as i8),
            AddressingMode::ImmediateU16 => DataType::ValueU16(self.read_mem_u16(self.pc.wrapping_add(1))),
            AddressingMode::AddressU16 => DataType::Address(self.read_mem_u16(self.pc.wrapping_add(1))),
            AddressingMode::IoAdressOffset => DataType::Address(0xFF00 + self.reg.c as u16),
            AddressingMode::None => DataType::None,
        }
    }

    pub fn push_stack(&mut self, value: u16) {
        let hi = ((value & 0xFF00) >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.sp -= 1;
        self.write_mem_u8(self.sp, hi);
        self.sp -= 1;
        self.write_mem_u8(self.sp, lo);
    }

    pub fn pop_stack(&mut self) -> u16 {
        let lo = self.read_mem_u8(self.sp);
        self.sp += 1;
        let hi = self.read_mem_u8(self.sp);
        self.sp += 1;
        ((hi as u16) << 8) | lo as u16
    }

    // Opcode methods

    fn load_or_store_value(
        &mut self,
        lhs: &AddressingMode,
        rhs: &AddressingMode,
        modifier: Option<StoreLoadModifier>,
    ) {
        let data: DataType = self.get_data(rhs);

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
                    _ => panic!("Must store u8 value in u8 register"),
                },
                DataType::ValueU16(value) => match reg {
                    Register::BC => self.reg.set_bc(value),
                    Register::DE => self.reg.set_de(value),
                    Register::HL => self.reg.set_hl(value),
                    Register::SP => self.sp = value,
                    _ => panic!("Must store u16 value in u16 register"),
                },
                DataType::Address(addr) => {
                    let value = self.read_mem_u8(addr);
                    match reg {
                        Register::A => self.reg.a = value,
                        Register::B => self.reg.b = value,
                        Register::C => self.reg.c = value,
                        Register::D => self.reg.d = value,
                        Register::E => self.reg.e = value,
                        Register::H => self.reg.h = value,
                        Register::L => self.reg.l = value,
                        _ => panic!("Must store u8 value in u8 register"),
                    }
                }
                _ => panic!("Should not have none or i8 here"),
            },
            AddressingMode::AddressRegister(reg) => {
                let addr = match reg {
                    Register::BC => self.reg.bc(),
                    Register::DE => self.reg.de(),
                    Register::HL => self.reg.hl(),
                    _ => panic!("address can't come from 8 bit registers"),
                };

                match data {
                    DataType::ValueU8(value) => self.write_mem_u8(addr, value),
                    _ => panic!("Should only write u8 to mem / not implemented - check docs"),
                }
            }
            AddressingMode::AddressU16
            | AddressingMode::IoAdressOffset
            | AddressingMode::AddressHRAM => {
                let addr: u16 = match self.get_data(lhs) {
                    DataType::Address(addr) => addr,
                    _ => panic!("Should only have address here"),
                };

                match data {
                    DataType::ValueU8(val) => self.write_mem_u8(addr, val),
                    DataType::ValueU16(val) => {
                        let lo = val & 0xFF;
                        let hi = val >> 8;
                        self.write_mem_u8(addr, lo as u8);
                        self.write_mem_u8(addr + 1, hi as u8);
                    }
                    _ => panic!("Should only have u8 or u16 here"),
                }
            }
            _ => panic!("Should only have an address or value"),
        }

        match modifier {
            Some(StoreLoadModifier::DecHL) => self.reg.set_hl(self.reg.hl() - 1),
            Some(StoreLoadModifier::IncHL) => self.reg.set_hl(self.reg.hl() + 1),
            None => (),
        };
    }

    fn carry_track_add_u8(&self, lhs: u8, rhs: u8) -> (u8, bool, bool) {
        let (sum, full_carry) = lhs.overflowing_add(rhs);
        let half_carry = (((lhs & 0xF) + (rhs & 0xF)) & 0x10) == 0x10;
        (sum, half_carry, full_carry)
    }

    fn carry_track_sub_u8(&self, lhs: u8, rhs: u8) -> (u8, bool, bool) {
        let (diff, full_borrow) = lhs.overflowing_sub(rhs);
        let half_borrow = lhs & 0xF < (rhs) & 0xF;
        (diff, half_borrow, full_borrow)
    }

    fn increment_u8(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Expected u8 here"),
        };

        let (sum, half_carry, _) = self.carry_track_add_u8(value, 1);

        match addressing_mode {
            AddressingMode::ImmediateRegister(Register::A) => self.reg.a = sum,
            AddressingMode::ImmediateRegister(Register::B) => self.reg.b = sum,
            AddressingMode::ImmediateRegister(Register::C) => self.reg.c = sum,
            AddressingMode::ImmediateRegister(Register::D) => self.reg.d = sum,
            AddressingMode::ImmediateRegister(Register::E) => self.reg.e = sum,
            AddressingMode::ImmediateRegister(Register::H) => self.reg.h = sum,
            AddressingMode::ImmediateRegister(Register::L) => self.reg.l = sum,
            AddressingMode::AddressRegister(Register::HL) => self.write_mem_u8(self.reg.hl(), sum),
            _ => panic!("Should not have any other addressing mode"),
        };

        if sum == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.clear_n_flag();

        if half_carry {
            self.reg.set_h_flag()
        } else {
            self.reg.clear_h_flag()
        }
    }

    fn increment_u16(&mut self, lhs: &AddressingMode) {
        let sum = match self.get_data(lhs) {
            DataType::ValueU16(val) => val.wrapping_add(1),
            _ => panic!("expected u16 here"),
        };

        match lhs {
            AddressingMode::ImmediateRegister(Register::BC) => self.reg.set_bc(sum),
            AddressingMode::ImmediateRegister(Register::DE) => self.reg.set_de(sum),
            AddressingMode::ImmediateRegister(Register::HL) => self.reg.set_hl(sum),
            AddressingMode::ImmediateRegister(Register::SP) => self.sp = sum,
            _ => panic!("expected 16 bit register"),
        }
    }

    fn decrement_u8(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("expected u8 here"),
        };

        let (diff, half_borrow, _) = self.carry_track_sub_u8(value, 1);

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

            AddressingMode::AddressRegister(Register::HL) => self.write_mem_u8(self.reg.hl(), diff),

            _ => panic!("Only use this fucntion for u8 values"),
        }

        if diff == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.set_n_flag();

        if half_borrow {
            self.reg.set_h_flag()
        } else {
            self.reg.clear_h_flag()
        }
    }

    fn decrement_u16(&mut self, addressing_mode: &AddressingMode) {
        let mut byte = match self.get_data(addressing_mode) {
            DataType::ValueU16(val) => val,
            _ => panic!("Should only have value from 16 bit register here"),
        };

        byte -= 1;

        match addressing_mode {
            AddressingMode::ImmediateRegister(Register::BC) => self.reg.set_bc(byte),
            AddressingMode::ImmediateRegister(Register::DE) => self.reg.set_de(byte),
            AddressingMode::ImmediateRegister(Register::HL) => self.reg.set_hl(byte),
            AddressingMode::ImmediateRegister(Register::SP) => self.sp = byte,
            _ => panic!("Should not have any mode code here"),
        }
    }

    fn rel_jump(
        &mut self,
        addressing_mode: &AddressingMode,
        condition: Option<JumpCondition>,
    ) -> usize {
        let offset = match self.get_data(addressing_mode) {
            DataType::ValueI8(val) => val,
            _ => panic!("Should only have i8 here"),
        };

        let mut jump = false;
        let extra_cycles: usize = match condition {
            Some(JumpCondition::Z) => {
                if self.reg.get_z_flag() != 0 {
                    jump = true
                };
                4
            }
            Some(JumpCondition::NZ) => {
                if self.reg.get_z_flag() == 0 {
                    jump = true
                };
                4
            }
            Some(JumpCondition::C) => {
                if self.reg.get_c_flag() != 0 {
                    jump = true
                };
                4
            }
            Some(JumpCondition::NC) => {
                if self.reg.get_c_flag() == 0 {
                    jump = true
                };
                4
            }
            None => {
                jump = true;
                0
            }
        };

        if jump {
            let res: i16 = (self.pc as i16).wrapping_add(offset as i16); 
            self.pc = res as u16;
        }

        extra_cycles
    }

    fn abs_jump(
        &mut self,
        addressing_mode: &AddressingMode,
        condition: Option<JumpCondition>,
    ) -> usize {
        let (jump, extra_cycles) = match condition {
            Some(JumpCondition::NZ) => {
                if self.reg.get_z_flag() == 0 {
                    (true, 4)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::NC) => {
                if self.reg.get_c_flag() == 0 {
                    (true, 4)
                } else {
                    (false, 0)
                }
            }
            None => (true, 0),
            _ => panic!("No other conditions"),
        };

        if jump {
            let addr = match self.get_data(addressing_mode) {
                DataType::Address(addr) => addr,
                _ => panic!("Should only have an address here"),
            };

            self.pc = addr;
        }

        extra_cycles
    }

    fn call(
        &mut self,
        addressing_mode: &AddressingMode,
        condition: Option<JumpCondition>,
    ) -> usize {
        let (jump, extra_cycles) = match condition {
            Some(JumpCondition::NZ) => {
                if self.reg.get_z_flag() == 0 {
                    (true, 12)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::NC) => {
                if self.reg.get_c_flag() == 0 {
                    (true, 12)
                } else {
                    (false, 0)
                }
            }
            None => (true, 0),
            _ => panic!("No other conditions"),
        };

        if jump {
            let addr = match self.get_data(addressing_mode) {
                DataType::Address(addr) => addr,
                _ => panic!("Should only have an address here"),
            };

            self.push_stack(self.pc.wrapping_add(3));
            self.pc = addr;
        }
        extra_cycles
    }

    fn ret(&mut self, condition: Option<JumpCondition>, set_ime: bool) -> usize {
        let jump = match condition {
            Some(JumpCondition::Z) => self.reg.get_z_flag() == 1,
            Some(JumpCondition::NZ) => self.reg.get_z_flag() == 0,
            Some(JumpCondition::C) => self.reg.get_c_flag() == 1,
            Some(JumpCondition::NC) => self.reg.get_c_flag() == 0,
            None => {
                self.pc = self.pop_stack();
                if set_ime {
                    self.write_mem_u8(0xFFFF, 0xFF);
                }
                return 0;
            }
        };

        if jump {
            self.pc = self.pop_stack();
            return 12;
        } else {
            return 0;
        }
    }

    fn push_stack_instr(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU16(value) => value,
            _ => panic!("Only expected u16 value here"),
        };

        self.push_stack(value);
    }

    fn pop_stack_instr(&mut self, addressing_mode: &AddressingMode) {
        let value = self.pop_stack();

        match addressing_mode {
            AddressingMode::ImmediateRegister(Register::AF) => self.reg.set_af(value),
            AddressingMode::ImmediateRegister(Register::BC) => self.reg.set_bc(value),
            AddressingMode::ImmediateRegister(Register::DE) => self.reg.set_de(value),
            AddressingMode::ImmediateRegister(Register::HL) => self.reg.set_hl(value),
            _ => panic!("Can only pop stack to 16 bit register"),
        }
    }

    fn shift(&mut self, addressing_mode: &AddressingMode, direction: Direction, logical: bool) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Should only have u8 value here"),
        };

        let (new_val, shifted_out_bit) = match direction {
            Direction::Left => (value << 1, value.get_bit(7)),
            Direction::Right => {
                if logical {
                    (value >> 1, value.get_bit(0))
                } else {
                    let sign_bit = value.get_bit(7);
                    ((value >> 1) | (sign_bit << 7), value.get_bit(0))
                }
            }
        };

        if new_val == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.clear_n_flag();
        self.reg.clear_h_flag();

        if shifted_out_bit == 1 {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::A => self.reg.a = new_val,
                Register::B => self.reg.b = new_val,
                Register::C => self.reg.c = new_val,
                Register::D => self.reg.d = new_val,
                Register::E => self.reg.e = new_val,
                Register::H => self.reg.h = new_val,
                Register::L => self.reg.l = new_val,
                _ => panic!("Should only rotate 8 bit values"),
            },
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => panic!("Expected addr value here"),
                };

                self.write_mem_u8(addr, new_val);
            }
            _ => panic!("Should only have r8 or address register"),
        }
    }

    fn rotate(
        &mut self,
        addressing_mode: &AddressingMode,
        direction: Direction,
        update_z: bool,
        through_carry: bool,
    ) {
        let data = match self.get_data(addressing_mode) {
            DataType::ValueU8(value) => value,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Expected u8 value here"),
        };

        let (shifted_out_bit, new_val) = match direction {
            Direction::Left => {
                let shifted_out_bit = data.get_bit(7);
                let new_val = if through_carry {
                    (data << 1) | self.reg.get_c_flag()
                } else {
                    (data << 1) | shifted_out_bit
                };
                (shifted_out_bit, new_val)
            }
            Direction::Right => {
                let shifted_out_bit = data.get_bit(0);
                let new_val = if through_carry {
                    (data >> 1) | (self.reg.get_c_flag() << 7)
                } else {
                    (data >> 1) | (shifted_out_bit << 7)
                };
                (shifted_out_bit, new_val)
            }
        };

        if update_z && (new_val == 0) {
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

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::A => self.reg.a = new_val,
                Register::B => self.reg.b = new_val,
                Register::C => self.reg.c = new_val,
                Register::D => self.reg.d = new_val,
                Register::E => self.reg.e = new_val,
                Register::H => self.reg.h = new_val,
                Register::L => self.reg.l = new_val,
                _ => panic!("Should only rotate 8 bit values"),
            },
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => panic!("Expected addr value here"),
                };

                self.write_mem_u8(addr, new_val);
            }
            _ => panic!("Should only have r8 or address register"),
        }
    }

    fn swap(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Expected u8 or addr here"),
        };

        let hi = value >> 4;
        let lo = value & 0x0F;

        let new_val = (lo << 4) | hi;

        if new_val == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.clear_n_flag();
        self.reg.clear_h_flag();
        self.reg.clear_c_flag();

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => match reg {
                Register::A => self.reg.a = new_val,
                Register::B => self.reg.b = new_val,
                Register::C => self.reg.c = new_val,
                Register::D => self.reg.d = new_val,
                Register::E => self.reg.e = new_val,
                Register::H => self.reg.h = new_val,
                Register::L => self.reg.l = new_val,
                _ => panic!("Should only rotate 8 bit values"),
            },
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => panic!("Expected addr value here"),
                };

                self.write_mem_u8(addr, new_val);
            }
            _ => panic!("Should only have r8 or address register"),
        }
    }

    fn set_u8_add_registers(&mut self, sum: u8, half_carry: bool, full_carry: bool) {
        if sum == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.clear_n_flag();

        if half_carry {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }

        if full_carry {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        self.reg.a = sum;
    }

    fn add_a_u8(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Should not have any other data type here"),
        };

        let (sum, half_carry, full_carry) = self.carry_track_add_u8(self.reg.a, value);

        self.set_u8_add_registers(sum, half_carry, full_carry);
    }

    fn add_hl_u16(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU16(val) => val,
            _ => panic!("Should only have a u16 value here"),
        };

        let hl = self.reg.hl();
        let (res, carry) = hl.overflowing_add(value);
        let half_carry = (((hl & 0xFFF) + (value & 0xFFF)) & 0x1000) == 0x1000;

        self.reg.set_hl(res);

        self.reg.clear_n_flag();

        if half_carry {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }

        if carry {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }
    }

    fn add_sp_e8(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueI8(val) => val as i16,
            _ => panic!("Should only have an i8 here"),
        };

        let s8 = (value&127)-(value&128);

        let before = self.sp;
        self.sp = (self.sp as i16).wrapping_add(value as i16) as u16;

        let full_carry: bool;
        let half_carry: bool;

        if value >= 0 {
            full_carry = ((before as i16 & 0xFF) + s8) > 0xFF;
            half_carry = ((before as i16 & 0xF) + (s8 & 0xF)) > 0xF;
        } else {
            full_carry = (self.sp & 0xFF) < (before & 0xFF);
            half_carry = (self.sp & 0xF) < (before & 0xF);
        }

        self.reg.clear_z_flag();
        self.reg.clear_n_flag();

        if full_carry {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        if half_carry {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }
    }

    fn ld_hl_sp_e8(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueI8(val) => val as i16,
            _ => panic!("Should only have an i8 here"),
        };

        self.reg.set_hl((self.sp as i16).wrapping_add(value) as u16);

        let full_carry = ((self.sp as i16 & 0xFF) + (value & 0xFF)) > 0xFF;
        let half_carry = ((self.sp as i16 & 0xF) + (value & 0xF)) > 0xF;

        self.reg.clear_z_flag();
        self.reg.clear_n_flag();

        if full_carry {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        if half_carry {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }
    }

    fn adc(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("expecred u8 here"),
        };

        let (sum, half_carry_1, carry_1) = self.carry_track_add_u8(self.reg.a, value);
        let (sum, half_carry_2, carry_2) = self.carry_track_add_u8(sum, self.reg.get_c_flag());

        let half_carry = half_carry_1 | half_carry_2;
        let full_carry = carry_1 | carry_2;

        self.set_u8_add_registers(sum, half_carry, full_carry);
    }

    fn sub_a(&mut self, rhs: &AddressingMode, store_result: bool) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Should only have u8"),
        };

        let (diff, half_borrow, borrow) = self.carry_track_sub_u8(self.reg.a, value);

        if diff == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.set_n_flag();

        if half_borrow {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }

        if borrow {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        if store_result {
            self.reg.a = diff
        }
    }

    fn sbc(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Should only have u8 value here"),
        };

        let (diff, half_borrow_1, borrow_1) = self.carry_track_sub_u8(self.reg.a, value);
        let (diff, half_borrow_2, borrow_2) = self.carry_track_sub_u8(diff, self.reg.get_c_flag());

        let half_borrow = half_borrow_1 | half_borrow_2;
        let borrow = borrow_1 | borrow_2;

        if diff == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.set_n_flag();

        if half_borrow {
            self.reg.set_h_flag();
        } else {
            self.reg.clear_h_flag();
        }

        if borrow {
            self.reg.set_c_flag();
        } else {
            self.reg.clear_c_flag();
        }

        self.reg.a = diff
    }

    fn and(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("Expected u8 here"),
        };

        let res = self.reg.a & value;

        if res == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }

        self.reg.clear_n_flag();
        self.reg.set_h_flag();
        self.reg.clear_c_flag();

        self.reg.a = res;
    }

    fn xor_with_a(&mut self, rhs: &AddressingMode) {
        let res = match self.get_data(rhs) {
            DataType::ValueU8(val) => self.reg.a ^ val,
            DataType::Address(addr) => {
                let val = self.read_mem_u8(addr);
                val ^ self.reg.a
            }
            _ => panic!("Should only have u8"),
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
    }

    fn or_with_a(&mut self, rhs: &AddressingMode) {
        let byte = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("no other data type should be here"),
        };

        let res = self.reg.a | byte;
        self.reg.a = res;

        if res == 0 {
            self.reg.set_z_flag()
        } else {
            self.reg.clear_z_flag()
        }

        self.reg.clear_n_flag();
        self.reg.clear_h_flag();
        self.reg.clear_c_flag();
    }

    fn check_bit(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let byte = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => panic!("bit check not yet implemented or dosent exist"),
        };

        if byte.get_bit(bit) == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }
        self.reg.clear_n_flag();
        self.reg.set_h_flag();
    }

    fn set_bit(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let (mut byte, addr) = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => (val, None),
            DataType::Address(addr) => (self.read_mem_u8(addr), Some(addr)),
            _ => panic!("Should not have any other type here"),
        };
        byte.set_bit(bit);

        match addressing_mode {
            AddressingMode::ImmediateRegister(Register::A) => self.reg.a = byte,
            AddressingMode::ImmediateRegister(Register::B) => self.reg.b = byte,
            AddressingMode::ImmediateRegister(Register::C) => self.reg.c = byte,
            AddressingMode::ImmediateRegister(Register::D) => self.reg.d = byte,
            AddressingMode::ImmediateRegister(Register::E) => self.reg.e = byte,
            AddressingMode::ImmediateRegister(Register::H) => self.reg.h = byte,
            AddressingMode::ImmediateRegister(Register::L) => self.reg.l = byte,
            AddressingMode::AddressRegister(Register::HL) => self.write_mem_u8(addr.unwrap(), byte),
            _ => panic!("should not have anything else here"),
        };
    }

    fn reset_bit(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let (mut byte, addr) = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => (val, None),
            DataType::Address(addr) => (self.read_mem_u8(addr), Some(addr)),
            _ => panic!("Should not have any other type here"),
        };
        byte.clear_bit(bit);

        match addressing_mode {
            AddressingMode::ImmediateRegister(Register::A) => self.reg.a = byte,
            AddressingMode::ImmediateRegister(Register::B) => self.reg.b = byte,
            AddressingMode::ImmediateRegister(Register::C) => self.reg.c = byte,
            AddressingMode::ImmediateRegister(Register::D) => self.reg.d = byte,
            AddressingMode::ImmediateRegister(Register::E) => self.reg.e = byte,
            AddressingMode::ImmediateRegister(Register::H) => self.reg.h = byte,
            AddressingMode::ImmediateRegister(Register::L) => self.reg.l = byte,
            AddressingMode::AddressRegister(Register::HL) => self.write_mem_u8(addr.unwrap(), byte),
            _ => panic!("should not have anything else here"),
        };
    }

    fn daa(&mut self) {
        if self.reg.get_n_flag() == 0 {
            // after an addition, adjust if (half-)carry occurred or if result is out of bounds
            if (self.reg.get_c_flag() == 1) || self.reg.a > 0x99 {
                self.reg.a = self.reg.a.wrapping_add(0x60);
                self.reg.set_c_flag()
            }
            if self.reg.get_h_flag() == 1 || (self.reg.a & 0x0f) > 0x09 {
                self.reg.a = self.reg.a.wrapping_add(0x6);
            }
        } else {
            // after a subtraction, only adjust if (half-)carry occurred
            if self.reg.get_c_flag() == 1 {
                self.reg.a = self.reg.a.wrapping_sub(0x60);
            }
            if self.reg.get_h_flag() == 1 {
                self.reg.a = self.reg.a.wrapping_sub(0x6);
            }
        }

        if self.reg.a == 0 {
            self.reg.set_z_flag();
        } else {
            self.reg.clear_z_flag();
        }
        self.reg.clear_h_flag();
    }

    fn cpl(&mut self) {
        self.reg.a = !self.reg.a;
        self.reg.set_n_flag();
        self.reg.set_h_flag();
    }

    fn scf(&mut self) {
        self.reg.clear_n_flag();
        self.reg.clear_h_flag();
        self.reg.set_c_flag();
    }

    fn ccf(&mut self) {
        self.reg.clear_n_flag();
        self.reg.clear_h_flag();

        if self.reg.get_c_flag() == 1 {
            self.reg.clear_c_flag();
        } else {
            self.reg.set_c_flag();
        }
    }

    fn reset_vec(&mut self, lhs: u8) {
        let addr: u16 = ((lhs as u16) << 8) | self.reg.h as u16;
        self.pc = addr;
    }

    pub fn execute_next_opcode(&mut self) -> Result<usize, CpuError> {
        // Get next instruction
        let mut code = self.read_mem_u8(self.pc);
        let prefixed = code == 0xcb;

        let (opcode_asm, opcode_bytes, opcode_cycles, lhs, rhs) = {
            let opcode_set = if prefixed {
                code = self.read_mem_u8(self.pc.wrapping_add(1));
                &self.prefixed_opcodes
            } else {
                &self.normal_opcodes
            };

            let opcode = match opcode_set.get(&code) {
                Some(op) => op,
                None => {
                    if prefixed {
                        return Err(self.crash(CpuError::UnrecognizedOpcode(code, true)));
                    } else {
                        return Err(self.crash(CpuError::UnrecognizedOpcode(code, false)));
                    }
                }
            };
            (
                opcode.asm.to_owned(),
                opcode.bytes as u16,
                opcode.t_cycles as usize,
                opcode.lhs.clone(),
                opcode.rhs.clone(),
            )
        };

        self.debugger
            .borrow_mut()
            .push_call_log(self.pc, code, &opcode_asm);

        // Execute instruction
        let mut skip_pc_increase = false;
        let mut extra_cycles: usize = 0;
        if prefixed {
            code = self.read_mem_u8(self.pc.wrapping_add(1));
            match code {
                0x00..=0x07 => self.rotate(&lhs, Direction::Left, true, false),
                0x08..=0x0f => self.rotate(&lhs, Direction::Right, true, false),
                0x10..=0x17 => self.rotate(&lhs, Direction::Left, true, true),
                0x18..=0x1f => self.rotate(&lhs, Direction::Right, true, true),
                0x20..=0x27 => self.shift(&lhs, Direction::Left, false),
                0x28..=0x2f => self.shift(&lhs, Direction::Right, false),
                0x30..=0x37 => self.swap(&lhs),
                0x38..=0x3f => self.shift(&lhs, Direction::Right, true),
                0x40..=0x47 => self.check_bit(0, &rhs),
                0x48..=0x4f => self.check_bit(1, &rhs),
                0x50..=0x57 => self.check_bit(2, &rhs),
                0x58..=0x5f => self.check_bit(3, &rhs),
                0x60..=0x67 => self.check_bit(4, &rhs),
                0x68..=0x6f => self.check_bit(5, &rhs),
                0x70..=0x77 => self.check_bit(6, &rhs),
                0x78..=0x7f => self.check_bit(7, &rhs),
                0x80..=0x87 => self.reset_bit(0, &rhs),
                0x88..=0x8f => self.reset_bit(1, &rhs),
                0x90..=0x97 => self.reset_bit(2, &rhs),
                0x98..=0x9f => self.reset_bit(3, &rhs),
                0xa0..=0xa7 => self.reset_bit(4, &rhs),
                0xa8..=0xaf => self.reset_bit(5, &rhs),
                0xb0..=0xb7 => self.reset_bit(6, &rhs),
                0xb8..=0xbf => self.reset_bit(7, &rhs),
                0xc0..=0xc7 => self.set_bit(0, &rhs),
                0xc8..=0xcf => self.set_bit(1, &rhs),
                0xd0..=0xd7 => self.set_bit(2, &rhs),
                0xd8..=0xdf => self.set_bit(3, &rhs),
                0xe0..=0xe7 => self.set_bit(4, &rhs),
                0xe8..=0xef => self.set_bit(5, &rhs),
                0xf0..=0xf7 => self.set_bit(6, &rhs),
                0xf8..=0xff => self.set_bit(7, &rhs),
            };
        } else {
            match code {
                0x00 => (),
                0x10 => (),
                0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => self.decrement_u8(&lhs),
                0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => self.increment_u8(&lhs),
                0x03 | 0x13 | 0x23 | 0x33 => self.increment_u16(&lhs),
                0x0b | 0x1b | 0x2b | 0x3b => self.decrement_u16(&lhs),
                0x09 | 0x19 | 0x29 | 0x39 => self.add_hl_u16(&rhs),
                0x76 => (), // TODO: HALT OPCODE
                0x01
                | 0x02
                | 0x06
                | 0x08
                | 0x0a
                | 0x0e
                | 0x11
                | 0x12
                | 0x16
                | 0x1a
                | 0x1e
                | 0x21
                | 0x26
                | 0x2e
                | 0x31
                | 0x36
                | 0x3e
                | 0x40..=0x75
                | 0xe2
                | 0xe0
                | 0xea
                | 0x77..=0x7f
                | 0xf0
                | 0xfa => self.load_or_store_value(&lhs, &rhs, None),
                0x27 => self.daa(),
                0x22 | 0x2a => self.load_or_store_value(&lhs, &rhs, Some(StoreLoadModifier::IncHL)),
                0x32 | 0x3a => self.load_or_store_value(&lhs, &rhs, Some(StoreLoadModifier::DecHL)),
                0x07 => self.rotate(&lhs, Direction::Left, false, false),
                0x0f => self.rotate(&lhs, Direction::Right, false, false),
                0x17 => self.rotate(&lhs, Direction::Left, false, true),
                0x1f => self.rotate(&lhs, Direction::Right, false, true),
                0x18 => extra_cycles = self.rel_jump(&rhs, None),
                0x20 => extra_cycles = self.rel_jump(&rhs, Some(JumpCondition::NZ)),
                0x28 => extra_cycles = self.rel_jump(&rhs, Some(JumpCondition::Z)),
                0x30 => extra_cycles = self.rel_jump(&rhs, Some(JumpCondition::NC)),
                0x38 => extra_cycles = self.rel_jump(&rhs, Some(JumpCondition::C)),
                0x2f => self.cpl(),
                0x37 => self.scf(),
                0x3f => self.ccf(),
                0xc0 => {
                    extra_cycles = self.ret(Some(JumpCondition::NZ), false);
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xc1 | 0xd1 | 0xe1 | 0xf1 => self.pop_stack_instr(&lhs),
                0xc2 => {
                    extra_cycles = self.abs_jump(&rhs, Some(JumpCondition::NZ));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                },
                0xc3 | 0xe9 => {
                    skip_pc_increase = true;
                    _ = self.abs_jump(&lhs, None);
                }
                0xc4 => {
                    extra_cycles = self.call(&rhs, Some(JumpCondition::NZ));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xc5 | 0xd5 | 0xe5 | 0xf5 => self.push_stack_instr(&lhs),
                0xc8 => {
                    extra_cycles = self.ret(Some(JumpCondition::Z), false);
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xc9 => {
                    skip_pc_increase = true;
                    self.ret(None, false);
                }
                0xcd => {
                    skip_pc_increase = true;
                    self.call(&lhs, None);
                }
                0xd0 => {
                    extra_cycles = self.ret(Some(JumpCondition::NC), false);
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xd2 => {
                    extra_cycles = self.abs_jump(&rhs, Some(JumpCondition::NC));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xd4 => {
                    extra_cycles = self.call(&rhs, Some(JumpCondition::NC));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0x80..=0x87 | 0xc6 => self.add_a_u8(&rhs),
                0x88..=0x8f | 0xce => self.adc(&rhs),
                0x90..=0x97 | 0xd6 => self.sub_a(&rhs, true),
                0x98..=0x9f | 0xde => self.sbc(&rhs),
                0xa0..=0xa7 | 0xe6 => self.and(&rhs),
                0xa8..=0xaf | 0xee => self.xor_with_a(&rhs),
                0xb0..=0xb7 | 0xf6 => self.or_with_a(&rhs),
                0xb8..=0xbf | 0xfe => self.sub_a(&rhs, false),
                0xe8 => self.add_sp_e8(&rhs),
                0xf8 => self.ld_hl_sp_e8(&rhs),
                0xf3 => self.ime = true,
                0xfb => self.ime = false,
                _ => return Err(self.crash(CpuError::OpcodeNotImplemented(code, false))),
            };
        };

        if !skip_pc_increase {
            self.pc = self.pc.wrapping_add(opcode_bytes);
        }
        Ok(opcode_cycles + extra_cycles)
    }

    pub fn _load_state(&mut self, state: &State) {
        self.reg.a = state.a;
        self.reg.b = state.b;
        self.reg.c = state.c;
        self.reg.d = state.d;
        self.reg.e = state.e;
        self.reg.f = state.f;
        self.reg.h = state.h;
        self.reg.l = state.l;
        self.sp = state.sp;
        self.pc = state.pc;
    }

    pub fn _get_state(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8, u16, u16) {
        (
            self.reg.a, self.reg.b, self.reg.c, self.reg.d, self.reg.e, self.reg.f, self.reg.h,
            self.reg.l, self.sp, self.pc,
        )
    }
}
