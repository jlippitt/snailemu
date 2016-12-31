use super::ppu::{Ppu, ScreenLayer};
use util::color::Color;

pub struct ColorMath {
    source: ColorMathSource,
    operation: ColorMathOperation,
    divisor: u8,
    fixed_color: Color
}

enum ColorMathSource {
    FixedColor,
    SubScreen
}

enum ColorMathOperation {
    Add,
    Subtract
}

impl ColorMath {
    pub fn new() -> ColorMath {
        // TODO: Window settings
        ColorMath {
            source: ColorMathSource::FixedColor,
            operation: ColorMathOperation::Add,
            divisor: 1,
            fixed_color: Color::default()
        }
    }

    pub fn set_source(&mut self, value: u8) {
        // TODO: Clipping window settings
        self.source = match value & 0x02 {
            0x02 => ColorMathSource::SubScreen,
            _ => ColorMathSource::FixedColor
        };
    }

    pub fn set_operation(&mut self, value: u8) {
        self.operation = match value & 0x80 {
            0x80 => ColorMathOperation::Subtract,
            _ => ColorMathOperation::Add
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

    pub fn apply(&self, ppu: &Ppu, lhs: Color, screen_x: usize, screen_y: usize) -> Color {
        // TODO: Window clipping
        let (rhs, divisor) = match self.source {
            ColorMathSource::FixedColor => (self.fixed_color, self.divisor),
            ColorMathSource::SubScreen => {
                let maybe_color = ppu.background_mode().color_at(ppu, screen_x, screen_y, ScreenLayer::SubScreen);

                // Don't apply divisor if we fall back to fixed colour (for whatever reason)
                match maybe_color {
                    Some((subscreen_color, _)) => (subscreen_color, self.divisor),
                    None => (self.fixed_color, 1)
                }
            }
        };

        let operator = match self.operation {
            ColorMathOperation::Add => u8::saturating_add,
            ColorMathOperation::Subtract => u8::saturating_sub
        };

        Color::new(
            operator(lhs.red(), rhs.red()) / divisor,
            operator(lhs.green(), rhs.green()) / divisor,
            operator(lhs.blue(), rhs.blue()) / divisor
        )
    }
}
