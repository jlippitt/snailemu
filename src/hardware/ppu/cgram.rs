use util::byte_access::{ByteAccess, ByteSelector};
use util::color::Color;

const COLOR_COUNT: usize = 256;

pub struct Cgram {
    colors: Vec<Color>,
    address: usize,
    write_buffer: u8,
    byte_selector: ByteSelector
}

impl Cgram {
    pub fn new() -> Cgram {
        Cgram {
            colors: vec![Color::default(); COLOR_COUNT],
            address: 0,
            write_buffer: 0x00,
            byte_selector: ByteSelector::Lower
        }
    }

    pub fn set_address(&mut self, value: u8) {
        self.address = value as usize;
        self.byte_selector = ByteSelector::Lower;
    }

    pub fn read(&mut self) -> u8 {
        match self.byte_selector {
            ByteSelector::Lower => {
                self.byte_selector = ByteSelector::Upper;
                self.colors[self.address].lower()
            },
            ByteSelector::Upper => {
                self.byte_selector = ByteSelector::Lower;
                let value = self.colors[self.address].upper();
                self.address = (self.address + 1) % COLOR_COUNT;
                value
            }
        }
    }

    pub fn write(&mut self, value: u8) {
        // Values are only written to memory when the upper byte of the word is written
        match self.byte_selector {
            ByteSelector::Lower => {
                debug!("CGRAM Write (Low): {:02X} <= {:02X}", self.address, value);
                self.byte_selector = ByteSelector::Upper;
                self.write_buffer = value;
            },
            ByteSelector::Upper => {
                debug!("CGRAM Write (High): {:02X} <= {:02X}", self.address, value);
                self.byte_selector = ByteSelector::Lower;
                let color = &mut self.colors[self.address];
                color.set_lower(self.write_buffer);
                color.set_upper(value);
                self.address = (self.address + 1) % COLOR_COUNT;
            }
        };
    }

    pub fn color(&self, index: usize) -> Color {
        self.colors[index]
    }
}
