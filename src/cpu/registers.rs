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
        (self.h as u16) << 8 | (self.h as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    pub fn set_z_flag(&mut self) {
        self.f |= 0b0000_0001;
    }

    pub fn clear_z_flag(&mut self) {
        self.f &= 0b1111_1110;
    }

    pub fn check_z_flag(&self) {}

    pub fn set_n_flag(&mut self) {
        self.f |= 0b0000_0010;
    }

    pub fn clear_n_flag(&mut self) {
        self.f &= 0b1111_1101;
    }

    pub fn set_h_flag(&mut self) {
        self.f |= 0b0000_0100;
    }

    pub fn clear_h_flag(&mut self) {
        self.f &= 0b1111_1011;
    }

    pub fn set_c_flag(&mut self) {
        self.f |= 0b0000_1000;
    }

    pub fn clear_c_flag(&mut self) {
        self.f &= 0b1111_0111;
    }
}
