use std::{fs, ops::Range};

use super::{
    cartridge::{Cartridge, MBC},
    errors::MemError,
};

pub struct MemoryBus {
    size: u16,
    boot_rom: Vec<u8>,
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

    boot_rom_active: bool,
    current_bank: usize,
}

impl MemoryBus {
    pub fn new(size: u16) -> Self {
        let boot_rom = fs::read("./DMG_ROM.bin").unwrap();
        println!("boot rom size {}", boot_rom.len());
        MemoryBus {
            size,
            boot_rom,
            rom: vec![0xFF; 0x4000],
            switchable_rom: vec![0xFF; 0x4000],
            vram: vec![0xFF; 0x2000],
            ram: vec![0xFF; 0x2000],
            work_ram: vec![0xFF; 0x2000],
            echo_ram: vec![0xFF; 0x1E00],
            oam: vec![0xFF; 0x00A0],
            io_registers: vec![0xFF; 0x0080],
            hram: vec![0xFF; 0x0080],

            ram_banks: Vec::new(),
            rom_banks: Vec::new(),
            cartridge: None,

            boot_rom_active: true,
            current_bank: 1,
        }
    }

    pub fn load_rom(&mut self, p_rom: Cartridge) {
        dbg!(p_rom.title(), p_rom.mbc(), p_rom.rom_banks());
        match p_rom.mbc() {
            None => self.set_range(0x0000..0x8000, &p_rom.bytes()[0x0000..0x8000]),
            Some(MBC::MBC1) => {
                for i in 0..p_rom.rom_banks() {
                    println!("creating bank");
                    let start = 0x4000 * i;
                    let end = start + 0x4000;
                    let mem_block = &p_rom.bytes()[start..end];
                    self.rom_banks.push(mem_block.to_vec());
                }
                self.set_range(0x0000..0x8000, &p_rom.bytes()[0x0000..0x8000]);
                println!("rom_banks created: {}", self.rom_banks.len());
            }
            _ => println!("MBC Not supported yet"),
        }

        self.cartridge = Some(p_rom);
    }

    pub fn _clear(&mut self) {
        self.rom = vec![0xFF; 0x4000];
        self.switchable_rom = vec![0xFF; 0x4000];
        self.vram = vec![0xFF; 0x2000];
        self.ram = vec![0xFF; 0x2000];
        self.work_ram = vec![0xFF; 0x2000];
        self.echo_ram = vec![0xFF; 0x1E00];
        self.oam = vec![0xFF; 0x00A0];
        self.io_registers = vec![0xFF; 0x0080];
        self.hram = vec![0xFF; 0x0080];

        self.ram_banks = Vec::new();
        self.rom_banks = Vec::new();
    }

    fn set_rom_bank(&mut self, bank_number: u8) {
        if self.cartridge.as_ref().unwrap().mbc().is_none() {
            return;
        }
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
        if self.boot_rom_active {
            if let 0x0000..=0x00FF = addr {
                return self.boot_rom[addr as usize];
            }
        };
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

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        let mut value = value;
        match addr {
            0x2000..=0x3FFF => {
                println!("Changing rom bank by writing to addr: {:#06x}", addr);
                self.set_rom_bank(value);
                return;
            }
            0xff04 => value = 0,
            0xff50 => self.boot_rom_active = false,
            _ => (),
        }

        match addr {
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000] = value,
            0xA000..=0xBFFF => self.ram[addr as usize - 0xA000] = value,
            0xC000..=0xDFFF => self.work_ram[addr as usize - 0xC000] = value,
            0xE000..=0xFDFF => self.echo_ram[addr as usize - 0xE000] = value,
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = value,
            0xFEA0..=0xFEFF => (), // not useable range, refer to pandocs
            0xFF00..=0xFF7F => self.io_registers[addr as usize - 0xFF00] = value,
            0xFF80..=0xFFFF => self.hram[addr as usize - 0xFF80] = value,
            _ => panic!("Tried writing to illegal address {:#06x}", addr),
        }
    }

    pub fn set_range(&mut self, range: Range<u16>, data: &[u8]) {
        assert!((range.len()) == data.len(), "error");
        for addr in range.clone() {
            let index = (addr - range.start) as usize;
            let value = data[index];

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
    }

    pub fn get_range(&self, range: Range<u16>) -> Result<Vec<u8>, MemError> {
        if range.end > self.size {
            return Err(MemError::OutOfRange);
        }

        let bytes: Vec<u8> = range.into_iter().map(|i| self.read_u8(i)).collect();

        Ok(bytes)
    }
}
