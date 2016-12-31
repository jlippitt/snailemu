use std::cell::Cell;
use super::background_layer::{ColorMode, PixelOptions};
use super::ppu::Ppu;
use util::color::Color;

pub struct BackgroundMode {
    mode_fn: Box<ModeFn>,
    pseudo_hi_res: bool,
    prev_clip: Cell<bool>
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ScreenLayer {
    MainScreen,
    SubScreen
}

pub type Priority = u8;

type ModeFn = Fn(&Ppu, usize, usize, ScreenLayer) -> Option<Pixel>;

type Pixel = (Color, bool);

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

#[inline]
fn resolve_pixel(maybe_pixel: Option<Pixel>, ppu: &Ppu) -> Pixel {
    if let Some(pixel) = maybe_pixel {
        pixel
    } else {
        (ppu.cgram().color(0), ppu.backdrop_color_math_enabled())
    }
}

impl BackgroundMode {
    pub fn new() -> BackgroundMode {
        BackgroundMode {
            mode_fn: Box::new(mode_0),
            pseudo_hi_res: false,
            prev_clip: Cell::new(false)
        }
    }

    pub fn set_mode(&mut self, value: u8) {
        let mode = value & 0x07;

        self.mode_fn = Box::new(match mode {
            0 => mode_0,
            1 => if value & 0x08 != 0 { mode_1_high_priority } else { mode_1_low_priority },
            2 => mode_2,
            4 => mode_4,
            5 => mode_5,
            6 => mode_6,
            _ => panic!("Mode {} not yet supported", mode)
        });

        self.pseudo_hi_res = mode == 5 || mode == 6;
    }

    pub fn color_at(&self, ppu: &Ppu, screen_x: usize, screen_y: usize) -> (Color, Color) {
        let main_screen_pixel = (self.mode_fn)(ppu, screen_x, screen_y, ScreenLayer::MainScreen);
        let (main_screen_color, color_math_enabled) = resolve_pixel(main_screen_pixel, ppu);

        let sub_screen_fn = || (self.mode_fn)(ppu, screen_x, screen_y, ScreenLayer::SubScreen);

        let color_math = ppu.color_math();

        if self.pseudo_hi_res {
            let sub_screen_pixel = sub_screen_fn();
            let (sub_screen_color, _) = resolve_pixel(sub_screen_pixel, ppu);
            let clip = color_math.clip(color_math_enabled, screen_x, screen_y);
            let even_color = color_math.apply(sub_screen_color, self.prev_clip.get(), || main_screen_pixel);
            let odd_color = color_math.apply(main_screen_color, clip, || sub_screen_pixel);
            self.prev_clip.set(clip);
            (even_color, odd_color)
        } else {
            let clip = color_math.clip(color_math_enabled, screen_x, screen_y);
            let final_color = color_math.apply(main_screen_color, clip, sub_screen_fn);
            (final_color, final_color)
        }
    }
}

fn mode_0(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
    let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
    try_pixel!(object_pixel, 3);
    let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color4,
        palette_offset: 0,
        ..Default::default()
    });
    try_pixel!(bg1_pixel, 1);
    let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color4,
        palette_offset: 32,
        ..Default::default()
    });
    try_pixel!(bg2_pixel, 1);
    try_pixel!(object_pixel, 2);
    try_pixel!(bg1_pixel);
    try_pixel!(bg2_pixel);
    try_pixel!(object_pixel, 1);
    let bg3_pixel = ppu.bg3().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color4,
        palette_offset: 64,
        ..Default::default()
    });
    try_pixel!(bg3_pixel, 1);
    let bg4_pixel = ppu.bg4().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color4,
        palette_offset: 96,
        ..Default::default()
    });
    try_pixel!(bg4_pixel, 1);
    try_pixel!(object_pixel);
    try_pixel!(bg3_pixel);
    try_pixel!(bg4_pixel);
    None
}

fn mode_1_high_priority(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
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
    None
}

fn mode_1_low_priority(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
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
    None
}

fn mode_2(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
    let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
    try_pixel!(object_pixel, 3);
    let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color16,
        ..Default::default()
    });
    try_pixel!(object_pixel, 2);
    let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color16,
        ..Default::default()
    });
    try_pixel!(bg2_pixel, 1);
    try_pixel!(object_pixel, 1);
    try_pixel!(bg1_pixel);
    try_pixel!(object_pixel);
    try_pixel!(bg2_pixel);
    None
}

fn mode_4(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
    let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
    try_pixel!(object_pixel, 3);
    let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color256,
        ..Default::default()
    });
    try_pixel!(bg1_pixel, 1);
    try_pixel!(object_pixel, 2);
    let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color4,
        ..Default::default()
    });
    try_pixel!(bg2_pixel, 1);
    try_pixel!(object_pixel, 1);
    try_pixel!(bg1_pixel);
    try_pixel!(object_pixel);
    try_pixel!(bg2_pixel);
    None
}

fn mode_5(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
    let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
    try_pixel!(object_pixel, 3);
    let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color16,
        always_wide: true,
        ..Default::default()
    });
    try_pixel!(object_pixel, 2);
    let bg2_pixel = ppu.bg2().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color4,
        always_wide: true,
        ..Default::default()
    });
    try_pixel!(bg2_pixel, 1);
    try_pixel!(object_pixel, 1);
    try_pixel!(bg1_pixel);
    try_pixel!(object_pixel);
    try_pixel!(bg2_pixel);
    None
}

fn mode_6(ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer) -> Option<Pixel> {
    let object_pixel = ppu.object_layer().color_at(ppu, screen_x, screen_y, screen_layer);
    try_pixel!(object_pixel, 3);
    let bg1_pixel = ppu.bg1().color_at(ppu, screen_x, screen_y, screen_layer, &PixelOptions {
        color_mode: ColorMode::Color16,
        always_wide: true,
        ..Default::default()
    });
    try_pixel!(object_pixel, 2);
    try_pixel!(object_pixel, 1);
    try_pixel!(bg1_pixel);
    try_pixel!(object_pixel);
    None
}
