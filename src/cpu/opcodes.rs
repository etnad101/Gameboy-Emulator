use std::collections::HashMap;

#[derive(Clone)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
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
    JoypadU8,
    ImmediateI8,
    ImmediateU16,
    AdressU16,
}

pub struct Opcode {
    pub code: u8,
    pub name: String,
    pub bytes: u8,
    pub cycles: u8,
    pub addressing_mode: AddressingMode,
}

impl Opcode {
    pub fn new(
        code: u8,
        name: String,
        bytes: u8,
        cycles: u8,
        addressing_mode: AddressingMode,
    ) -> Self {
        Opcode {
            code,
            name,
            bytes,
            cycles,
            addressing_mode,
        }
    }

    pub fn generate_normal_opcode_map() -> HashMap<u8, Opcode> {
        let opcodes: Vec<Opcode> = vec![
            Opcode::new(
                0x31,
                "LD SP,n16".to_string(),
                3,
                3,
                AddressingMode::ImmediateU16,
            ),
            Opcode::new(
                0xaf,
                "XOR A,r8".to_string(),
                1,
                1,
                AddressingMode::ImmediateRegister(Register::A),
            ),
            Opcode::new(
                0x21,
                "LD r16,n16".to_string(),
                3,
                3,
                AddressingMode::ImmediateU16,
            ),
            Opcode::new(
                0x32,
                "LD [HL-],A".to_string(),
                1,
                2,
                AddressingMode::AddressRegister(Register::HL),
            ),
        ];

        let mut map = HashMap::new();
        for op in opcodes {
            map.insert(op.code, op);
        }
        map
    }

    pub fn generate_prefixed_opcode_map() -> HashMap<u8, Opcode> {
        let opcodes: Vec<Opcode> = vec![Opcode::new(
            0x7c,
            "BIT 7, H".to_string(),
            2,
            2,
            AddressingMode::ImmediateRegister(Register::A),
        )];

        let mut map = HashMap::new();
        for op in opcodes {
            map.insert(op.code, op);
        }
        map
    }
}
