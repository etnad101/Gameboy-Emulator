use core::fmt;

#[derive(Debug)]
pub enum DrawOutOfBoundsError {
    X(usize),
    Y(usize), 
}

impl fmt::Display for DrawOutOfBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawOutOfBoundsError::X(val) => write!(f, "Tried drawing out of bound at x = {}", val),
            DrawOutOfBoundsError::Y(val) => write!(f, "Tried drawing out of bound at y = {}", val),
        }
    }
}

impl std::error::Error for DrawOutOfBoundsError {}