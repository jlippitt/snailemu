use util::byte_access::ByteAccess;

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8
}

impl Color {
    pub fn new(red: u8, green: u8, blue: u8) -> Color {
        Color {
            red: red,
            green: green,
            blue: blue
        }
    }

    pub fn red(&self) -> u8 {
        self.red
    }

    pub fn set_red(&mut self, intensity: u8) {
        self.red = intensity;
    }

    pub fn green(&self) -> u8 {
        self.green
    }

    pub fn set_green(&mut self, intensity: u8) {
        self.green = intensity;
    }

    pub fn blue(&self) -> u8 {
        self.blue
    }

    pub fn set_blue(&mut self, intensity: u8) {
        self.blue = intensity;
    }
}

impl From<u16> for Color {
    fn from(value: u16) -> Color {
        Color {
            red: (value & 0x001F) as u8,
            green: ((value & 0x03E0) >> 5) as u8,
            blue: ((value & 0x7C00) >> 10) as u8
        }
    }
}

impl From<Color> for u16 {
    fn from(color: Color) -> u16 {
        ((color.blue as u16) << 10) | ((color.green as u16) << 5) | (color.red as u16)
    }
}

impl ByteAccess for Color {
    fn lower(&self) -> u8 {
        u16::from(*self).lower()
    }

    fn upper(&self) -> u8 {
        u16::from(*self).upper()
    }

    fn set_lower(&mut self, value: u8) {
        self.red = value & 0x1F;
        self.green = (self.green & 0x18) | ((value & 0xE0) >> 5);
    }

    fn set_upper(&mut self, value: u8) {
        self.blue = (value & 0x7C) >> 2;
        self.green = ((value & 0x03) << 3) | (self.green & 0x07);
    }
}
