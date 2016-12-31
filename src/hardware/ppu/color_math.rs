use util::color::Color;

pub struct ColorMath {
    source: ColorMathSource,
    operation: ColorMathOperator,
    divisor: u8,
    fixed_color: Color
}

enum ColorMathSource {
    FixedColor,
    SubScreen
}

enum ColorMathOperator {
    Add,
    Subtract
}

impl ColorMath {
    pub fn new() -> ColorMath {
        // TODO: Window settings
        ColorMath {
            source: ColorMathSource::FixedColor,
            operation: ColorMathOperator::Add,
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

    pub fn clip(&self, enabled: bool, screen_x: usize, screen_y: usize) -> bool {
        // TODO: Window clipping
        !enabled
    }

    pub fn apply<F>(&self, lhs: Color, clip: bool, sub_screen_fn: F) -> Color
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

        Color::new(
            operator(lhs.red(), rhs.red()) / divisor,
            operator(lhs.green(), rhs.green()) / divisor,
            operator(lhs.blue(), rhs.blue()) / divisor
        )
    }
}
