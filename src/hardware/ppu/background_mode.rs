use super::background_layer::{ColorMode, PixelOptions};
use super::ppu::{Ppu, ScreenLayer};
use util::color::Color;

pub struct BackgroundMode {
    mode: u8,
    bg3_priority: bool
}

pub type Priority = u8;

macro_rules! try_pixel {
    ($maybe_color:expr, $priority:expr) => {{
        if let Some((color, priority, color_math_enabled)) = $maybe_color {
            if priority == $priority {
                return Some((color, color_math_enabled));
            }
        }
    }};
    ($maybe_color:expr) => {{
        if let Some((color, _, color_math_enabled)) = $maybe_color {
            return Some((color, color_math_enabled));
        }
    }};
}

impl BackgroundMode {
    pub fn new() -> BackgroundMode {
        BackgroundMode {
            mode: 0,
            bg3_priority: false
        }
    }

    pub fn set_mode(&mut self, value: u8) {
        self.mode = value & 0x07;
        self.bg3_priority = value & 0x08 != 0;
    }

    pub fn color_at(&self, ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<(Color, bool)> {
        match self.mode {
            0 => self.mode_0(ppu, screen_x, screen_y, screen_layer),
            1 => self.mode_1(ppu, screen_x, screen_y, screen_layer),
            _ => panic!("Mode {} not yet supported", self.mode)
        }
    }

    fn mode_0(&self, ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<(Color, bool)> {
        let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
        try_pixel!(object_pixel, 3);
        let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
            color_mode: ColorMode::Color4,
            palette_offset: 0
        });
        try_pixel!(bg1_pixel, 1);
        let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
            color_mode: ColorMode::Color4,
            palette_offset: 32
        });
        try_pixel!(bg2_pixel, 1);
        try_pixel!(object_pixel, 2);
        try_pixel!(bg1_pixel);
        try_pixel!(bg2_pixel);
        try_pixel!(object_pixel, 1);
        let bg3_pixel = ppu.bg3().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
            color_mode: ColorMode::Color4,
            palette_offset: 64
        });
        try_pixel!(bg3_pixel, 1);
        let bg4_pixel = ppu.bg4().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
            color_mode: ColorMode::Color4,
            palette_offset: 96
        });
        try_pixel!(bg4_pixel, 1);
        try_pixel!(object_pixel);
        try_pixel!(bg3_pixel);
        try_pixel!(bg4_pixel);
        None
    }

    fn mode_1(&self, ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<(Color, bool)> {
        if self.bg3_priority {
            let bg3_pixel = ppu.bg3().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
                color_mode: ColorMode::Color4,
                ..Default::default()
            });
            try_pixel!(bg3_pixel, 1);
            let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
            try_pixel!(object_pixel, 3);
            let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
                color_mode: ColorMode::Color16,
                ..Default::default()
            });
            try_pixel!(bg1_pixel, 1);
            let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
                color_mode: ColorMode::Color16,
                ..Default::default()
            });
            try_pixel!(bg2_pixel, 1);
            try_pixel!(object_pixel, 2);
            try_pixel!(bg1_pixel);
            try_pixel!(bg2_pixel);
            try_pixel!(object_pixel, 1);
            try_pixel!(object_pixel);
            try_pixel!(bg3_pixel);
        } else {
            let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
            try_pixel!(object_pixel, 3);
            let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
                color_mode: ColorMode::Color16,
                ..Default::default()
            });
            try_pixel!(bg1_pixel, 1);
            let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
                color_mode: ColorMode::Color16,
                ..Default::default()
            });
            try_pixel!(bg2_pixel, 1);
            try_pixel!(object_pixel, 2);
            try_pixel!(bg1_pixel);
            try_pixel!(bg2_pixel);
            try_pixel!(object_pixel, 1);
            let bg3_pixel = ppu.bg3().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
                color_mode: ColorMode::Color4,
                ..Default::default()
            });
            try_pixel!(bg3_pixel, 1);
            try_pixel!(object_pixel);
            try_pixel!(bg3_pixel);
        }

        None
    }
}
