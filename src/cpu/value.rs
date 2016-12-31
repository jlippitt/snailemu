use hardware::MemoryAccess;
use std::fmt::{Debug, Display, LowerHex, UpperHex};
use std::ops::{BitAnd, BitOr, BitXor, Not};
use util::byte_access::ByteAccess;

pub trait Value : MemoryAccess +
    Copy + Clone + Default + Eq + PartialEq + Ord + PartialOrd +
    Display + Debug + UpperHex + LowerHex +
    BitAnd<Output=Self> + BitOr<Output=Self> + BitXor<Output=Self> + Not<Output=Self> +
    From<u8>
{
    fn from_modal(modal: u16) -> Self;
    fn to_modal(&self, modal: &mut u16);
    fn from_bool(value: bool) -> Self;
    fn is_zero(&self) -> bool;
    fn is_overflow(&self) -> bool;
    fn is_negative(&self) -> bool;
    fn add_value(self, rhs: Self) -> Self;
    fn subtract_value(self, rhs: Self) -> Self;
    fn left_shift_value(self) -> (Self, bool);
    fn right_shift_value(self) -> (Self, bool);
    fn left_rotate_value(self, carry: bool) -> (Self, bool);
    fn right_rotate_value(self, carry: bool) -> (Self, bool);
}

impl Value for u8 {
    fn from_modal(modal: u16) -> Self {
        modal as Self
    }

    fn to_modal(&self, modal: &mut u16) {
        modal.set_lower(*self);
    }

    fn from_bool(value: bool) -> Self {
        value as Self
    }

    fn is_zero(&self) -> bool {
        *self == 0
    }

    fn is_overflow(&self) -> bool {
        *self & 0x40 != 0
    }

    fn is_negative(&self) -> bool {
        *self & 0x80 != 0
    }

    fn add_value(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }

    fn subtract_value(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }

    fn left_shift_value(self) -> (Self, bool) {
        (self.wrapping_shl(1), self & 0x80 != 0)
    }

    fn right_shift_value(self) -> (Self, bool) {
        (self.wrapping_shr(1), self & 0x01 != 0)
    }

    fn left_rotate_value(self, carry: bool) -> (Self, bool) {
        (self.wrapping_shl(1) | (carry as u8), self & 0x80 != 0)
    }

    fn right_rotate_value(self, carry: bool) -> (Self, bool) {
        (((carry as u8) << 7) | self.wrapping_shr(1), self & 0x01 != 0)
    }
}

impl Value for u16 {
    fn from_modal(modal: u16) -> Self {
        modal
    }

    fn to_modal(&self, modal: &mut u16) {
        *modal = *self;
    }

    fn from_bool(value: bool) -> Self {
        value as Self
    }

    fn is_zero(&self) -> bool {
        *self == 0
    }

    fn is_overflow(&self) -> bool {
        *self & 0x4000 != 0
    }

    fn is_negative(&self) -> bool {
        *self & 0x8000 != 0
    }

    fn add_value(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }

    fn subtract_value(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }

    fn left_shift_value(self) -> (Self, bool) {
        (self.wrapping_shl(1), self & 0x8000 != 0)
    }

    fn right_shift_value(self) -> (Self, bool) {
        (self.wrapping_shr(1), self & 0x0001 != 0)
    }

    fn left_rotate_value(self, carry: bool) -> (Self, bool) {
        (self.wrapping_shl(1) | (carry as u16), self & 0x8000 != 0)
    }

    fn right_rotate_value(self, carry: bool) -> (Self, bool) {
        (((carry as u16) << 15) | self.wrapping_shr(1), self & 0x0001 != 0)
    }
}
