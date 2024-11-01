use std::{error::Error, fs, ops::Range};

use super::{
    cartridge::{Cartridge, MBC},
    errors::{EmulatorError, MemError},
};

pub struct MemoryBus {
    size: u16,
    rom: Vec<u8>,
    switchable_rom: Vec<u8>,
    vram: Vec<u8>,
    ram: Vec<u8>,
    work_ram: Vec<u8>,
    echo_ram: Vec<u8>,
    oam: Vec<u8>,
    io_registers: Vec<u8>,
    hram: Vec<u8>,

    rom_banks: Vec<Vec<u8>>,
    ram_banks: Vec<Vec<u8>>,
    cartridge: Option<Cartridge>,

    current_bank: usize,
}

impl MemoryBus {
    pub fn new(size: u16) -> Self {
        MemoryBus {
            size,
            rom: vec![0xFF; 0x4000],
            switchable_rom: vec![0xFF; 0x4000],
            vram: vec![0xFF; 0x2000],
            ram: vec![0xFF; 0x2000],
            work_ram: vec![0xFF; 0x2000],
            echo_ram: vec![0xFF; 0x1E00],
            oam: vec![0xFF; 0x00A0],
            io_registers: vec![0xFF; 0x0060],
            hram: vec![0xFF; 0x0080],

            ram_banks: Vec::new(),
            rom_banks: Vec::new(),
            cartridge: None,

            current_bank: 1,
        }
    }

    pub fn load_rom(
        &mut self,
        boot_rom: bool,
        p_rom: Option<Cartridge>,
    ) -> Result<(), Box<dyn Error>> {
        if boot_rom {
            let boot_rom = fs::read("./DMG_ROM.bin")?;

            self.rom[0x0000..0x0100].copy_from_slice(&boot_rom[0x0000..0x0100]);
            return Ok(());
        }

        self.cartridge = match p_rom {
            Some(cart) => Some(cart),
            None => return Err(Box::new(EmulatorError::NoProgramRom)),
        };

        let cart = self.cartridge.as_ref().unwrap();

        match cart.mbc() {
            None => self.set_range(0x0100..0x8000, &cart.bytes()[0x0100..0x8000]),
            Some(MBC::MBC1) => {
                // TODO: figure out how many banks there are
                for i in 0..4 {
                    println!("creating bank");
                    let start = 0x4000 * i;
                    let end = start + 0x4000;
                    let mem_block = &cart.bytes()[start..end];
                    self.rom_banks.push(mem_block.to_vec());
                }
                self.set_range(0x0100..0x8000, &cart.bytes()[0x0100..0x8000]);
            }
            _ => (),
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.rom = vec![0xFF; 0x4000];
        self.switchable_rom = vec![0xFF; 0x4000];
        self.vram = vec![0xFF; 0x2000];
        self.ram = vec![0xFF; 0x2000];
        self.work_ram = vec![0xFF; 0x2000];
        self.echo_ram = vec![0xFF; 0x1E00];
        self.oam = vec![0xFF; 0x00A0];
        self.io_registers = vec![0xFF; 0x0060];
        self.hram = vec![0xFF; 0x0080];

        self.ram_banks = Vec::new();
        self.rom_banks = Vec::new();
    }

    fn unmap_boot_rom(&mut self) {
        let replacement = self.cartridge.as_ref().unwrap().bytes();
        self.set_range(0x0000..0x0100, &replacement[0..0x100]);
    }

    fn set_rom_bank(&mut self, bank_number: u8) {
        let bank_number = if bank_number == 0 {
            1
        } else {
            bank_number & 0x1F
        };

        self.rom_banks[self.current_bank] = self.switchable_rom.clone();
        self.switchable_rom = self.rom_banks[bank_number as usize].clone();
    }

    pub fn get_size(&self) -> u16 {
        self.size
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize],
            0x4000..=0x7FFF => self.switchable_rom[addr as usize - 0x4000],
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000],
            0xA000..=0xBFFF => self.ram[addr as usize - 0xA000],
            0xC000..=0xDFFF => self.work_ram[addr as usize - 0xC000],
            0xE000..=0xFDFF => self.echo_ram[addr as usize - 0xE000],
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00],
            0xFEA0..=0xFEFF => 0x00, // not useable range, refer to pandocs
            0xFF00..=0xFF7F => self.io_registers[addr as usize - 0xFF00],
            0xFF80..=0xFFFF => self.hram[addr as usize - 0xFF80],
        }
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr + 1) as u16;
        (hi << 8) | lo
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize] = value,
            0x4000..=0x7FFF => self.switchable_rom[addr as usize - 0x4000] = value,
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000] = value,
            0xA000..=0xBFFF => self.ram[addr as usize - 0xA000] = value,
            0xC000..=0xDFFF => self.work_ram[addr as usize - 0xC000] = value,
            0xE000..=0xFDFF => self.echo_ram[addr as usize - 0xE000] = value,
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = value,
            0xFEA0..=0xFEFF => (), // not useable range, refer to pandocs
            0xFF00..=0xFF7F => self.io_registers[addr as usize - 0xFF00] = value,
            0xFF80..=0xFFFF => self.hram[addr as usize - 0xFF80] = value,
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
            0xff50 => self.unmap_boot_rom(),
            _ => (),
        }

        match addr {
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000] = value,
            0xA000..=0xBFFF => self.ram[addr as usize - 0xA000] = value,
            0xC000..=0xDFFF => self.work_ram[addr as usize - 0xC000] = value,
            0xE000..=0xFDFF => self.echo_ram[addr as usize - 0xE000] = value,
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = value,
            0xFF00..=0xFF7F => self.io_registers[addr as usize - 0xFF00] = value,
            0xFF80..=0xFFFF => self.hram[addr as usize - 0xFF80] = value,
            _ => panic!("Tried writing to illegal address {}", addr),
        }
    }

    pub fn set_range(&mut self, range: Range<u16>, data: &[u8]) {
        assert!((range.len()) as usize == data.len(), "error");
        for i in range.clone().into_iter() {
            let index = (i - range.start) as usize;
            self.mem_write(i, data[index]);
        }
    }

    pub fn get_range(&self, range: Range<u16>) -> Result<Vec<u8>, MemError> {
        if range.end > self.size {
            return Err(MemError::OutOfRange);
        }

        let bytes: Vec<u8> = range.into_iter().map(|i| self.read_u8(i)).collect();

        Ok(bytes)
    }
}
