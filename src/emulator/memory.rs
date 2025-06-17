use std::{fs, ops::Range};

use super::{
    cartridge::{Cartridge, MBC},
    errors::MemError,
};

pub struct MemoryBus {
    boot_rom: Vec<u8>,
    vram: Vec<u8>,
    ram: Vec<u8>,
    work_ram: Vec<u8>,
    oam: Vec<u8>,
    io_registers: Vec<u8>,
    hram: Vec<u8>,

    cartridge: Option<Cartridge>,

    boot_rom_active: bool,
    current_bank: usize,
}

impl MemoryBus {
    pub fn new() -> Result<Self, String> {
        let boot_rom = match fs::read("./DMG_ROM.bin") {
            Ok(rom) => rom,
            Err(_) => {
                return Err(
                    "Unable to read boot rom. Make sure DMG_ROM.bin is in root directory"
                        .to_string(),
                )
            }
        };

        Ok(MemoryBus {
            boot_rom,
            vram: vec![0xFF; 0x2000],
            ram: vec![0xFF; 0x2000],
            work_ram: vec![0xFF; 0x2000],
            oam: vec![0xFF; 0x00A0],
            io_registers: vec![0xFF; 0x0080],
            hram: vec![0xFF; 0x0080],

            cartridge: None,

            boot_rom_active: true,
            current_bank: 1,
        })
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    pub fn _clear(&mut self) {
        self.vram = vec![0xFF; 0x2000];
        self.ram = vec![0xFF; 0x2000];
        self.work_ram = vec![0xFF; 0x2000];
        self.oam = vec![0xFF; 0x00A0];
        self.io_registers = vec![0xFF; 0x0080];
        self.hram = vec![0xFF; 0x0080];
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        if self.boot_rom_active {
            if let 0x0000..=0x00FF = addr {
                return self.boot_rom[addr as usize];
            }
        };

        let cartridge = self.cartridge.as_ref().unwrap();

        match addr {
            0x0000..=0x7FFF => cartridge.read(addr),
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000],
            0xA000..=0xBFFF => self.ram[addr as usize - 0xA000],
            0xC000..=0xDFFF => self.work_ram[addr as usize - 0xC000],
            0xE000..=0xFDFF => self.work_ram[addr as usize - 0xE000],
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

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        let value = if addr == 0xff04 { 0 } else { value };
        if addr == 0xff50 {
            self.boot_rom_active = false;
        }

        match addr {
            0x0000..=0x7FFF => {
                let cartridge = self.cartridge.as_mut().unwrap();
                cartridge.write(addr, value);
            }
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000] = value,
            0xA000..=0xBFFF => self.ram[addr as usize - 0xA000] = value,
            0xC000..=0xDFFF => self.work_ram[addr as usize - 0xC000] = value,
            0xE000..=0xFDFF => self.work_ram[addr as usize - 0xE000] = value,
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = value,
            0xFEA0..=0xFEFF => (), // not useable range, refer to pandocs
            0xFF00..=0xFF7F => self.io_registers[addr as usize - 0xFF00] = value,
            0xFF80..=0xFFFF => self.hram[addr as usize - 0xFF80] = value,
            _ => panic!("Tried writing to illegal address {:#06x}", addr),
        }
    }

    pub fn get_range(&self, range: Range<u16>) -> Vec<u8> {
        range.into_iter().map(|i| self.read_u8(i)).collect()
    }
}
