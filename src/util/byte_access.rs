pub trait ByteAccess : Copy {
    fn lower(&self) -> u8;
    fn upper(&self) -> u8;
    fn set_lower(&mut self, value: u8);
    fn set_upper(&mut self, value: u8);

    fn get(&self, byte_selector: ByteSelector) -> u8 {
        match byte_selector {
            ByteSelector::Lower => self.lower(),
            ByteSelector::Upper => self.upper()
        }
    }

    fn set(&mut self, byte_selector: ByteSelector, value: u8) {
        match byte_selector {
            ByteSelector::Lower => self.set_lower(value),
            ByteSelector::Upper => self.set_upper(value)
        };
    }
}

pub struct WriteTwice<T: ByteAccess> {
    value: T,
    write_mask: T,
    byte_selector: ByteSelector
}

// This is just an alias
pub type ReadTwice<T> = WriteTwice<T>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ByteSelector {
    Lower,
    Upper
}

impl ByteAccess for u16 {
    fn lower(&self) -> u8 {
        *self as u8
    }

    fn upper(&self) -> u8 {
        self.wrapping_shr(8) as u8
    }
    
    fn set_lower(&mut self, value: u8) {
        *self = (*self & 0xFF00) | (value as u16);
    }

    fn set_upper(&mut self, value: u8) {
        *self = (*self & 0x00FF) | ((value as u16) << 8);
    }
}

impl<T: ByteAccess> WriteTwice<T> {
    pub fn new(initial_value: T, write_mask: T) -> WriteTwice<T> {
        WriteTwice {
            value: initial_value,
            write_mask: write_mask,
            byte_selector: ByteSelector::Lower
        }
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn set_value(&mut self, value: T) {
        self.value.set_lower(value.lower() & self.write_mask.lower());
        self.value.set_upper(value.upper() & self.write_mask.upper());
    }

    pub fn reset_byte_selector(&mut self) {
        self.byte_selector = ByteSelector::Lower;
    }

    pub fn write(&mut self, value: u8) {
        match self.byte_selector {
            ByteSelector::Lower => {
                self.byte_selector = ByteSelector::Upper;
                self.value.set_lower(value & self.write_mask.lower());
            },
            ByteSelector::Upper => {
                self.byte_selector = ByteSelector::Lower;
                self.value.set_upper(value & self.write_mask.upper());
            }
        };
    }

    pub fn read(&mut self) -> u8 {
        match self.byte_selector {
            ByteSelector::Lower => {
                self.byte_selector = ByteSelector::Upper;
                self.value.lower()
            },
            ByteSelector::Upper => {
                self.byte_selector = ByteSelector::Lower;
                self.value.upper()
            }
        }
    }
}
