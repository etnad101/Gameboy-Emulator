use std::{error::Error, fs, ops::Range};

use super::errors::{EmulatorError, MemError};

pub struct MemoryBus {
    size: usize,
    bytes: Vec<u8>,
}

impl MemoryBus {
    pub fn new(size: usize) -> Self {
        let mut memory: Vec<u8> = vec![0xFF; size + 1];

        MemoryBus {
            size,
            bytes: memory,
        }
    }

    pub fn load_rom(
        &mut self,
        boot_rom: bool,
        p_rom: Option<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        let rom: Vec<u8> = if boot_rom {
            let path = "./DMG_ROM.bin";
            fs::read(path)?
        } else {
            match p_rom {
                Some(rom) => rom,
                None => return Err(Box::new(EmulatorError::NoPrgmRom)),
            }
        };

        // Temporary size limit until I set up MBCs, so I can load a rom to get the boot screen
        let mut len = if rom.len() > 0x200 { 0x200 } else { rom.len() };

        let mut start_addr: usize = 0;
        if !boot_rom {
            start_addr = 0x100;
            len += 0x100;
        }

        self.bytes[start_addr..len].copy_from_slice(&rom[start_addr..len]);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.bytes = vec![0xFF; self.size + 1];
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.bytes[addr as usize]
    }

    pub fn get_range(&self, range: Range<usize>) -> Result<Vec<u8>, MemError> {
        if range.end > self.size {
            return Err(MemError::OutOfRange)
        }
        let bytes = self.bytes[range].to_owned();

        Ok(bytes)
    }

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        let mut value = value;
        if addr == 0xFF04 {
            value = 0;
        }
        self.bytes[addr as usize] = value;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.bytes[addr as usize] as u16;
        let hi = self.bytes[(addr + 1) as usize] as u16;
        (hi << 8) | lo
    }
}
