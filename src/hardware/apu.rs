use super::hardware::HardwareBus;

pub struct Apu {
    ports: [u8; 4],
    transfer_started: bool
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            ports: [0xAA, 0x00, 0x00, 0x00],
            transfer_started: false
        }
    }
}

impl HardwareBus for Apu {
    fn read(&mut self, offset: usize) -> u8 {
        match offset {
            0x00 => self.ports[0],
            0x01 => 0xBB,
            0x02 => self.ports[2],
            0x03 => self.ports[3],
            _ => unreachable!()
        }
    }

    fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0x00 => {
                if self.transfer_started {
                    if value == 0 || value == (self.ports[0] + 1) || self.ports[1] != 0 {
                        debug!("SPC700 {:02X} = {:02X}", value, self.ports[1]);
                    } else {
                        debug!("SPC700 transfer finished");
                        self.transfer_started = false;
                    }
                    self.ports[0] = value;
                } else if value == 0xCC && self.ports[1] != 0 {
                    debug!("SPC700 transfer started");
                    self.transfer_started = true;
                    self.ports[0] = value;
                } else if value == 0x00 {
                    // Reset to default value
                    debug!("SPC700 reset");
                    self.ports[0] = 0xAA;
                }
            },
            0x01 => self.ports[1] = value,
            0x02 => self.ports[2] = value,
            0x03 => self.ports[3] = value,
            _ => unreachable!()
        };
    }
}

