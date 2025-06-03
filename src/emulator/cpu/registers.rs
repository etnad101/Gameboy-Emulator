use crate::utils::bit_ops::BitOps;

#[derive(Clone)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
}

impl Registers {
    pub fn new() -> Self {
        let (a, b, c, d, e, f, h, l) = (0, 0, 0, 0, 0, 0, 0, 0);
        Registers {
            a,
            b,
            c,
            d,
            e,
            f,
            h,
            l,
        }
    }

    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f = (value & 0xF0) as u8;
    }

    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
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

    pub fn get_z_flag(&self) -> u8 {
        self.f.get_bit(7)
    }

    pub fn set_n_flag(&mut self) {
        self.f.set_bit(6)
    }

    pub fn clear_n_flag(&mut self) {
        self.f.clear_bit(6)
    }

    pub fn get_n_flag(&self) -> u8 {
        self.f.get_bit(6)
    }

    pub fn set_h_flag(&mut self) {
        self.f.set_bit(5)
    }

    pub fn clear_h_flag(&mut self) {
        self.f.clear_bit(5)
    }

    pub fn get_h_flag(&self) -> u8 {
        self.f.get_bit(5)
    }

    pub fn set_c_flag(&mut self) {
        self.f.set_bit(4);
    }

    pub fn clear_c_flag(&mut self) {
        self.f.clear_bit(4)
    }

    pub fn get_c_flag(&self) -> u8 {
        self.f.get_bit(4)
    }
}
