use crate::utils::bit_ops::BitOps;

#[derive(Clone, Default)]
pub struct CpuState {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn af(&self) -> u16 {
        (u16::from(self.a) << 8) | u16::from(self.f)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f = (value & 0xF0) as u8;
    }

    pub fn bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    pub fn set_z_flag(&mut self) {
        self.f.set_bit(7);
    }

    pub fn clear_z_flag(&mut self) {
        self.f.clear_bit(7);
    }

    pub fn set_z_flag_from_value(&mut self, val: u8) {
        if val == 0 {
            self.set_z_flag();
        } else {
            self.clear_z_flag();
        }
    }

    pub fn get_z_flag(&self) -> u8 {
        self.f.get_bit(7)
    }

    pub fn set_n_flag(&mut self) {
        self.f.set_bit(6);
    }

    pub fn clear_n_flag(&mut self) {
        self.f.clear_bit(6);
    }

    pub fn get_n_flag(&self) -> u8 {
        self.f.get_bit(6)
    }

    pub fn set_h_flag(&mut self) {
        self.f.set_bit(5);
    }

    pub fn clear_h_flag(&mut self) {
        self.f.clear_bit(5);
    }

    pub fn get_h_flag(&self) -> u8 {
        self.f.get_bit(5)
    }

    pub fn set_c_flag(&mut self) {
        self.f.set_bit(4);
    }

    pub fn clear_c_flag(&mut self) {
        self.f.clear_bit(4);
    }

    pub fn get_c_flag(&self) -> u8 {
        self.f.get_bit(4)
    }
}
