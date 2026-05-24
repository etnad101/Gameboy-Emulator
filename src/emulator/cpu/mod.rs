pub mod opcodes;
pub(super) mod state;

use super::{debug::DebugCtx, errors::CpuError};
use crate::{
    emulator::{
        cpu::{
            opcodes::{AddressingMode, Opcode, Register},
            state::CpuState,
        },
        memory::Bus,
    },
    utils::bit_ops::BitOps,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
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

pub struct Cpu<B: Bus> {
    state: CpuState,
    normal_opcodes: HashMap<u8, Opcode>,
    prefixed_opcodes: HashMap<u8, Opcode>,
    memory: Rc<RefCell<B>>,
}

impl<B: Bus> Cpu<B> {
    pub fn new(memory: Rc<RefCell<B>>) -> Self {
        Self {
            state: CpuState::new(),
            normal_opcodes: Opcode::generate_normal_opcode_map(),
            prefixed_opcodes: Opcode::generate_prefixed_opcode_map(),
            memory,
        }
    }

    // Debugging methods

    pub fn crash(&self, error: CpuError, debug_ctx: &mut DebugCtx<B>) -> CpuError {
        debug_ctx.dump_logs();
        eprintln!("{:#06x}", self.state.pc);
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
                Register::A => DataType::ValueU8(self.state.a),
                Register::B => DataType::ValueU8(self.state.b),
                Register::C => DataType::ValueU8(self.state.c),
                Register::D => DataType::ValueU8(self.state.d),
                Register::E => DataType::ValueU8(self.state.e),
                Register::H => DataType::ValueU8(self.state.h),
                Register::L => DataType::ValueU8(self.state.l),
                Register::AF => DataType::ValueU16(self.state.af()),
                Register::BC => DataType::ValueU16(self.state.bc()),
                Register::DE => DataType::ValueU16(self.state.de()),
                Register::HL => DataType::ValueU16(self.state.hl()),
                Register::SP => DataType::ValueU16(self.state.sp),
            },
            AddressingMode::AddressRegister(register) => match register {
                Register::BC => DataType::Address(self.state.bc()),
                Register::DE => DataType::Address(self.state.de()),
                Register::HL => DataType::Address(self.state.hl()),
                _ => todo!("Address_Register not implemented"),
            },
            AddressingMode::ImmediateU8 => {
                DataType::ValueU8(self.read_mem_u8(self.state.pc.wrapping_add(1)))
            }
            AddressingMode::AddressHRAM => {
                let hi = 0xFF00;
                let lo: u16 = u16::from(self.read_mem_u8(self.state.pc.wrapping_add(1)));
                let addr = hi + lo;
                DataType::Address(addr)
            }
            AddressingMode::ImmediateI8 => {
                DataType::ValueI8(self.read_mem_u8(self.state.pc.wrapping_add(1)) as i8)
            }
            AddressingMode::ImmediateU16 => {
                DataType::ValueU16(self.read_mem_u16(self.state.pc.wrapping_add(1)))
            }
            AddressingMode::AddressU16 => {
                DataType::Address(self.read_mem_u16(self.state.pc.wrapping_add(1)))
            }
            AddressingMode::IoAddressOffset => DataType::Address(0xFF00 | u16::from(self.state.c)),
            AddressingMode::None => DataType::None,
        }
    }

    fn set_immediate_register_u8(&mut self, reg: &Register, value: u8) {
        match reg {
            Register::A => self.state.a = value,
            Register::B => self.state.b = value,
            Register::C => self.state.c = value,
            Register::D => self.state.d = value,
            Register::E => self.state.e = value,
            Register::H => self.state.h = value,
            Register::L => self.state.l = value,
            _ => unreachable!("Can only set 8 bit registers"),
        }
    }

    fn set_immediate_register_u16(&mut self, reg: &Register, value: u16) {
        match reg {
            Register::AF => self.state.set_af(value),
            Register::BC => self.state.set_bc(value),
            Register::DE => self.state.set_de(value),
            Register::HL => self.state.set_hl(value),
            Register::SP => self.state.sp = value,
            _ => unreachable!("Can only set 16 bit registers"),
        }
    }

    pub fn push_stack(&mut self, value: u16) {
        let hi = ((value & 0xFF00) >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.state.sp -= 1;
        self.write_mem_u8(self.state.sp, hi);
        self.state.sp -= 1;
        self.write_mem_u8(self.state.sp, lo);
    }

    pub fn pop_stack(&mut self) -> u16 {
        let lo = self.read_mem_u8(self.state.sp);
        self.state.sp += 1;
        let hi = self.read_mem_u8(self.state.sp);
        self.state.sp += 1;
        (u16::from(hi) << 8) | u16::from(lo)
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
                DataType::ValueU8(value) => self.set_immediate_register_u8(reg, value),
                DataType::ValueU16(value) => self.set_immediate_register_u16(reg, value),
                DataType::Address(addr) => {
                    let value = self.read_mem_u8(addr);
                    match reg {
                        Register::A => self.state.a = value,
                        Register::B => self.state.b = value,
                        Register::C => self.state.c = value,
                        Register::D => self.state.d = value,
                        Register::E => self.state.e = value,
                        Register::H => self.state.h = value,
                        Register::L => self.state.l = value,
                        _ => unreachable!("Must store u8 value in u8 register"),
                    }
                }
                _ => unreachable!("Should not have none or i8 here"),
            },
            AddressingMode::AddressRegister(reg) => {
                let addr = match reg {
                    Register::BC => self.state.bc(),
                    Register::DE => self.state.de(),
                    Register::HL => self.state.hl(),
                    _ => unreachable!("address can't come from 8 bit registers"),
                };

                match data {
                    DataType::ValueU8(value) => self.write_mem_u8(addr, value),
                    _ => unreachable!("Should only write u8 to mem / not implemented - check docs"),
                }
            }
            AddressingMode::AddressU16
            | AddressingMode::IoAddressOffset
            | AddressingMode::AddressHRAM => {
                let addr: u16 = match self.get_data(lhs) {
                    DataType::Address(addr) => addr,
                    _ => unreachable!("Should only have address here"),
                };

                match data {
                    DataType::ValueU8(val) => self.write_mem_u8(addr, val),
                    DataType::ValueU16(val) => {
                        let lo = val & 0xFF;
                        let hi = val >> 8;
                        self.write_mem_u8(addr, lo as u8);
                        self.write_mem_u8(addr + 1, hi as u8);
                    }
                    _ => unreachable!("Should only have u8 or u16 here"),
                }
            }
            _ => unreachable!("Should only have an address or value"),
        }

        match modifier {
            Some(StoreLoadModifier::DecHL) => self.state.set_hl(self.state.hl() - 1),
            Some(StoreLoadModifier::IncHL) => self.state.set_hl(self.state.hl() + 1),
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
            _ => unreachable!("Expected u8 here"),
        };

        let (sum, half_carry, _) = self.carry_track_add_u8(value, 1);

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, sum),
            AddressingMode::AddressRegister(Register::HL) => {
                self.write_mem_u8(self.state.hl(), sum)
            }
            _ => unreachable!("Should not have any other addressing mode"),
        };

        self.state.set_z_flag_from_value(sum);

        self.state.clear_n_flag();

        if half_carry {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }
    }

    fn increment_u16(&mut self, lhs: &AddressingMode) {
        let sum = match self.get_data(lhs) {
            DataType::ValueU16(val) => val.wrapping_add(1),
            _ => unreachable!("expected u16 here"),
        };

        match lhs {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u16(reg, sum),
            _ => unreachable!("Should not have any other addressing mode"),
        }
    }

    fn decrement_u8(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("expected u8 here"),
        };

        let (diff, half_borrow, _) = self.carry_track_sub_u8(value, 1);

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, diff),

            AddressingMode::AddressRegister(Register::HL) => {
                self.write_mem_u8(self.state.hl(), diff)
            }

            _ => unreachable!("Only use this fucntion for u8 values"),
        }

        self.state.set_z_flag_from_value(diff);

        self.state.set_n_flag();

        if half_borrow {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }
    }

    fn decrement_u16(&mut self, addressing_mode: &AddressingMode) {
        let mut byte = match self.get_data(addressing_mode) {
            DataType::ValueU16(val) => val,
            _ => unreachable!("Should only have value from 16 bit register here"),
        };

        byte = byte.wrapping_sub(1);

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u16(reg, byte),
            _ => unreachable!("Should only have 16 bit register here"),
        }
    }

    fn rel_jump(
        &mut self,
        addressing_mode: &AddressingMode,
        condition: Option<JumpCondition>,
    ) -> usize {
        let offset = match self.get_data(addressing_mode) {
            DataType::ValueI8(val) => val,
            _ => unreachable!("Should only have i8 here"),
        };

        let mut jump = false;
        let extra_cycles: usize = match condition {
            Some(JumpCondition::Z) => {
                if self.state.get_z_flag() != 0 {
                    jump = true;
                };
                4
            }
            Some(JumpCondition::NZ) => {
                if self.state.get_z_flag() == 0 {
                    jump = true;
                };
                4
            }
            Some(JumpCondition::C) => {
                if self.state.get_c_flag() != 0 {
                    jump = true;
                };
                4
            }
            Some(JumpCondition::NC) => {
                if self.state.get_c_flag() == 0 {
                    jump = true;
                };
                4
            }
            None => {
                jump = true;
                0
            }
        };

        if jump {
            let res: i16 = (self.state.pc as i16).wrapping_add(i16::from(offset));
            self.state.pc = res as u16;
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
                if self.state.get_z_flag() == 0 {
                    (true, 4)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::NC) => {
                if self.state.get_c_flag() == 0 {
                    (true, 4)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::Z) => {
                if self.state.get_z_flag() == 1 {
                    (true, 4)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::C) => {
                if self.state.get_c_flag() == 1 {
                    (true, 4)
                } else {
                    (false, 0)
                }
            }
            None => (true, 0),
        };

        if jump {
            let addr = match self.get_data(addressing_mode) {
                DataType::Address(addr) => addr,
                _ => unreachable!("Should only have an address here"),
            };

            self.state.pc = addr;
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
                if self.state.get_z_flag() == 0 {
                    (true, 12)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::NC) => {
                if self.state.get_c_flag() == 0 {
                    (true, 12)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::Z) => {
                if self.state.get_z_flag() == 1 {
                    (true, 12)
                } else {
                    (false, 0)
                }
            }
            Some(JumpCondition::C) => {
                if self.state.get_c_flag() == 1 {
                    (true, 12)
                } else {
                    (false, 0)
                }
            }
            None => (true, 0),
        };

        if jump {
            let addr = match self.get_data(addressing_mode) {
                DataType::Address(addr) => addr,
                _ => unreachable!("Should only have an address here"),
            };

            self.push_stack(self.state.pc.wrapping_add(3));
            self.state.pc = addr;
        }
        extra_cycles
    }

    fn ret(&mut self, condition: Option<JumpCondition>, set_ime: bool) -> usize {
        let jump = match condition {
            Some(JumpCondition::Z) => self.state.get_z_flag() == 1,
            Some(JumpCondition::NZ) => self.state.get_z_flag() == 0,
            Some(JumpCondition::C) => self.state.get_c_flag() == 1,
            Some(JumpCondition::NC) => self.state.get_c_flag() == 0,
            None => {
                self.state.pc = self.pop_stack();
                if set_ime {
                    // self.write_mem_u8(0xFFFF, 0xFF);
                    self.state.ime = true;
                }
                return 0;
            }
        };

        if jump {
            self.state.pc = self.pop_stack();
            12
        } else {
            0
        }
    }

    fn push_stack_instr(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU16(value) => value,
            _ => unreachable!("Only expected u16 value here"),
        };

        self.push_stack(value);
    }

    fn pop_stack_instr(&mut self, addressing_mode: &AddressingMode) {
        let value = self.pop_stack();

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u16(reg, value),
            _ => unreachable!("Can only pop stack to 16 bit register"),
        }
    }

    fn shift(&mut self, addressing_mode: &AddressingMode, direction: Direction, logical: bool) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("Should only have u8 value here"),
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

        self.state.set_z_flag_from_value(new_val);

        self.state.clear_n_flag();
        self.state.clear_h_flag();

        if shifted_out_bit == 1 {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, new_val),
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => unreachable!("Expected addr value here"),
                };

                self.write_mem_u8(addr, new_val);
            }
            _ => unreachable!("Should only have r8 or address register"),
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
            _ => unreachable!("Expected u8 value here"),
        };

        let (shifted_out_bit, new_val) = match direction {
            Direction::Left => {
                let shifted_out_bit = data.get_bit(7);
                let new_val = if through_carry {
                    (data << 1) | self.state.get_c_flag()
                } else {
                    (data << 1) | shifted_out_bit
                };
                (shifted_out_bit, new_val)
            }
            Direction::Right => {
                let shifted_out_bit = data.get_bit(0);
                let new_val = if through_carry {
                    (data >> 1) | (self.state.get_c_flag() << 7)
                } else {
                    (data >> 1) | (shifted_out_bit << 7)
                };
                (shifted_out_bit, new_val)
            }
        };

        if update_z && (new_val == 0) {
            self.state.set_z_flag();
        } else {
            self.state.clear_z_flag();
        }

        self.state.clear_n_flag();
        self.state.clear_h_flag();

        if shifted_out_bit == 1 {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, new_val),
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => unreachable!("Expected addr value here"),
                };

                self.write_mem_u8(addr, new_val);
            }
            _ => unreachable!("Should only have r8 or address register"),
        }
    }

    fn swap(&mut self, addressing_mode: &AddressingMode) {
        let value = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("Expected u8 or addr here"),
        };

        let hi = value >> 4;
        let lo = value & 0x0F;

        let new_val = (lo << 4) | hi;

        self.state.set_z_flag_from_value(new_val);

        self.state.clear_n_flag();
        self.state.clear_h_flag();
        self.state.clear_c_flag();

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, new_val),
            AddressingMode::AddressRegister(_) => {
                let addr = match self.get_data(addressing_mode) {
                    DataType::Address(addr) => addr,
                    _ => unreachable!("Expected addr value here"),
                };

                self.write_mem_u8(addr, new_val);
            }
            _ => unreachable!("Should only have r8 or address register"),
        }
    }

    fn set_u8_add_registers(&mut self, sum: u8, half_carry: bool, full_carry: bool) {
        self.state.set_z_flag_from_value(sum);

        self.state.clear_n_flag();

        if half_carry {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }

        if full_carry {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        self.state.a = sum;
    }

    fn add_a_u8(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("Should not have any other data type here"),
        };

        let (sum, half_carry, full_carry) = self.carry_track_add_u8(self.state.a, value);

        self.set_u8_add_registers(sum, half_carry, full_carry);
    }

    fn add_hl_u16(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU16(val) => val,
            _ => unreachable!("Should only have a u16 value here"),
        };

        let hl = self.state.hl();
        let (res, carry) = hl.overflowing_add(value);
        let half_carry = (((hl & 0xFFF) + (value & 0xFFF)) & 0x1000) == 0x1000;

        self.state.set_hl(res);

        self.state.clear_n_flag();

        if half_carry {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }

        if carry {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }
    }

    fn add_sp_e8(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueI8(val) => i16::from(val),
            _ => unreachable!("Should only have an i8 here"),
        };

        let s8 = (value & 127) - (value & 128);

        let before = self.state.sp;
        self.state.sp = (self.state.sp as i16).wrapping_add(value as i16) as u16;

        let full_carry: bool = if value >= 0 {
            ((before as i16 & 0xFF) + s8) > 0xFF
        } else {
            (self.state.sp & 0xFF) < (before & 0xFF)
        };

        let half_carry: bool = if value >= 0 {
            ((before as i16 & 0xF) + (s8 & 0xF)) > 0xF
        } else {
            (self.state.sp & 0xF) < (before & 0xF)
        };

        self.state.clear_z_flag();
        self.state.clear_n_flag();

        if full_carry {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        if half_carry {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }
    }

    fn ld_hl_sp_e8(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueI8(val) => i16::from(val),
            _ => unreachable!("Should only have an i8 here"),
        };

        self.state
            .set_hl((self.state.sp as i16).wrapping_add(value) as u16);

        let full_carry = ((self.state.sp as i16 & 0xFF) + (value & 0xFF)) > 0xFF;
        let half_carry = ((self.state.sp as i16 & 0xF) + (value & 0xF)) > 0xF;

        self.state.clear_z_flag();
        self.state.clear_n_flag();

        if full_carry {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        if half_carry {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }
    }

    fn adc(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("expecred u8 here"),
        };

        let (sum, half_carry_1, carry_1) = self.carry_track_add_u8(self.state.a, value);
        let (sum, half_carry_2, carry_2) = self.carry_track_add_u8(sum, self.state.get_c_flag());

        let half_carry = half_carry_1 | half_carry_2;
        let full_carry = carry_1 | carry_2;

        self.set_u8_add_registers(sum, half_carry, full_carry);
    }

    fn sub_a(&mut self, rhs: &AddressingMode, store_result: bool) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("Should only have u8"),
        };

        let (diff, half_borrow, borrow) = self.carry_track_sub_u8(self.state.a, value);

        self.state.set_z_flag_from_value(diff);

        self.state.set_n_flag();

        if half_borrow {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }

        if borrow {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        if store_result {
            self.state.a = diff;
        }
    }

    fn sbc(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("Should only have u8 value here"),
        };

        let (diff, half_borrow_1, borrow_1) = self.carry_track_sub_u8(self.state.a, value);
        let (diff, half_borrow_2, borrow_2) =
            self.carry_track_sub_u8(diff, self.state.get_c_flag());

        let half_borrow = half_borrow_1 | half_borrow_2;
        let borrow = borrow_1 | borrow_2;

        self.state.set_z_flag_from_value(diff);

        self.state.set_n_flag();

        if half_borrow {
            self.state.set_h_flag();
        } else {
            self.state.clear_h_flag();
        }

        if borrow {
            self.state.set_c_flag();
        } else {
            self.state.clear_c_flag();
        }

        self.state.a = diff;
    }

    fn and(&mut self, rhs: &AddressingMode) {
        let value = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("Expected u8 here"),
        };

        let res = self.state.a & value;

        self.state.set_z_flag_from_value(res);

        self.state.clear_n_flag();
        self.state.set_h_flag();
        self.state.clear_c_flag();

        self.state.a = res;
    }

    fn xor_with_a(&mut self, rhs: &AddressingMode) {
        let res = match self.get_data(rhs) {
            DataType::ValueU8(val) => self.state.a ^ val,
            DataType::Address(addr) => {
                let val = self.read_mem_u8(addr);
                val ^ self.state.a
            }
            _ => unreachable!("Should only have u8"),
        };

        self.state.a = res;
        self.state.set_z_flag_from_value(res);

        self.state.clear_n_flag();
        self.state.clear_h_flag();
        self.state.clear_c_flag();
    }

    fn or_with_a(&mut self, rhs: &AddressingMode) {
        let byte = match self.get_data(rhs) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("no other data type should be here"),
        };

        let res = self.state.a | byte;
        self.state.a = res;

        self.state.set_z_flag_from_value(res);

        self.state.clear_n_flag();
        self.state.clear_h_flag();
        self.state.clear_c_flag();
    }

    fn check_bit(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let byte = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => val,
            DataType::Address(addr) => self.read_mem_u8(addr),
            _ => unreachable!("bit check not yet implemented or dosent exist"),
        };
        self.state.set_z_flag_from_value(byte.get_bit(bit));
        self.state.clear_n_flag();
        self.state.set_h_flag();
    }

    fn set_bit(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let (mut byte, addr) = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => (val, None),
            DataType::Address(addr) => (self.read_mem_u8(addr), Some(addr)),
            _ => unreachable!("Should not have any other type here"),
        };
        byte.set_bit(bit);

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, byte),
            AddressingMode::AddressRegister(Register::HL) => self.write_mem_u8(addr.unwrap(), byte),
            _ => unreachable!("should not have anything else here"),
        };
    }

    fn reset_bit(&mut self, bit: u8, addressing_mode: &AddressingMode) {
        let (mut byte, addr) = match self.get_data(addressing_mode) {
            DataType::ValueU8(val) => (val, None),
            DataType::Address(addr) => (self.read_mem_u8(addr), Some(addr)),
            _ => unreachable!("Should not have any other type here"),
        };
        byte.clear_bit(bit);

        match addressing_mode {
            AddressingMode::ImmediateRegister(reg) => self.set_immediate_register_u8(reg, byte),
            AddressingMode::AddressRegister(Register::HL) => self.write_mem_u8(addr.unwrap(), byte),
            _ => unreachable!("should not have anything else here"),
        };
    }

    fn daa(&mut self) {
        if self.state.get_n_flag() == 0 {
            // after an addition, adjust if (half-)carry occurred or if result is out of bounds
            if (self.state.get_c_flag() == 1) || self.state.a > 0x99 {
                self.state.a = self.state.a.wrapping_add(0x60);
                self.state.set_c_flag();
            }
            if self.state.get_h_flag() == 1 || (self.state.a & 0x0f) > 0x09 {
                self.state.a = self.state.a.wrapping_add(0x6);
            }
        } else {
            // after a subtraction, only adjust if (half-)carry occurred
            if self.state.get_c_flag() == 1 {
                self.state.a = self.state.a.wrapping_sub(0x60);
            }
            if self.state.get_h_flag() == 1 {
                self.state.a = self.state.a.wrapping_sub(0x6);
            }
        }

        self.state.set_z_flag_from_value(self.state.a);

        self.state.clear_h_flag();
    }

    fn cpl(&mut self) {
        self.state.a = !self.state.a;
        self.state.set_n_flag();
        self.state.set_h_flag();
    }

    fn scf(&mut self) {
        self.state.clear_n_flag();
        self.state.clear_h_flag();
        self.state.set_c_flag();
    }

    fn ccf(&mut self) {
        self.state.clear_n_flag();
        self.state.clear_h_flag();

        if self.state.get_c_flag() == 1 {
            self.state.clear_c_flag();
        } else {
            self.state.set_c_flag();
        }
    }

    fn reset_vec(&mut self, addr: u16) {
        self.push_stack(self.state.pc.wrapping_add(1));
        self.state.pc = addr;
    }

    pub fn execute_next_opcode(&mut self, debug_ctx: &mut DebugCtx<B>) -> Result<usize, CpuError> {
        // Get next instruction
        let mut code = self.read_mem_u8(self.state.pc);
        let prefixed = code == 0xcb;

        let (opcode_asm, opcode_bytes, opcode_cycles, lhs, rhs) = {
            let opcode_set = if prefixed {
                code = self.read_mem_u8(self.state.pc.wrapping_add(1));
                &self.prefixed_opcodes
            } else {
                &self.normal_opcodes
            };

            let opcode = match opcode_set.get(&code) {
                Some(op) => op,
                None => {
                    if prefixed {
                        return Err(self.crash(CpuError::UnrecognizedOpcode(code, true), debug_ctx));
                    } else {
                        return Err(
                            self.crash(CpuError::UnrecognizedOpcode(code, false), debug_ctx)
                        );
                    }
                }
            };
            (
                opcode.asm.to_owned(),
                u16::from(opcode.bytes),
                opcode.t_cycles as usize,
                opcode.lhs.clone(),
                opcode.rhs.clone(),
            )
        };

        debug_ctx.push_call_log(self.state.pc, code, prefixed);

        // Execute instruction
        let mut skip_pc_increase = false;
        let mut extra_cycles: usize = 0;
        if prefixed {
            code = self.read_mem_u8(self.state.pc.wrapping_add(1));
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
                | 0xf2
                | 0xf9
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
                }
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
                0xca => {
                    extra_cycles = self.abs_jump(&rhs, Some(JumpCondition::Z));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xcc => {
                    extra_cycles = self.call(&rhs, Some(JumpCondition::Z));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
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
                0xd8 => {
                    extra_cycles = self.ret(Some(JumpCondition::C), false);
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xd9 => {
                    skip_pc_increase = true;
                    extra_cycles = self.ret(None, true);
                }
                0xda => {
                    extra_cycles = self.abs_jump(&rhs, Some(JumpCondition::C));
                    if extra_cycles > 0 {
                        skip_pc_increase = true;
                    }
                }
                0xdc => {
                    extra_cycles = self.call(&rhs, Some(JumpCondition::C));
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
                0xc7 => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0000);
                }
                0xcf => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0008);
                }
                0xd7 => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0010);
                }
                0xdf => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0018);
                }
                0xe7 => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0020);
                }
                0xef => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0028);
                }
                0xf7 => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0030);
                }
                0xff => {
                    skip_pc_increase = true;
                    self.reset_vec(0x0038);
                }
                0xe8 => self.add_sp_e8(&rhs),
                0xf8 => self.ld_hl_sp_e8(&rhs),
                0xf3 => self.state.ime = false,
                0xfb => self.state.ime = true,
                _ => return Err(self.crash(CpuError::OpcodeNotImplemented(code, false), debug_ctx)),
            };
        };

        if !skip_pc_increase {
            self.state.pc = self.state.pc.wrapping_add(opcode_bytes);
        }
        Ok(opcode_cycles + extra_cycles)
    }

    pub fn handle_interrupts(&mut self, debug_ctx: &mut DebugCtx<B>) -> Option<usize> {
        if !self.state.ime {
            return None;
        }

        let interrupt_enable = self.memory.borrow().read_u8(0xFFFF); // Interrupt enable address
        let mut interrupt_flag = self.memory.borrow().read_u8(0xFF0F); // Interrupt flag address

        let triggered_interrupts = interrupt_enable & interrupt_flag;

        if triggered_interrupts == 0 {
            return None;
        }

        for bit in 0..5 {
            if triggered_interrupts.get_bit(bit) != 0 {
                self.state.ime = false;
                interrupt_flag.clear_bit(bit);
                self.memory.borrow_mut().write_u8(0xFF0F, interrupt_flag);

                self.push_stack(self.state.pc);
                self.state.pc = match bit {
                    0 => 0x40,
                    1 => 0x48,
                    2 => 0x50,
                    3 => 0x58,
                    4 => 0x60,
                    _ => unreachable!(),
                };
                debug_ctx.push_note(format!("triggered interrupt: {:4x}", self.state.pc));
                debug_ctx.dump_logs();
                return Some(20);
            }
        }
        None
    }

    pub fn load_state(&mut self, state: CpuState) {
        self.state = state;
    }

    pub fn get_state(&self) -> CpuState {
        self.state.clone()
    }
}
