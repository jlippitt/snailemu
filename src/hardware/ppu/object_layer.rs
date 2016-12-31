use super::background_mode::Priority;
use super::oam::SizeSelector;
use super::ppu::{Ppu, ScreenLayer};
use util::color::Color;

const TABLE_SIZE: usize = 256;
const TABLE_ROW_SIZE: usize = 16;
const CHR_SIZE: usize = 8;

pub struct ObjectLayer {
    main_screen_enabled: bool,
    sub_screen_enabled: bool,
    color_math_enabled: bool,
    small_size: ObjectSize,
    large_size: ObjectSize,
    table_offsets: [usize; 2]
}

#[derive(Copy, Clone)]
struct ObjectSize {
    x: isize,
    y: isize
}

impl ObjectLayer {
    pub fn new() -> ObjectLayer {
        ObjectLayer {
            main_screen_enabled: false,
            sub_screen_enabled: false,
            color_math_enabled: false,
            small_size: ObjectSize::new(8, 8),
            large_size: ObjectSize::new(16, 16),
            table_offsets: [0, TABLE_SIZE]
        }
    }

    pub fn set_main_screen_enabled(&mut self, enabled: bool) {
        self.main_screen_enabled = enabled;
    }

    pub fn set_sub_screen_enabled(&mut self, enabled: bool) {
        self.sub_screen_enabled = enabled;
    }

    pub fn set_color_math_enabled(&mut self, enabled: bool) {
        self.color_math_enabled = enabled;
    }

    pub fn set_config(&mut self, value: u8) {
        self.table_offsets[0] = ((value & 0x07) as usize) * TABLE_SIZE * 2;
        self.table_offsets[1] = self.table_offsets[0] + ((((value & 0x18) >> 3) + 1) as usize) * TABLE_SIZE;

        let (small_size, large_size) = match value & 0xE0 {
            0x00 => (ObjectSize::new(8, 8), ObjectSize::new(16, 16)),
            0x20 => (ObjectSize::new(8, 8), ObjectSize::new(32, 32)),
            0x40 => (ObjectSize::new(8, 8), ObjectSize::new(64, 64)),
            0x60 => (ObjectSize::new(16, 16), ObjectSize::new(32, 32)),
            0x80 => (ObjectSize::new(16, 16), ObjectSize::new(64, 64)),
            0xA0 => (ObjectSize::new(32, 32), ObjectSize::new(64, 64)),
            0xC0 => (ObjectSize::new(16, 32), ObjectSize::new(32, 64)),
            0xE0 => (ObjectSize::new(16, 32), ObjectSize::new(32, 32)),
            _ => unreachable!()
        };

        self.small_size = small_size;
        self.large_size = large_size;
    }

    pub fn color_at(&self, ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer)
        -> Option<(Color, Priority, bool)>
    {
        let enabled = match screen_layer {
            ScreenLayer::MainScreen => self.main_screen_enabled,
            ScreenLayer::SubScreen => self.sub_screen_enabled
        };

        if !enabled {
            return None;
        }

        let pos_x = screen_x as isize;
        let pos_y = screen_y as isize;

        for object in ppu.oam().iter_objects() {
            if pos_x < object.pos_x || pos_y < object.pos_y {
                continue;
            }

            let size = match object.size_selector {
                SizeSelector::Small => self.small_size,
                SizeSelector::Large => self.large_size
            };

            if pos_x >= (object.pos_x + size.x) || pos_y >= (object.pos_y + size.y) {
                continue;
            }

            let offset_x = (pos_x - object.pos_x) as usize;
            let offset_y = (pos_y - object.pos_y) as usize;

            let pixel_x = if object.flip_x { (size.x as usize) - offset_x - 1 } else { offset_x };
            let pixel_y = if object.flip_y { (size.y as usize) - offset_y - 1 } else { offset_y };

            // Objects larger than 8x8 will map to multiple characters
            let row_offset = ((object.chr_index / TABLE_ROW_SIZE) + (pixel_y / CHR_SIZE)) * TABLE_ROW_SIZE;
            let column_offset = (object.chr_index + (pixel_x / CHR_SIZE)) % TABLE_ROW_SIZE;

            let chr_index = self.table_offsets[object.table_index] + row_offset + column_offset;

            let color_index = ppu.vram().chr_16(chr_index).pixel_at(pixel_x % CHR_SIZE, pixel_y % CHR_SIZE);

            if color_index != 0 {
                let color = ppu.cgram().color(object.palette_offset + (color_index as usize));
                return Some((color, object.priority, self.color_math_enabled));
            }
        }

        None
    }
}

impl ObjectSize {
    fn new(x: isize, y: isize) -> ObjectSize {
        ObjectSize {
            x: x,
            y: y
        }
    }
}
