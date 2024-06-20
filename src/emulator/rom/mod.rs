use std::{fs, io::Error};

pub struct Rom {
    bytes: Vec<u8>,
    gb_compatible: bool,
}
   


impl Rom {
    pub fn from(rom_path: &str) -> Result<Rom, Error> {
        let bytes = fs::read(rom_path)?;

        let gb_compatible = bytes[0x143] == 0x80;

        Ok(Rom { bytes, gb_compatible })
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}