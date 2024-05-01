use crate::cpu::registers::Registers;

const MEM_SIZE: usize = 0xFFFF;

const MAX_CYCLES: usize = 69905;

enum TargetRegister {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HL
}

struct MemoryBus {
    memory: [u8; MEM_SIZE],
}

impl MemoryBus {
    pub fn new() -> Self {
        let path = "./DMG_ROM.bin";
        let boot_rom = std::fs::read(path).unwrap();

        let mut memory =  [0; MEM_SIZE];
        memory[0..boot_rom.len()].copy_from_slice(&boot_rom);
        MemoryBus { memory }
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.memory[(addr) as usize] as u16;
        let hi = self.memory[(addr + 1) as usize] as u16;
        (hi << 8) | lo
    }
}

pub struct CPU {
    memory: MemoryBus,
    reg: Registers,
    sp: u16,
    pc: u16,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            memory: MemoryBus::new(),
            reg: Registers::new(),
            sp: 0,
            pc: 0,
        }
    }

    pub fn update(&mut self) {
        let mut cycles_this_frame = 0;

        while cycles_this_frame < MAX_CYCLES as u32 {
            let cycles = self.execute_next_opcode();

            cycles_this_frame += cycles;

            // self.update_timers(cycles);

            // self.update_graphics(cycles);

            // self.do_interupts();
        }

        // self.render_screen();
    }

    fn execute_next_opcode(&mut self) -> u32 {
        let opcode = self.memory.read_u8(self.pc);

        let cycles = match opcode {
            0x31 => self.load_sp(),
            0xaf => self.xor_with_a(TargetRegister::A),
            _ => {
                println!("Unknown opcode: {:#04x}", opcode);
                println!("PC: {:#06x}", self.pc);
                println!("SP: {:#06x}", self.sp);
                panic!()
            },
        };

        cycles
    }

    fn update_timers(&self, cycles: u32) {
        todo!()
    }

    fn update_graphics(&self, cycles: u32) {
        todo!()
    }

    fn do_interupts(&self) {
        todo!()
    }

    fn render_screen(&self) {
        todo!()
    }

    fn load_sp(&mut self) -> u32 {
        self.sp = self.memory.read_u16(self.pc + 1);
        self.pc += 3;
        3
    }

    fn xor_with_a(&mut self, target_register: TargetRegister) -> u32 {
        self.pc += 1;

        let (value, cycles) = match target_register {
            TargetRegister::A => (self.reg.a, 1),
            _ => todo!("Xor register not implemented")
        };

        let res = value ^ self.reg.a;

        if res == 0 {
            self.reg.set_z()
        }

        cycles
    }
}
