
pub enum LCDRegister {
    LCDC = 0xFF40,
    STAT = 0xff41,
    SCY = 0xff42,
    SCX = 0xff43,
    LY = 0xff44,
    LYC = 0xff45,
    DMA = 0xff46,
    BGP = 0xff47,
    OBP0 = 0xff48,
    OBP1 = 0xff49,
}

pub struct MemoryBus {
    size: usize,
    memory: Vec<u8>,
}

impl MemoryBus {
    pub fn new(size: usize) -> Self {
        let path: &str = "./DMG_ROM.bin";
        let boot_rom: Vec<u8> = std::fs::read(path).unwrap();

        let mut memory: Vec<u8> = vec![0; size + 1];

        memory[0..boot_rom.len()].copy_from_slice(&boot_rom);
        MemoryBus { size, memory }
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        self.memory[addr as usize] = value;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.memory[(addr) as usize] as u16;
        let hi = self.memory[(addr + 1) as usize] as u16;
        (hi << 8) | lo
    }
}