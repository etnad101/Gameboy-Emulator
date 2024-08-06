use std::fmt;

#[derive(Debug)]
pub enum MemError {
    OutOfRange,
}

impl fmt::Display for MemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemError::OutOfRange => write!(f, "requested memory range does not exist"),
        }
    }
}

impl std::error::Error for MemError {}

#[derive(Debug)]
pub enum CpuError {
    OpcodeNotImplemented(u8, bool),
    UnrecognizedOpcode(u8, bool),
    OpcodeError(String),
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuError::OpcodeNotImplemented(code, prefixed) => {
                if prefixed.to_owned() {
                    write!(
                        f,
                        "CPU_ERROR: Prefixed Opcode {:#04x} not implemented yet",
                        code
                    )
                } else {
                    write!(
                        f,
                        "CPU_ERROR: Normal Opcode {:#04x} not implemented yet",
                        code
                    )
                }
            }
            CpuError::UnrecognizedOpcode(code, prefixed) => {
                if prefixed.to_owned() {
                    write!(
                        f,
                        "CPU_ERROR: Prefixed Opcode {:#04x} not found in opcode map",
                        code
                    )
                } else {
                    write!(
                        f,
                        "CPU_ERROR: Normal Opcode {:#04x} not found in opcode map",
                        code
                    )
                }
            }
            CpuError::OpcodeError(msg) => write!(f, "CPU_ERROR: {}", msg),
        }
    }
}

impl std::error::Error for CpuError {}

#[derive(Debug)]
pub enum EmulatorError {
    IncompatibleRom,
    NoProgramRom,
}

impl fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmulatorError::IncompatibleRom => write!(f, "Selected rom is incompatible"),
            EmulatorError::NoProgramRom => write!(f, "No rom was given to the emulator"),
        }
    }
}

impl std::error::Error for EmulatorError {}
