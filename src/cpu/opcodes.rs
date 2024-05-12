// TOOD: Give every opcode a lhs and rhs addressing mode

use std::{collections::HashMap, ops::Add};

#[derive(Clone)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    AF,
    BC,
    DE,
    HL,
    SP,
}

#[derive(Clone)]
pub enum AddressingMode {
    ImmediateRegister(Register),
    AddressRegister(Register),
    ImmediateU8,
    AddressHRAM,
    ImmediateI8,
    ImmediateU16,
    AddressU16,
    IoAdressOffset,
    None,
}

pub struct Opcode {
    pub code: u8,
    pub asm: String,
    pub bytes: u8,
    pub cycles: u8,
    pub lhs: AddressingMode,
    pub rhs: AddressingMode,
}

impl Opcode {
    pub fn new(
        code: u8,
        asm: String,
        bytes: u8,
        cycles: u8,
        lhs: AddressingMode,
        rhs: AddressingMode,
    ) -> Self {
        Opcode {
            code,
            asm,
            bytes,
            cycles,
            lhs,
            rhs,
        }
    }

    pub fn generate_normal_opcode_map() -> HashMap<u8, Opcode> {
        let opcodes: Vec<Opcode> = vec![
            // Inc/Dec Instructions
            Opcode::new(
                0x05,
                "DEC B".to_string(),
                1,
                1,
                AddressingMode::ImmediateRegister(Register::B),
                AddressingMode::None,
            ),
            Opcode::new(
                0x0c,
                "INC C".to_string(),
                1,
                1,
                AddressingMode::ImmediateRegister(Register::C),
                AddressingMode::None,
            ),
            Opcode::new(
                0x23,
                "INC HL".to_string(),
                1,
                2,
                AddressingMode::ImmediateRegister(Register::HL),
                AddressingMode::None,
            ),
            // Load Instructions
            Opcode::new(
                0x06,
                "LD B, n8".to_string(),
                2,
                2,
                AddressingMode::ImmediateRegister(Register::B),
                AddressingMode::ImmediateU8,
            ),
            Opcode::new(
                0x0e,
                "LD C, n8".to_string(),
                2,
                2,
                AddressingMode::ImmediateRegister(Register::C),
                AddressingMode::ImmediateU8,
            ),
            Opcode::new(
                0x11,
                "LD DE, n16".to_string(),
                3,
                3,
                AddressingMode::ImmediateRegister(Register::DE),
                AddressingMode::ImmediateU16,
            ),
            Opcode::new(
                0x1a,
                "LD A, [DE]".to_string(),
                1,
                2,
                AddressingMode::ImmediateRegister(Register::A),
                AddressingMode::AddressRegister(Register::DE),
            ),
            Opcode::new(
                0x22,
                "LD [HLI], A".to_string(),
                1,
                2,
                AddressingMode::AddressRegister(Register::HL),
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0x3e,
                "LD A, n8".to_string(),
                2,
                2,
                AddressingMode::ImmediateRegister(Register::A),
                AddressingMode::ImmediateU8,
            ),
            Opcode::new(
                0x31,
                "LD SP,n16".to_string(),
                3,
                3,
                AddressingMode::ImmediateRegister(Register::SP),
                AddressingMode::ImmediateU16,
            ),
            Opcode::new(
                0x21,
                "LD HL,n16".to_string(),
                3,
                3,
                AddressingMode::ImmediateRegister(Register::HL),
                AddressingMode::ImmediateU16,
            ),
            Opcode::new(
                0x77,
                "LD [HL], A".to_string(),
                1,
                2,
                AddressingMode::AddressRegister(Register::HL),
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0x32,
                "LD [HL-],A".to_string(),
                1,
                2,
                AddressingMode::AddressRegister(Register::HL),
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0x4f,
                "LD C, A".to_string(),
                1,
                2,
                AddressingMode::ImmediateRegister(Register::C),
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0xe0,
                "LDH [a8], A".to_string(),
                2,
                3,
                AddressingMode::AddressHRAM,
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0xe2,
                "LD [C], A".to_string(),
                1,
                2,
                AddressingMode::IoAdressOffset,
                AddressingMode::ImmediateRegister(Register::A),
            ),
            // Logic and Bit Instructions
            Opcode::new(
                0xaf,
                "XOR A".to_string(),
                1,
                1,
                AddressingMode::None,
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0x17,
                "RLA".to_string(),
                1,
                1,
                AddressingMode::ImmediateRegister(Register::A),
                AddressingMode::None,
            ),
            // Jump/Call Instructions
            Opcode::new(
                0xcd,
                "CALL a16".to_string(),
                3,
                6,
                AddressingMode::AddressU16,
                AddressingMode::None,
            ),
            Opcode::new(
                0xc9,
                "RET".to_string(),
                1,
                4,
                AddressingMode::None,
                AddressingMode::None,
            ),
            Opcode::new(
                0x20,
                "JR NZ, e8".to_string(),
                2,
                2, // + 1 if taken,
                AddressingMode::None,
                AddressingMode::ImmediateI8,
            ),
            // Stack
            Opcode::new(
                0xc5,
                "PUSH BC".to_string(),
                1,
                4,
                AddressingMode::ImmediateRegister(Register::BC),
                AddressingMode::None,
            ),
            Opcode::new(
                0xc1,
                "POP BC".to_string(),
                1,
                4,
                AddressingMode::ImmediateRegister(Register::BC),
                AddressingMode::None,
            ),
        ];

        let mut map = HashMap::new();
        for op in opcodes {
            map.insert(op.code, op);
        }
        map
    }

    pub fn generate_prefixed_opcode_map() -> HashMap<u8, Opcode> {
        let opcodes: Vec<Opcode> = vec![
            // Bit check instructions
            Opcode::new(
                0x7c,
                "BIT 7, H".to_string(),
                2,
                2,
                AddressingMode::None,
                AddressingMode::ImmediateRegister(Register::H),
            ),
            // Rotate instructions
            Opcode::new(
                0x11,
                "RL C".to_string(),
                2,
                2,
                AddressingMode::ImmediateRegister(Register::C),
                AddressingMode::None,
            ),
        ];

        let mut map = HashMap::new();
        for op in opcodes {
            map.insert(op.code, op);
        }
        map
    }
}
