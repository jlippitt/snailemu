use hardware::hardware::HardwareBus;
use hardware::io_port::{IoPort, PPU_LATCH_BIT};
use hardware::screen::Screen;
use std::rc::Rc;
use super::background_layer::BackgroundLayer;
use super::background_mode::BackgroundMode;
use super::cgram::Cgram;
use super::color_math::ColorMath;
use super::mode_7::Mode7;
use super::oam::Oam;
use super::object_layer::ObjectLayer;
use super::vram::Vram;
use super::window::Window;
use util::byte_access::{ReadTwice, WriteTwice};
use util::color::Color;

const DOTS_PER_LINE: usize = 340;
const TOTAL_SCANLINES: usize = 262;

const DISPLAY_LEFT: usize = 22;
const DISPLAY_RIGHT: usize = 278;
const DISPLAY_TOP: usize = 1;

const HBLANK_START: usize = 274;
const HBLANK_END: usize = 1;

const VBLANK_START_NORMAL: usize = 225;
const VBLANK_START_OVERSCAN: usize = 240;

const STANDARD_PIXEL_CYCLES: u64 = 4;
const WIDE_PIXEL_CYCLES: u64 = 6;

const CHIP_VERSION_5C77: u8 = 1;
const CHIP_VERSION_5C78: u8 = 3;

pub struct Ppu {
    screen: Screen,
    io_port: Rc<IoPort>,
    position: Position,
    stored_position: StoredPosition,
    force_blank: bool,
    hblank: bool,
    vblank: bool,
    oam: Oam,
    vram: Vram,
    cgram: Cgram,
    background_mode: BackgroundMode,
    bg1: BackgroundLayer,
    bg2: BackgroundLayer,
    bg3: BackgroundLayer,
    bg4: BackgroundLayer,
    mode_7: Mode7,
    object_layer: ObjectLayer,
    window1: Window,
    window2: Window,
    color_math: ColorMath,
    backdrop_color_math_enabled: bool,
    multiplication: Multiplication,
    cycles: u64,
    next_pixel_cycles: u64
}

pub struct Position {
    h: usize,
    v: usize
}

struct StoredPosition {
    h: ReadTwice<u16>,
    v: ReadTwice<u16>,
    stored: bool
}

struct Multiplication {
    lhs: WriteTwice<u16>,
    result: u32
}

