use std::fmt;

#[derive(Debug)]
pub enum EmulatorError {
    IncompatableRom
}

impl fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       match self {
            EmulatorError::IncompatableRom => write!(f, "Selected rom is incompatable"),
       } 
    }
}

impl std::error::Error for EmulatorError {}