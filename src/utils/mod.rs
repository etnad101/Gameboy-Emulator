pub trait BitOps<T> {
    fn get_bit(&self, bit: T) -> T;
    fn set_bit(&mut self, bit: T);
    fn clear_bit(&mut self, bit: T);
}

impl BitOps<u8> for u8 {
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

impl BitOps<i8> for i8 {
    fn get_bit(&self, bit: i8) -> i8 {
        (self & (1 << bit)) >> bit
    }

    fn set_bit(&mut self, bit: i8) {
        *self |= 1 << bit
    }

    fn clear_bit(&mut self, bit: i8) {
        *self &= !(1 << bit)
    }
}
