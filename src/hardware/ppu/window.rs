use super::ppu::Ppu;

pub struct Window {
    left: usize,
    right: usize
}

pub struct WindowMask {
    w1_enabled: bool,
    w1_inverted: bool,
    w2_enabled: bool,
    w2_inverted: bool,
    operator: WindowMaskOperator
}

enum WindowMaskOperator {
    Or,
    And,
    Xor,
    Xnor
}

#[inline]
fn invert(value: bool, inverted: bool) -> bool {
    if inverted {
        !value
    } else {
        value
    }
}

impl Window {
    pub fn new() -> Window {
        Window {
            left: 0,
            right: 0
        }
    }

    pub fn set_left(&mut self, value: u8) {
        debug!("Window Left: {:02X}", value);
        self.left = value as usize;
    }

    pub fn set_right(&mut self, value: u8) {
        debug!("Window Right: {:02X}", value);
        self.right = value as usize;
    }

    pub fn contains(&self, x: usize) -> bool {
        x >= self.left && x < self.right
    }
}

impl WindowMask {
    pub fn new() -> WindowMask {
        WindowMask {
            w1_enabled: false,
            w1_inverted: false,
            w2_enabled: false,
            w2_inverted: false,
            operator: WindowMaskOperator::Or
        }
    }

    pub fn set_options(&mut self, value: u8) {
        self.w1_inverted = value & 0x01 != 0;
        self.w1_enabled = value & 0x02 != 0;
        self.w2_inverted = value & 0x04 != 0;
        self.w2_enabled = value & 0x08 != 0;
    }

    pub fn set_operator(&mut self, value: u8) {
        self.operator = match value {
            0x00 => WindowMaskOperator::Or,
            0x01 => WindowMaskOperator::And,
            0x02 => WindowMaskOperator::Xor,
            0x03 => WindowMaskOperator::Xnor,
            _ => unreachable!()
        };
    }

    pub fn contains(&self, ppu: &Ppu, x: usize) -> bool {
        match (self.w1_enabled, self.w2_enabled) {
            (false, false) => false,
            (true, false) => invert(ppu.window1().contains(x), self.w1_inverted),
            (false, true) => invert(ppu.window2().contains(x), self.w2_inverted),
            (true, true) => {
                let w1 = invert(ppu.window1().contains(x), self.w1_inverted);
                let w2 = invert(ppu.window2().contains(x), self.w2_inverted);
                match self.operator {
                    WindowMaskOperator::Or => w1 || w2,
                    WindowMaskOperator::And => w1 && w2,
                    WindowMaskOperator::Xor => (w1 && !w2) || (!w1 && w2),
                    WindowMaskOperator::Xnor => (w1 && w2) || (!w1 && !w2)
                }
            }
        }
    }
}
