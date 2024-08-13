use std::{error::Error, fs, ops::Range};

use super::errors::{EmulatorError, MemError};

pub struct MemoryBus {
    size: usize,
    bytes: Vec<u8>,
    cutoff_rom: Vec<u8>,
}

impl MemoryBus {
    pub fn new(size: usize) -> Self {
        let memory: Vec<u8> = vec![0xFF; size + 1];

        MemoryBus {
            size,
            bytes: memory,
            cutoff_rom: Vec::new(),
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
                None => return Err(Box::new(EmulatorError::NoProgramRom)),
            }
        };

        let rom_size = rom.len();
        println!("rom size: {:#06x}", rom_size);

        let mut start_addr = 0;

        if !boot_rom {
            start_addr = 0x100;
            self.cutoff_rom = rom[0..0x100].to_owned();
        }
        self.bytes[start_addr..rom_size].copy_from_slice(&rom[start_addr..rom_size]);

        Ok(())
    }

    fn unmap_boot_rom(&mut self) {
        self.bytes[0..0x100].copy_from_slice(&self.cutoff_rom[0..0x100]);
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
            return Err(MemError::OutOfRange);
        }
        let bytes = self.bytes[range].to_owned();

        Ok(bytes)
    }

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        let mut value = value;
        if addr == 0xff50 {
            self.unmap_boot_rom()
        }
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
