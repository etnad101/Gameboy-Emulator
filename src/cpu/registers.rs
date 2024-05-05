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

    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    pub fn set_z_flag(&mut self) {
        self.f |= 0b1000_0000;
    }

    pub fn clear_z_flag(&mut self) {
        self.f &= 0b0111_1111;
    }

    pub fn check_z_flag(&self) -> bool {
        self.f & 0b1000_0000 > 0
    }

    pub fn set_n_flag(&mut self) {
        self.f |= 0b0100_0000;
    }

    pub fn clear_n_flag(&mut self) {
        self.f &= 0b1011_1111;
    }

    pub fn check_n_flag(&self) -> bool {
        self.f & 0b0100_0000 > 0
    }

    pub fn set_h_flag(&mut self) {
        self.f |= 0b0010_0000;
    }

    pub fn clear_h_flag(&mut self) {
        self.f &= 0b1101_1111;
    }
    
    pub fn check_h_flag(&self) -> bool {
        self.f & 0b0010_0000 > 0
    }

    pub fn set_c_flag(&mut self) {
        self.f |= 0b0001_0000;
    }

    pub fn clear_c_flag(&mut self) {
        self.f &= 0b1110_1111;
    }
    
    pub fn check_c_flag(&self) -> bool {
        self.f & 0b0001_0000 > 0
    }
}
