pub trait BinaryCodedDecimal {
    fn to_decimal(self) -> Self;
    fn to_binary(self) -> Self;
    fn is_valid_decimal(self) -> bool;
    fn fix_underflow(self) -> Self;
}

impl BinaryCodedDecimal for u8 {
    fn to_decimal(self) -> Self {
        ((self & 0xF0) >> 4) * 10 + (self & 0x0F)
    }

    fn to_binary(self) -> Self {
        (((self % 100) / 10) << 4) | (self % 10)
    }

    fn is_valid_decimal(self) -> bool {
        self < 100
    }

    fn fix_underflow(self) -> Self {
        if self.is_valid_decimal() {
            self
        } else {
            self.wrapping_add(100)
        }
    }
}


impl BinaryCodedDecimal for u16 {
    fn to_decimal(self) -> Self {
        ((self & 0xF000) >> 12) * 1000 + ((self & 0x0F00) >> 8) * 100 +
            ((self & 0x00F0) >> 4) * 10 + (self & 0x000F)
    }

    fn to_binary(self) -> Self {
        (((self % 10000) / 1000) << 12) |(((self % 1000) / 100) << 8) |
            (((self % 100) / 10) << 4) | (self % 10)
    }
    
    fn is_valid_decimal(self) -> bool {
        self < 10000
    }

    fn fix_underflow(self) -> Self {
        if self.is_valid_decimal() {
            self
        } else {
            self.wrapping_add(10000)
        }
    }
}
