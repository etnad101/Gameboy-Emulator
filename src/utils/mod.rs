pub trait GetBit {
    fn get_bit(&self, bit: u8) -> u8;
}

impl GetBit for u8 {
    fn get_bit(&self, bit: u8) -> u8 {
        (self & (1 << bit)) >> bit
    }
}
