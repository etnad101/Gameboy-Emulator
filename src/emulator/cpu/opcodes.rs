// TOOD: Give every opcode a lhs and rhs addressing mode

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

    #[rustfmt::skip]
    pub fn generate_normal_opcode_map() -> HashMap<u8, Opcode> {
        let opcodes: Vec<Opcode> = vec![
            // Inc/Dec Instructions
            Opcode::new(0x04, "INC B".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::B), AddressingMode::None),
            Opcode::new(0x05, "DEC B".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::B), AddressingMode::None),
            Opcode::new(0x0c, "INC C".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::C), AddressingMode::None),
            Opcode::new(0x0d, "DEC C".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::C), AddressingMode::None),
            Opcode::new(0x13, "INC DE".to_string(), 1, 2, AddressingMode::ImmediateRegister(Register::DE), AddressingMode::None),
            Opcode::new(0x15, "DEC D".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::D), AddressingMode::None),
            Opcode::new(0x1d, "DEC E".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::E), AddressingMode::None),
            Opcode::new(0x23, "INC HL".to_string(), 1, 2, AddressingMode::ImmediateRegister(Register::HL), AddressingMode::None),
            Opcode::new(0x24, "INC H".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::H), AddressingMode::None),
            Opcode::new(0x3d, "DEC A".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::A), AddressingMode::None),
            // Load Instructions
            Opcode::new(0x06, "LD B, n8".to_string(), 2, 2, AddressingMode::ImmediateRegister(Register::B), AddressingMode::ImmediateU8),
            Opcode::new(0x0e, "LD C, n8".to_string(), 2, 2, AddressingMode::ImmediateRegister(Register::C), AddressingMode::ImmediateU8),
            Opcode::new(0x11, "LD DE, n16".to_string(), 3, 3, AddressingMode::ImmediateRegister(Register::DE), AddressingMode::ImmediateU16),
            Opcode::new(0x1a, "LD A, [DE]".to_string(), 1, 2, AddressingMode::ImmediateRegister(Register::A), AddressingMode::AddressRegister(Register::DE)),
            Opcode::new(0x1e, "LD E, n8".to_string(), 2, 2, AddressingMode::ImmediateRegister(Register::E), AddressingMode::ImmediateU8),
            Opcode::new(0x21, "LD HL, n16".to_string(), 3, 3, AddressingMode::ImmediateRegister(Register::HL), AddressingMode::ImmediateU16,),
            Opcode::new(0x22, "LD [HLI], A".to_string(), 1, 2, AddressingMode::AddressRegister(Register::HL), AddressingMode::ImmediateRegister(Register::A),),
            Opcode::new(0x2e, "LD L, n8".to_string(), 2, 2, AddressingMode::ImmediateRegister(Register::L), AddressingMode::ImmediateU8),
            Opcode::new(0x31, "LD SP,n16".to_string(), 3, 3, AddressingMode::ImmediateRegister(Register::SP), AddressingMode::ImmediateU16,),
            Opcode::new(0x32, "LD [HL-],A".to_string(), 1, 2, AddressingMode::AddressRegister(Register::HL), AddressingMode::ImmediateRegister(Register::A),),
            Opcode::new(0x3e, "LD A, n8".to_string(), 2, 2, AddressingMode::ImmediateRegister(Register::A), AddressingMode::ImmediateU8,),
            Opcode::new(0x4f, "LD C, A".to_string(), 1, 2, AddressingMode::ImmediateRegister(Register::C), AddressingMode::ImmediateRegister(Register::A),),
            Opcode::new(0x57, "LD D, A".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::D), AddressingMode::ImmediateRegister(Register::A)),
            Opcode::new(0x67, "LD H, A".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::H), AddressingMode::ImmediateRegister(Register::A)),
            Opcode::new(0x77, "LD [HL], A".to_string(), 1, 2, AddressingMode::AddressRegister(Register::HL), AddressingMode::ImmediateRegister(Register::A),),
            Opcode::new(0x7b, "LD A, E".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::A), AddressingMode::ImmediateRegister(Register::E)),
            Opcode::new(0x7c, "LD A, H".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::A), AddressingMode::ImmediateRegister(Register::H)),
            Opcode::new(0xe0, "LDH [a8], A".to_string(), 2, 3, AddressingMode::AddressHRAM, AddressingMode::ImmediateRegister(Register::A)),
            Opcode::new(0xe2, "LD [C], A".to_string(), 1, 2, AddressingMode::IoAdressOffset, AddressingMode::ImmediateRegister(Register::A)),
            Opcode::new(0xea, "LD [a16], A".to_string(), 3, 4, AddressingMode::AddressU16, AddressingMode::ImmediateRegister(Register::A)),
            Opcode::new(0xf0, "LDH A, [a8]".to_string(), 2, 3, AddressingMode::ImmediateRegister(Register::A), AddressingMode::AddressHRAM),
            // Arithmetic Instructions
            Opcode::new(0x90, "SUB A, B".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::A), AddressingMode::ImmediateRegister(Register::B)),
            Opcode::new(0xbe, "CP A, [HL]".to_string(), 1, 2, AddressingMode::ImmediateRegister(Register::A), AddressingMode::AddressRegister(Register::HL)),
            Opcode::new(0xfe, "CP A, n8".to_string(), 2, 2, AddressingMode::ImmediateRegister(Register::A), AddressingMode::ImmediateU8),
            // Logic and Bit Instructions
            Opcode::new(0x17, "RLA".to_string(), 1, 1, AddressingMode::ImmediateRegister(Register::A), AddressingMode::None),
            Opcode::new(0xaf, "XOR A".to_string(), 1, 1, AddressingMode::None, AddressingMode::ImmediateRegister(Register::A)),
            // Jump/Call Instructions
            Opcode::new(0x18, "JR, e8".to_string(), 2, 3, AddressingMode::None, AddressingMode::ImmediateI8),
            Opcode::new(0x20, "JR NZ, e8".to_string(), 2, 2 /* + 1 if taken */, AddressingMode::None, AddressingMode::ImmediateI8),
            Opcode::new(0x28, "JR Z, e8".to_string(), 2, 2 /* + 1 if taken */, AddressingMode::None, AddressingMode::ImmediateI8),
            Opcode::new(0xc9, "RET".to_string(), 1, 4, AddressingMode::None, AddressingMode::None),
            Opcode::new(0xcd, "CALL a16".to_string(), 3 ,6, AddressingMode::AddressU16, AddressingMode::None),
            // Stack
            Opcode::new(0xc1, "POP BC".to_string(), 1, 4, AddressingMode::ImmediateRegister(Register::BC), AddressingMode::None),
            Opcode::new(0xc5, "PUSH BC".to_string(), 1, 4, AddressingMode::ImmediateRegister(Register::BC), AddressingMode::None),
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