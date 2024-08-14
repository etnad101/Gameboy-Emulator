pub trait BitOps {
    fn get_bit(&self, bit: u8) -> u8;
    fn set_bit(&mut self, bit: u8);
    fn clear_bit(&mut self, bit: u8);
}

impl BitOps for u8 {
    fn get_bit(&self, bit: u8) -> u8 {
        (self & (1 << bit)) >> bit
    }

    fn set_bit(&mut self, bit: u8) {
        *self |= 1 << bit
    }

    fn clear_bit(&mut self, bit: u8) {
        *self &= !(1 << bit)
    }
}