impl Ppu {
    pub fn new(screen: Screen, io_port: Rc<IoPort>) -> Ppu {
        Ppu {
            screen: screen,
            io_port: io_port,
            position: Position {
                h: 0,
                v: 0
            },
            stored_position: StoredPosition {
                h: ReadTwice::new(0x0000, 0x01FF),
                v: ReadTwice::new(0x0000, 0x01FF),
                stored: false
            },
            force_blank: true,
            hblank: true,
            vblank: true,
            oam: Oam::new(),
            vram: Vram::new(),
            cgram: Cgram::new(),
            background_mode: BackgroundMode::new(),
            bg1: BackgroundLayer::new(),
            bg2: BackgroundLayer::new(),
            bg3: BackgroundLayer::new(),
            bg4: BackgroundLayer::new(),
            mode_7: Mode7::new(),
            object_layer: ObjectLayer::new(),
            window1: Window::new(),
            window2: Window::new(),
            color_math: ColorMath::new(),
            backdrop_color_math_enabled: false,
            multiplication: Multiplication {
                lhs: WriteTwice::new(0x0000, 0xFFFF),
                result: 0x00000000
            },
            cycles: 0,
            next_pixel_cycles: STANDARD_PIXEL_CYCLES
        }
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn store_position(&mut self) {
        self.stored_position.h.set_value(self.position.h as u16);
        self.stored_position.v.set_value(self.position.v as u16);
        self.stored_position.stored = true;
    }

    pub fn oam(&self) -> &Oam {
        &self.oam
    }

    pub fn vram(&self) -> &Vram {
        &self.vram
    }

    pub fn cgram(&self) -> &Cgram {
        &self.cgram
    }

    pub fn background_mode(&self) -> &BackgroundMode {
        &self.background_mode
    }

    pub fn bg1(&self) -> &BackgroundLayer {
        &self.bg1
    }

    pub fn bg2(&self) -> &BackgroundLayer {
        &self.bg2
    }

    pub fn bg3(&self) -> &BackgroundLayer {
        &self.bg3
    }

    pub fn bg4(&self) -> &BackgroundLayer {
        &self.bg4
    }

    pub fn mode_7(&self) -> &Mode7 {
        &self.mode_7
    }

    pub fn object_layer(&self) -> &ObjectLayer {
        &self.object_layer
    }

    pub fn window1(&self) -> &Window {
        &self.window1
    }

    pub fn window2(&self) -> &Window {
        &self.window2
    }

    pub fn color_math(&self) -> &ColorMath {
        &self.color_math
    }

    pub fn backdrop_color_math_enabled(&self) -> bool {
        self.backdrop_color_math_enabled
    }

    pub fn add_cycles(&mut self, cycles: u64) {
        self.cycles += cycles;
    }

    pub fn next_pixel(&mut self) -> bool {
        if self.cycles < self.next_pixel_cycles {
            return false;
        }

        self.cycles -= self.next_pixel_cycles;

        let vblank_start = match self.screen.overscan() {
            false => VBLANK_START_NORMAL,
            true => VBLANK_START_OVERSCAN
        };

        if self.position.v >= DISPLAY_TOP && self.position.v < vblank_start &&
            self.position.h >= DISPLAY_LEFT && self.position.h < DISPLAY_RIGHT
        {
            let (even_color, odd_color) = if !self.force_blank {
                let screen_x = self.position.h - DISPLAY_LEFT;
                let screen_y = self.position.v - DISPLAY_TOP;
                self.background_mode.color_at(self, screen_x, screen_y)
            } else {
                (Color::default(), Color::default())
            };

            // Blit two pixels because we are always in 'pseudo-HD'
            self.screen.blit(even_color);
            self.screen.blit(odd_color);
        }

        self.position.h += 1;

        if self.position.h == DOTS_PER_LINE {
            self.position.h = 0;
            self.position.v += 1;

            if self.position.v == DISPLAY_TOP {
                self.screen.begin_frame();
            } else if self.position.v < vblank_start {
                self.screen.next_line();
            } else if !self.vblank {
                self.screen.end_frame();
                self.vblank = true;
            } else if self.position.v == TOTAL_SCANLINES {
                self.position.v = 0;
                self.vblank = false;
            }
        }

        self.hblank = self.position.h >= HBLANK_START || self.position.h < HBLANK_END;

        self.next_pixel_cycles = match self.position.h {
            322 | 326 => WIDE_PIXEL_CYCLES,
            _ => STANDARD_PIXEL_CYCLES
        };

        true
    }

    pub fn vblank(&self) -> bool {
        self.vblank
    }

    pub fn hblank(&self) -> bool {
        self.hblank
    }
}

impl HardwareBus for Ppu {
    fn read(&mut self, offset: usize) -> u8 {
        match offset {
            0x34 => self.multiplication.result as u8,
            0x35 => self.multiplication.result.wrapping_shr(8) as u8,
            0x36 => self.multiplication.result.wrapping_shr(16) as u8,
            0x37 => {
                // Store current H and V counter values if IO port latch is 'high'
                if self.io_port.value() & PPU_LATCH_BIT != 0 {
                    self.store_position();
                }
                0x00 // TODO: Open bus
            },
            0x38 => self.oam.read(),
            0x39 => self.vram.read_low_byte(),
            0x3A => self.vram.read_high_byte(),
            0x3B => self.cgram.read(),
            0x3C => self.stored_position.h.read(),
            0x3D => self.stored_position.v.read(),
            0x3E => {
                // TODO: Time over flag
                // TODO: Range over flag
                CHIP_VERSION_5C77
            },
            0x3F => {
                let mut value = 0x00;
                // TODO: Interlace field
                if self.stored_position.stored {
                    value |= 0x40;
                }
                self.stored_position.h.reset_byte_selector();
                self.stored_position.v.reset_byte_selector();
                value | CHIP_VERSION_5C78
            },
            _ => 0x00 // TODO: Open bus
        }
    }

    fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0x00 => {
                self.screen.set_brightness(((value & 0x0F) << 4) | 0x0F);
                self.force_blank = value & 0x80 != 0;
            },
            0x01 => self.object_layer.set_config(value),
            0x02 => self.oam.set_address(value),
            0x03 => {
                // TODO: Object priority
                self.oam.set_table(value & 0x01);
            },
            0x04 => self.oam.write(value),
            0x05 => {
                // TODO: BG tile size
                self.background_mode.set_mode(value & 0x0F);
            },
            0x07 => self.bg1.set_tile_map_locations(value),
            0x08 => self.bg2.set_tile_map_locations(value),
            0x09 => self.bg3.set_tile_map_locations(value),
            0x0A => self.bg4.set_tile_map_locations(value),
            0x0B => {
                self.bg1.set_chr_offset(value & 0x0F);
                self.bg2.set_chr_offset((value & 0xF0) >> 4);
            },
            0x0C => {
                self.bg3.set_chr_offset(value & 0x0F);
                self.bg4.set_chr_offset((value & 0xF0) >> 4);
            },
            0x0D => {
                self.bg1.set_scroll_x(value);
                self.mode_7.set_scroll_x(value);
            },
            0x0E => {
                self.bg1.set_scroll_y(value);
                self.mode_7.set_scroll_y(value);
            },
            0x0F => self.bg2.set_scroll_x(value),
            0x10 => self.bg2.set_scroll_y(value),
            0x11 => self.bg3.set_scroll_x(value),
            0x12 => self.bg3.set_scroll_y(value),
            0x13 => self.bg4.set_scroll_x(value),
            0x14 => self.bg4.set_scroll_y(value),
            0x15 => self.vram.set_port_control(value),
            0x16 => self.vram.set_lower_address_byte(value),
            0x17 => self.vram.set_upper_address_byte(value),
            0x18 => self.vram.write_low_byte(value),
            0x19 => self.vram.write_high_byte(value),
            0x1B => self.multiplication.lhs.write(value),
            0x1C => {
                // Multiplication is signed and result is only 24-bit, which complicates things...
                let lhs = (self.multiplication.lhs.value() as i16) as i32;
                let rhs = (value as i8) as i32;
                let result = (lhs * rhs) as u32;
                // Drag the sign bit to the right so it sits at bit 23
                self.multiplication.result = ((result & 0x80000000) >> 8) | (result & 0x007FFFFF);
            },
            0x21 => self.cgram.set_address(value),
            0x22 => self.cgram.write(value),
            0x23 => {
                self.bg1.set_window_mask_options(value & 0x0F);
                self.bg2.set_window_mask_options((value & 0xF0) >> 4);
            },
            0x24 => {
                self.bg3.set_window_mask_options(value & 0x0F);
                self.bg4.set_window_mask_options((value & 0xF0) >> 4);
            },
            0x25 => {
                self.object_layer.set_window_mask_options(value & 0x0F);
                self.color_math.set_window_mask_options((value & 0xF0) >> 4);
            },
            0x26 => self.window1.set_left(value),
            0x27 => self.window1.set_right(value),
            0x28 => self.window2.set_left(value),
            0x29 => self.window2.set_right(value),
            0x2A => {
                self.bg1.set_window_mask_logic(value & 0x03);
                self.bg2.set_window_mask_logic((value & 0x0C) >> 2);
                self.bg3.set_window_mask_logic((value & 0x30) >> 4);
                self.bg4.set_window_mask_logic((value & 0xC0) >> 6);
            },
            0x2B => {
                self.object_layer.set_window_mask_logic(value & 0x03);
                self.color_math.set_window_mask_logic((value & 0x0C) >> 2);
            },
            0x2C => {
                self.bg1.set_main_screen_enabled(value & 0x01 != 0);
                self.bg2.set_main_screen_enabled(value & 0x02 != 0);
                self.bg3.set_main_screen_enabled(value & 0x04 != 0);
                self.bg4.set_main_screen_enabled(value & 0x08 != 0);
                self.object_layer.set_main_screen_enabled(value & 0x10 != 0);
            },
            0x2D => {
                self.bg1.set_sub_screen_enabled(value & 0x01 != 0);
                self.bg2.set_sub_screen_enabled(value & 0x02 != 0);
                self.bg3.set_sub_screen_enabled(value & 0x04 != 0);
                self.bg4.set_sub_screen_enabled(value & 0x08 != 0);
                self.object_layer.set_sub_screen_enabled(value & 0x10 != 0);
            },
            0x30 => self.color_math.set_source(value),
            0x31 => {
                self.bg1.set_color_math_enabled(value & 0x01 != 0);
                self.bg2.set_color_math_enabled(value & 0x02 != 0);
                self.bg3.set_color_math_enabled(value & 0x04 != 0);
                self.bg4.set_color_math_enabled(value & 0x08 != 0);
                self.object_layer.set_color_math_enabled(value & 0x10 != 0);
                self.backdrop_color_math_enabled = value & 0x20 != 0;
                self.color_math.set_operation(value & 0xC0);
            },
            0x32 => self.color_math.adjust_fixed_color(value),
            0x33 => {
                self.background_mode.set_mode_7_ext(value & 0x40 != 0);
                // TODO: Pseudo-hi-res mode
                self.screen.set_overscan(value & 0x04 != 0);
                // TODO: Interlace settings
            },
            _ => ()
        }
    }
}

impl Position {
    pub fn h(&self) -> u16 {
        self.h as u16
    }

    pub fn v(&self) -> u16 {
        self.v as u16
    }
}
