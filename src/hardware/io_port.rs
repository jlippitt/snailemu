use std::cell::Cell;

pub const PPU_LATCH_BIT: u8 = 0x80;

pub struct IoPort {
    value: Cell<u8>,
    triggered: Cell<bool>
}

impl IoPort {
    pub fn new() -> IoPort {
        IoPort {
            value: Cell::new(0xC0),
            triggered: Cell::new(false)
        }
    }

    pub fn value(&self) -> u8 {
        self.value.get()
    }

    pub fn set_value(&self, value: u8) {
        let old_value = self.value.get();

        self.value.set(value);

        if (old_value & PPU_LATCH_BIT) != 0 && (value & PPU_LATCH_BIT) == 0 {
            self.triggered.set(true);
        }
    }

    pub fn triggered(&self) -> bool {
        self.triggered.get()
    }

    pub fn reset_trigger(&self) {
        self.triggered.set(false);
    }
}
