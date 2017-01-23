use super::ppu::Ppu;
use super::window::WindowMask;
use util::color::Color;

pub struct ColorMath {
    source: ColorMathSource,
    prevent: ColorMathWindowOperator,
    clip_to_black: ColorMathWindowOperator,
    operation: ColorMathOperator,
    divisor: u8,
    fixed_color: Color,
    window_mask: WindowMask
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ColorMathSource {
    FixedColor,
    SubScreen
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ColorMathOperator {
    Add,
    Subtract
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ColorMathWindowOperator {
    Never,
    Outside,
    Inside,
    Always
}

impl ColorMath {
    pub fn new() -> ColorMath {
        // TODO: Window settings
        ColorMath {
            source: ColorMathSource::FixedColor,
            prevent: ColorMathWindowOperator::Never,
            clip_to_black: ColorMathWindowOperator::Never,
            operation: ColorMathOperator::Add,
            divisor: 1,
            fixed_color: Color::default(),
            window_mask: WindowMask::new()
        }
    }

    pub fn set_source(&mut self, value: u8) {
        self.source = match value & 0x02 {
            0x02 => ColorMathSource::SubScreen,
            _ => ColorMathSource::FixedColor
        };

        self.prevent = ColorMathWindowOperator::from((value & 0x30) >> 4);
        self.clip_to_black = ColorMathWindowOperator::from((value & 0xC0) >> 6);
    }

    pub fn set_operation(&mut self, value: u8) {
        self.operation = match value & 0x80 {
            0x80 => ColorMathOperator::Subtract,
            _ => ColorMathOperator::Add
        };

        self.divisor = match value & 0x40 {
            0x40 => 2,
            _ => 1
        };
    }

    pub fn adjust_fixed_color(&mut self, value: u8) {
        if value & 0x20 != 0 {
            self.fixed_color.set_red(value & 0x1F);
        }

        if value & 0x40 != 0 {
            self.fixed_color.set_green(value & 0x1F);
        }

        if value & 0x80 != 0 {
            self.fixed_color.set_blue(value & 0x1F);
        }
    }

    pub fn set_window_mask_options(&mut self, value: u8) {
        self.window_mask.set_options(value);
    }

    pub fn set_window_mask_logic(&mut self, value: u8) {
        self.window_mask.set_operator(value);
    }

    pub fn clip(&self, ppu: &Ppu, enabled: bool, screen_x: usize) -> bool {
        // TODO: Window masking
        !enabled || self.apply_window_logic(self.prevent, ppu, screen_x)
    }

    pub fn apply<F>(&self, ppu: &Ppu, screen_x: usize, lhs: Color, clip: bool, sub_screen_fn: F) -> Color
        where F: Fn() -> Option<(Color, bool)>
    {
        if clip {
            return lhs;
        }

        let (rhs, divisor) = match self.source {
            ColorMathSource::FixedColor => (self.fixed_color, self.divisor),
            ColorMathSource::SubScreen => {
                let maybe_color = sub_screen_fn();

                // Don't apply divisor if we fall back to fixed colour (for whatever reason)
                match maybe_color {
                    Some((subscreen_color, _)) => (subscreen_color, self.divisor),
                    None => (self.fixed_color, 1)
                }
            }
        };

        let operator = match self.operation {
            ColorMathOperator::Add => u8::saturating_add,
            ColorMathOperator::Subtract => u8::saturating_sub
        };

        if self.apply_window_logic(self.clip_to_black, ppu, screen_x) {
            Color::new(
                operator(0, rhs.red()),
                operator(0, rhs.green()),
                operator(0, rhs.blue())
            )
        } else {
            Color::new(
                operator(lhs.red(), rhs.red()) / divisor,
                operator(lhs.green(), rhs.green()) / divisor,
                operator(lhs.blue(), rhs.blue()) / divisor
            )
        }
    }

    fn apply_window_logic(&self, logic: ColorMathWindowOperator, ppu: &Ppu, screen_x: usize) -> bool {
        match logic {
            ColorMathWindowOperator::Never => false,
            ColorMathWindowOperator::Outside => !self.window_mask.contains(ppu, screen_x),
            ColorMathWindowOperator::Inside => self.window_mask.contains(ppu, screen_x),
            ColorMathWindowOperator::Always => true
        }
    }
}

impl From<u8> for ColorMathWindowOperator {
    fn from(value: u8) -> ColorMathWindowOperator {
        match value {
            0x00 => ColorMathWindowOperator::Never,
            0x01 => ColorMathWindowOperator::Outside,
            0x02 => ColorMathWindowOperator::Inside,
            0x03 => ColorMathWindowOperator::Always,
            _ => unreachable!()
        }
    }
}
