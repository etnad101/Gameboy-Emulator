use std::{error::Error, fs, ops::Range};

use super::{
    errors::{EmulatorError, MemError},
    cartridge::{Cartridge, MBC},
};

enum BlockType {
    Rom,
    Vram,
    Ram,
    Wram,
    EchoRam,
    Oam,
    NotUsable,
    IORegisters,
    Hram, // includes IE register (0xFFFF)
}

struct MemoryBlock {
    block_type: BlockType,
    data: Vec<u8>,
}

impl MemoryBlock {
    pub fn new(block_type: BlockType) -> Self {
        let size: usize = match block_type {
            BlockType::Rom => 0x4000,
            BlockType::Vram | BlockType::Ram => 0x2000,
            BlockType::Wram => 0x1000,
            BlockType::EchoRam => 0x1E00,
            BlockType::Oam => 0x00A0,
            BlockType::NotUsable => 0x0060,
            BlockType::IORegisters | BlockType::Hram => 0x80,

        };

        Self {
            block_type,
            data: vec![0xFF; size],
        }
    }

    pub fn get_val(&self, addr: u16) -> u8 {
        self.data[addr]
    }
}

pub struct MemoryBus {
    size: usize,
    memory: [MemoryBlock; 11],
    rom: Option<Cartridge>,
}

impl MemoryBus {
    pub fn new(size: usize) -> Self {
        let memory: [MemoryBlock; 11] = [
            MemoryBlock::new(BlockType::Rom),
            MemoryBlock::new(BlockType::Rom),
            MemoryBlock::new(BlockType::Vram),
            MemoryBlock::new(BlockType::Ram),
            MemoryBlock::new(BlockType::Wram),
            MemoryBlock::new(BlockType::Wram),
            MemoryBlock::new(BlockType::EchoRam),
            MemoryBlock::new(BlockType::Oam),
            MemoryBlock::new(BlockType::NotUsable),
            MemoryBlock::new(BlockType::IORegisters),
            MemoryBlock::new(BlockType::Hram),
        ];

        MemoryBus {
            size,
            memory,
            rom: None,
        }
    }

    pub fn load_rom(&mut self, boot_rom: bool, p_rom: Option<Cartridge>) -> Result<(), Box<dyn Error>> {
        if boot_rom {
            let boot_rom = fs::read("./DMG_ROM.bin")?;

            self.memory[0].data[0x0000..0x0100].copy_from_slice(&boot_rom[0x0000..0x0100]);
            return Ok(());
        } 

        self.rom = match p_rom {
            Some(rom) => Some(rom),
            None => return Err(Box::new(EmulatorError::NoProgramRom)),
        };

        let rom = self.rom.as_ref().unwrap();

        match rom.mbc() {
            None => self.memory[0x0100..0x8000].copy_from_slice(&rom.bytes()[0x0100..0x8000]),
            Some(MBC::MBC1) => {
                self.memory[0x0100..0x8000].copy_from_slice(&rom.bytes()[0x0100..0x8000])
            }
            _ => (),
        }

        Ok(())
    }

    fn unmap_boot_rom(&mut self) {
        let replacement = self.rom.as_ref().unwrap().bytes();
        self.memory[0..0x100].copy_from_slice(&replacement[0..0x100]);
    }

    fn set_rom_bank(&mut self, bank_number: u8) {
        let bank_number = if bank_number == 0 {
            1 
        } else {
            bank_number & 0x1F
        };

        let bytes = self.rom.as_ref().unwrap().bytes(); 
        let bank_addr: usize = bank_number as usize * 0x4000;
        self.memory[0x4000..0x8000].copy_from_slice(&bytes[bank_addr..bank_addr + 0x4000]);
    }

    pub fn clear(&mut self) {
        self.memory = vec![0xFF; self.size + 1];
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn get_range(&self, range: Range<usize>) -> Result<Vec<u8>, MemError> {
        if range.end > self.size {
            return Err(MemError::OutOfRange);
        }
        let bytes = self.memory[range].to_owned();

        Ok(bytes)
    }

    fn addr_to_block_addr(&self, addr: u16) -> (usize, u16) {
        match addr {
            0x0000..=0x3FFF => (0, addr),
            0x4000..=0x7FFF => (1, addr - 0x4000),
            0x8000..=0x9FFF => (2, addr - 0x8000),
            0xA000..=0xBFFF => (3, addr - 0xA000),
            0xC000..=0xCFFF => (4, addr - 0xC000),
            0xD000..=0xDFFF => (5, addr - 0xD000),
            0xE000..=0xFDFF => (6, addr - 0xE000),
            0xFE00..=0xFE9F => (7, addr - 0xFE00),
            0xFEA0..=0xFEFF => (8, addr - 0xFEA0),
            0xFF00..=0xFF7F => (9, addr - 0xFF00),
            0xFF80..=0xFFFF => (10, addr - 0xFF80),
        }
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

        self.memory[addr as usize] = value;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let (block, block_addr) = self.addr_to_block_addr(addr);
        let lo = self.memory[block].get_val(block_addr) as u16;
        let hi = self.memory[block].get_val(block_addr + 1) as u16;
        (hi << 8) | lo
    }
}
