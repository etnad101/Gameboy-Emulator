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
    bytes: Vec<u8>,
    title: String,
    gb_compatible: bool,
    mbc: Option<MBC>,
    ram: bool,
    battery: bool,
    timer: bool,
}

impl Cartridge {
    pub fn from(rom_path: &str) -> Result<Cartridge, Error> {
        println!("Looking for rom at '{}'", rom_path);
        let bytes = fs::read(rom_path)?;
        let cgb_flag = bytes[0x143];
        let (gb_compatible, title_bytes) = match cgb_flag {
            0x80 => {
                let title_bytes = &bytes[0x134..=0x142];
                let title_bytes = title_bytes.to_owned();
                (true, title_bytes)
            }
            0xC0 => {
                let title_bytes = &bytes[0x134..=0x142];
                let title_bytes = title_bytes.to_owned();
                (false, title_bytes)
            }
            _ => {
                let title_bytes = &bytes[0x134..=0x143];
                let title_bytes = title_bytes.to_owned();
                (true, title_bytes)
            }
        };

        let title = String::from_utf8(title_bytes.clone()).unwrap();

        println!("Found Rom: {}", title);

        let (mbc, ram, battery, timer) = match bytes[0x147] {
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
            _ => panic!("Cartrige type not implemented yet"),
        };
        dbg!(&mbc);

        Ok(Cartridge {
            bytes,
            title,
            gb_compatible,
            mbc,
            ram,
            battery,
            timer,
        })
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    pub fn gb_compatible(&self) -> bool {
        self.gb_compatible
    }

    pub fn mbc(&self) -> Option<MBC> {
        self.mbc.clone()
    }
}
