use std::{error::Error, fs, ops::Range};

use super::{
    errors::{EmulatorError, MemError},
    rom::{Rom, MBC},
};

pub struct MemoryBus {
    size: usize,
    bytes: Vec<u8>,
    rom: Option<Rom>,
}

impl MemoryBus {
    pub fn new(size: usize) -> Self {
        let memory: Vec<u8> = vec![0xFF; size + 1];

        MemoryBus {
            size,
            bytes: memory,
            rom: None,
        }
    }

    pub fn load_rom(&mut self, boot_rom: bool, p_rom: Option<Rom>) -> Result<(), Box<dyn Error>> {
        if boot_rom {
            let boot_rom = fs::read("./DMG_ROM.bin")?;

            self.bytes[0x0000..0x0100].copy_from_slice(&boot_rom[0x0000..0x0100]);
            return Ok(());
        } 

        self.rom = match p_rom {
            Some(rom) => Some(rom),
            None => return Err(Box::new(EmulatorError::NoProgramRom)),
        };

        let rom = self.rom.as_ref().unwrap();

        match rom.mbc() {
            None => self.bytes[0x0100..0x8000].copy_from_slice(&rom.bytes()[0x0100..0x8000]),
            Some(MBC::MBC1) => {
                self.bytes[0x0100..0x8000].copy_from_slice(&rom.bytes()[0x0100..0x8000])
            }
            _ => (),
        }

        Ok(())
    }

    fn unmap_boot_rom(&mut self) {
        let replacement = self.rom.as_ref().unwrap().bytes();
        self.bytes[0..0x100].copy_from_slice(&replacement[0..0x100]);
    }

    fn set_rom_bank(&mut self, bank_number: u8) {
        let bank_number = if bank_number == 0 {
            1 
        } else {
            bank_number & 0x1F
        };

        println!("switching to bank: {}", bank_number);
        let bytes = self.rom.as_ref().unwrap().bytes(); 
        let bank_addr: usize = bank_number as usize * 0x4000;
        self.bytes[0x4000..0x8000].copy_from_slice(&bytes[bank_addr..bank_addr + 0x4000]);
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
        match addr {
            0x2000..=0x3FFF => {
                self.set_rom_bank(value);
                return;
            }
            0xff04 => value = 0,
            0xff50 => {
                self.unmap_boot_rom()
            }
            _ => (),
        }

        self.bytes[addr as usize] = value;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.bytes[addr as usize] as u16;
        let hi = self.bytes[(addr + 1) as usize] as u16;
        (hi << 8) | lo
    }
}
