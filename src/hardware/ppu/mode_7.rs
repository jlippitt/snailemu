use super::background_mode::Priority;
use super::ppu::Ppu;
use util::byte_access::WriteTwice;
use std::mem;
use util::color::Color;

const CHR_SIZE: usize = 8;
const FIELD_SIZE: isize = (CHR_SIZE * 128) as isize;

pub struct Mode7 {
    scroll_x_raw: WriteTwice<u16>,
    scroll_y_raw: WriteTwice<u16>,
    scroll_x: isize,
    scroll_y: isize
}

#[inline]
fn signed_scroll_value(raw_value: u16) -> isize {
    // Convert raw value into scroll value using 13-bit signed format
    let sign_bit = raw_value & 0x1000 != 0;
    let unsigned_value = raw_value & 0x0FFF;
    if sign_bit {
        ((0xF000 | unsigned_value) as i16) as isize
    } else {
        unsigned_value as isize
    }
}

impl Mode7 {
    pub fn new() -> Mode7 {
        Mode7 {
            scroll_x_raw: WriteTwice::new(0x0000, 0x1FFF),
            scroll_y_raw: WriteTwice::new(0x0000, 0x1FFF),
            scroll_x: 0,
            scroll_y: 0
        }
    }

    pub fn set_scroll_x(&mut self, value: u8) {
        self.scroll_x_raw.write(value);
        self.scroll_x = signed_scroll_value(self.scroll_x_raw.value());
        debug!("Mode 7 Scroll X: {:04X} => {:04X} ({})", self.scroll_x_raw.value(), self.scroll_x, self.scroll_x);
    }

    pub fn set_scroll_y(&mut self, value: u8) {
        self.scroll_y_raw.write(value);
        self.scroll_y = signed_scroll_value(self.scroll_y_raw.value());
        debug!("Mode 7 Scroll Y: {:04X} => {:04X} ({})", self.scroll_y_raw.value(), self.scroll_y, self.scroll_y);
    }

    pub fn color_at(&self, ppu: &Ppu, screen_x: usize, screen_y: usize, priority_enabled: bool)
        -> Option<(Color, Priority, bool)>
    {
        let signed_pos_x = (screen_x as isize) + self.scroll_x;
        let signed_pos_y = (screen_y as isize) + self.scroll_y;

        if signed_pos_x < 0 || signed_pos_y < 0 || signed_pos_x >= FIELD_SIZE || signed_pos_y >= FIELD_SIZE {
            // TODO: *May* be character 0, depending on settings
            return None;
        }

        let pos_x = signed_pos_x as usize;
        let pos_y = signed_pos_y as usize;

        let tile_x = pos_x / CHR_SIZE;
        let tile_y = pos_y / CHR_SIZE;

        let character = ppu.vram().mode_7_chr_at(tile_x, tile_y);

        let color_index = character.pixel_at(pos_x % CHR_SIZE, pos_y % CHR_SIZE);

        if color_index != 0 {
            if priority_enabled {
                Some((ppu.cgram().color((color_index & 0x7F) as usize), 0, color_index & 0x80 != 0))
            } else {
                Some((ppu.cgram().color(color_index as usize), 0, false))
            }
        } else {
            None
        }
    }
}
