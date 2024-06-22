pub struct MemoryBus {
    size: usize,
    bytes: Vec<u8>,
}

impl MemoryBus {
    pub fn new(size: usize) -> Self {
        let mut memory: Vec<u8> = vec![0xFF; size + 1];
        let path: &str = "./DMG_ROM.bin";
        let boot_rom: Vec<u8> = std::fs::read(path).unwrap();

        memory[0..boot_rom.len()].copy_from_slice(&boot_rom);
        
        MemoryBus { size, bytes: memory }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        // Temporary size limit until I setup MBCs, so I can load a rom to get the boot screen
        let mut len = if rom.len() > 0x200 {
            0x200
        } else {
            rom.len()
        };
        len += 0x100;

        self.bytes[0x100..len].copy_from_slice(&rom[0x100..len]);
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.bytes[addr as usize]
    }

    pub fn write_u8(&mut self, addr: u16, value: u8) {
        // TODO: implement Echo RAM and range checks
        let mut value = value;
        if addr == 0xFF04 {
            value = 0;
        }
        self.bytes[addr as usize] = value;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.bytes[(addr) as usize] as u16;
        let hi = self.bytes[(addr + 1) as usize] as u16;
        (hi << 8) | lo
    }
}