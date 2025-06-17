use std::{fs, io::Error};

#[derive(Debug, Clone)]
pub(super) enum MBC {
    MBC1,
    MBC2,
    MBC3,
    MBC5,
    MBC6,
    MBC7,
    MMM01,
    M161,
    HuC1,
    HuC3,
}

pub struct Cartridge {
    // Cartridge header information
    title: String,
    gb_compatible: bool,
    mbc: Option<MBC>,
    ram: bool,
    battery: bool,
    timer: bool,

    // catridge ram and rom
    fixed_rom_bank: Vec<u8>,
    switchable_banks: Vec<Vec<u8>>,
    current_bank: usize,
}

impl Cartridge {
    pub fn from(rom_path: &str) -> Result<Cartridge, Error> {
        println!("Looking for rom at '{}'", rom_path);
        let raw_file = fs::read(rom_path)?;
        let cgb_flag = raw_file[0x143];
        let (gb_compatible, title_bytes) = match cgb_flag {
            0x80 => {
                let title_bytes = &raw_file[0x134..=0x142];
                let title_bytes = title_bytes.to_owned();
                (true, title_bytes)
            }
            0xC0 => {
                let title_bytes = &raw_file[0x134..=0x142];
                let title_bytes = title_bytes.to_owned();
                (false, title_bytes)
            }
            _ => {
                let title_bytes = &raw_file[0x134..=0x143];
                let title_bytes = title_bytes.to_owned();
                (true, title_bytes)
            }
        };

        let title = String::from_utf8(title_bytes.clone()).unwrap();

        println!("Found Rom: {}", title);

        let (mbc, ram, battery, timer) = match raw_file[0x147] {
            0x00 => (None, false, false, false),
            0x01 => (Some(MBC::MBC1), false, false, false),
            0x02 => (Some(MBC::MBC1), true, false, false),
            0x03 => (Some(MBC::MBC1), true, true, false),
            0x05 => (Some(MBC::MBC2), false, false, false),
            0x06 => (Some(MBC::MBC2), false, true, false),
            0x08 => (None, true, false, false),
            0x09 => (None, true, true, false),
            0x0b => (Some(MBC::MMM01), false, false, false),
            0x0c => (Some(MBC::MMM01), true, false, false),
            0x0d => (Some(MBC::MMM01), true, true, false),
            0x0f => (Some(MBC::MBC3), false, true, true),
            0x10 => (Some(MBC::MBC3), true, true, true),
            0x11 => (Some(MBC::MBC3), false, false, false),
            0x12 => (Some(MBC::MBC3), true, false, false),
            0x13 => (Some(MBC::MBC3), true, true, false),
            0x19 => (Some(MBC::MBC5), false, false, false),
            0x1a => (Some(MBC::MBC5), true, false, false),
            0x1b => (Some(MBC::MBC5), true, true, false),
            0x20 => (Some(MBC::MBC6), false, false, false),
            0xfe => (Some(MBC::HuC3), false, false, false),
            0xff => (Some(MBC::HuC1), true, true, false),
            _ => panic!("Cartridge type not implemented yet"),
        };

        let rom_banks = match raw_file[0x148] {
            0x00..=0x08 => {
                let base: usize = 2;
                base.pow(raw_file[0x148] as u32)
            }
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => panic!("No other rom sizes"),
        };

        let fixed_rom_bank: Vec<u8> = raw_file[0x0000..0x4000].to_vec();
        let mut switchable_banks: Vec<Vec<u8>> = Vec::new();

        match mbc {
            None => {
                switchable_banks.push(raw_file[0x4000..0x8000].to_vec());
            }
            Some(MBC::MBC1) => {
                for i in 0..rom_banks {
                    println!("creating bank");
                    let start = 0x4000 * i;
                    let end = start + 0x4000;
                    let bank: &[u8] = &raw_file[start..end];
                    switchable_banks.push(bank.to_vec());
                }
                println!("rom_banks created: {}", switchable_banks.len());
            }
            _ => println!("MBC Not supported yet"),
        }

        Ok(Cartridge {
            title,
            gb_compatible,
            mbc,
            ram,
            battery,
            timer,

            fixed_rom_bank,
            switchable_banks,
            current_bank: 0,
        })
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.fixed_rom_bank.clone()
    }

    pub fn gb_compatible(&self) -> bool {
        self.gb_compatible
    }

    pub(super) fn mbc(&self) -> Option<MBC> {
        self.mbc.clone()
    }

    pub fn read(&self, addr: u16) -> u8 {
        if addr < 0x4000 {
            self.fixed_rom_bank[addr as usize]
        } else {
            self.switchable_banks[self.current_bank][addr as usize - 0x4000]
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        if let 0x2000..=0x3FFF = addr {
            println!("Changing rom bank by writing {value:#04x} to addr: {addr:#06x}");
            self.set_rom_bank(value);
        }
    }

    pub fn set_rom_bank(&mut self, bank_number: u8) {
        let bank_number = if bank_number == 0 {
            1
        } else {
            bank_number & 0x1f
        };
        self.current_bank = (bank_number - 1) as usize;
    }
}
