use super::hardware::HardwareBus;

const WRAM_SIZE: usize = 131072;

pub struct Wram {
    data: WramData,
    address: usize
}

pub struct WramData(Vec<u8>);

impl Wram {
    pub fn new() -> Wram {
        Wram {
            data: WramData(vec![0; WRAM_SIZE]),
            address: 0
        }
    }

    pub fn data(&mut self) -> &mut WramData {
        &mut self.data
    }
}

impl HardwareBus for Wram {
    fn read(&mut self, offset: usize) -> u8 {
        match offset {
            0x00 => {
                let value = self.data.0[self.address];
                self.address = (self.address + 1) % WRAM_SIZE;
                value
            },
            _ => 0x00 // TODO: Open bus
        }
    }

    fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0x00 => {
                self.data.0[self.address] = value;
                self.address = (self.address + 1) % WRAM_SIZE;
            },
            0x01 => self.address = (self.address & 0x1FF00) | (value as usize),
            0x02 => self.address = (self.address & 0x100FF) | ((value as usize) << 8),
            0x03 => self.address = (self.address & 0x0FFFF) | (((value & 0x01) as usize) << 16),
            _ => ()
        };
    }
}

impl HardwareBus for WramData {
    fn read(&mut self, offset: usize) -> u8 {
        self.0[offset]
    }

    fn write(&mut self, offset: usize, value: u8) {
        self.0[offset] = value;
    }
}
