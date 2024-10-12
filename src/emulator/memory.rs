use std::{error::Error, fs, mem, ops::Range};

use super::{
    errors::{EmulatorError, MemError},
    cartridge::{Cartridge, MBC},
};

#[derive(Clone)]
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

#[derive(Clone)]
struct MemoryBlock {
    block_type: BlockType,
    data: Vec<u8>,
    size: u16,
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
            size: size.clone() as u16,
        }
    }

    pub fn from(block_type: BlockType, data: &[u8]) -> Self {
        let mut block = MemoryBlock::new(block_type);        
        if block.size as usize != data.len() {
            panic!("Sizes do not match while constructing MemoryBlock");
        }

        for addr in 0..block.size {
            block.write(addr, data[addr as usize]);
        }

        block
    }

    pub fn clear(&mut self) {
        self.data = vec![0xFF; self.size as usize];
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.data[addr as usize] = value;
    }

    pub fn size(&self) -> u16 {
        self.size
    }
}

pub struct MemoryBus {
    size: u16,
    memory: [Box<MemoryBlock>; 11],
    rom_banks: Vec<Box<MemoryBlock>>,
    rom: Option<Cartridge>,
}

impl MemoryBus {
    pub fn new(size: u16) -> Self {
        let memory: [Box<MemoryBlock>; 11] = [
            Box::new(MemoryBlock::new(BlockType::Rom)),
            Box::new(MemoryBlock::new(BlockType::Rom)),
            Box::new(MemoryBlock::new(BlockType::Vram)),
            Box::new(MemoryBlock::new(BlockType::Ram)),
            Box::new(MemoryBlock::new(BlockType::Wram)),
            Box::new(MemoryBlock::new(BlockType::Wram)),
            Box::new(MemoryBlock::new(BlockType::EchoRam)),
            Box::new(MemoryBlock::new(BlockType::Oam)),
            Box::new(MemoryBlock::new(BlockType::NotUsable)),
            Box::new(MemoryBlock::new(BlockType::IORegisters)),
            Box::new(MemoryBlock::new(BlockType::Hram)),
        ];

        MemoryBus {
            size,
            memory,
            rom_banks: Vec::new(),
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
            None => self.load_range(0x0100..0x8000, &rom.bytes()[0x0100..0x8000]),
            Some(MBC::MBC1) => {
                // TODO: figure out how many banks there are
                for i in 0..4 {
                    println!("creating bank");
                    let start = 0x4000 * i;
                    let end = start + 0x4000;
                    let mem_block = Box::new(MemoryBlock::from(BlockType::Rom, &rom.bytes()[start..end]));
                    self.rom_banks.push(mem_block);
                }
                self.load_range(0x0100..0x8000, &rom.bytes()[0x0100..0x8000]);
            },
            _ => (),
        }

        Ok(())
    }

    fn unmap_boot_rom(&mut self) {
        let replacement = self.rom.as_ref().unwrap().bytes();
        self.load_range(0x0000..0x0100, &replacement[0..0x100]);
    }

    fn set_rom_bank(&mut self, bank_number: u8) {
        let bank_number = if bank_number == 0 {
            1 
        } else {
            bank_number & 0x1F
        };

        self.memory[1] = self.rom_banks[bank_number as usize].clone();
    }

    pub fn clear(&mut self) {
        for block in &mut self.memory {
            block.clear();
        }
    }

    pub fn get_size(&self) -> u16 {
        self.size
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

    pub fn read_u8(&self, addr: u16) -> u8 {
        let (block, addr) = self.addr_to_block_addr(addr);
        self.memory[block].read(addr)
    }

    pub fn load_range(&mut self, range: Range<u16>, data: &[u8]) {
        assert!((range.end - range.start) as usize == data.len(), "error");
        let (mut block, mut addr) = self.addr_to_block_addr(range.start);
        for value in data {
            self.memory[block].write(addr, value.to_owned());
            addr += 1;
            if addr >= self.memory[block].size() {
               block += 1; 
               addr = 0;
            }
        }
    }

    pub fn get_range(&self, range: Range<u16>) -> Result<Vec<u8>, MemError> {
        if range.end > self.size {
            return Err(MemError::OutOfRange);
        }
        let (mut block, mut addr) = self.addr_to_block_addr(range.start);
        let mut bytes: Vec<u8> = Vec::new();
        for _ in range {
            bytes.push(self.memory[block].read(addr));
            addr += 1;
            if addr >= self.memory[block].size() {
               block += 1; 
               addr = 0;
            }
        }

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
            0xff50 => self.unmap_boot_rom(),
            _ => (),
        }
        let (block, addr) = self.addr_to_block_addr(addr);
        self.memory[block].write(addr, value);
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let (block, block_addr) = self.addr_to_block_addr(addr);
        let lo = self.memory[block].read(block_addr) as u16;
        let hi = self.memory[block].read(block_addr + 1) as u16;
        (hi << 8) | lo
    }
}
