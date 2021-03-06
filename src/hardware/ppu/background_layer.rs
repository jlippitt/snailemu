use super::background_mode::{Priority, ScreenLayer};
use super::ppu::Ppu;
use super::vram::TILE_MAP_COUNT;
use super::window::WindowMask;
use util::byte_access::WriteTwice;
use util::color::Color;

const TILE_MAP_SIZE: usize = 32;

pub struct BackgroundLayer {
    main_screen_enabled: bool,
    sub_screen_enabled: bool,
    color_math_enabled: bool,
    tile_map_locations: [usize; 4],
    chr_4_offset: usize,
    chr_16_offset: usize,
    chr_256_offset: usize,
    scroll_x: WriteTwice<u16>,
    scroll_y: WriteTwice<u16>,
    window_mask: WindowMask
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ColorMode {
    Color4,
    Color16,
    Color256
}

pub struct PixelOptions {
    pub color_mode: ColorMode,
    pub palette_offset: usize,
    pub always_wide: bool
}

impl BackgroundLayer {
    pub fn new() -> BackgroundLayer {
        BackgroundLayer {
            main_screen_enabled: false,
            sub_screen_enabled: false,
            color_math_enabled: false,
            tile_map_locations: [0; 4],
            chr_4_offset: 0,
            chr_16_offset: 0,
            chr_256_offset: 0,
            scroll_x: WriteTwice::new(0x0000, 0x03FF),
            scroll_y: WriteTwice::new(0x0000, 0x03FF),
            window_mask: WindowMask::new()
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

    pub fn set_tile_map_locations(&mut self, value: u8) {
        let base_location = ((value & 0xFC) >> 2) as usize;
        self.tile_map_locations[0] = base_location;

        match value & 0x03 {
            0x00 => {
                // 32x32: AAAA
                self.tile_map_locations[1] = base_location;
                self.tile_map_locations[2] = base_location;
                self.tile_map_locations[3] = base_location;
            },
            0x01 => {
                // 64x32: ABAB
                self.tile_map_locations[1] = base_location + 1;
                self.tile_map_locations[2] = base_location;
                self.tile_map_locations[3] = base_location + 1;
            },
            0x02 => {
                // 32x64: AABB
                self.tile_map_locations[1] = base_location;
                self.tile_map_locations[2] = base_location + 1;
                self.tile_map_locations[3] = base_location + 1;
            },
            0x03 => {
                // 64x64: ABCD
                self.tile_map_locations[1] = base_location + 1;
                self.tile_map_locations[2] = base_location + 2;
                self.tile_map_locations[3] = base_location + 3;
            },
            _ => unreachable!()
        }
    }

    pub fn set_chr_offset(&mut self, value: u8) {
        self.chr_4_offset = (value as usize) * 512;
        self.chr_16_offset = (value as usize) * 256;
        self.chr_256_offset = (value as usize) * 128;
    }

    pub fn set_scroll_x(&mut self, value: u8) {
        self.scroll_x.write(value);
    }

    pub fn set_scroll_y(&mut self, value: u8) {
        self.scroll_y.write(value);
    }

    pub fn set_window_mask_options(&mut self, value: u8) {
        self.window_mask.set_options(value);
    }

    pub fn set_window_mask_logic(&mut self, value: u8) {
        self.window_mask.set_operator(value);
    }

    pub fn color_at(&self, ppu: &Ppu, screen_x: usize, screen_y: usize, screen_layer: ScreenLayer, pixel_options: &PixelOptions)
        -> Option<(Color, Priority, bool)>
    {
        let enabled = match screen_layer {
            ScreenLayer::MainScreen => self.main_screen_enabled,
            ScreenLayer::SubScreen => self.sub_screen_enabled
        };

        if !enabled || self.window_mask.contains(ppu, screen_x) {
            return None;
        }

        let pos_x = screen_x + (self.scroll_x.value() as usize);
        let pos_y = screen_y + (self.scroll_y.value() as usize);

        // TODO: 16x16 tiles
        let tile_x = (pos_x / 8) % (TILE_MAP_SIZE * 2);
        let tile_y = (pos_y / 8) % (TILE_MAP_SIZE * 2);

        let tile_map_offset = (tile_x / TILE_MAP_SIZE) + 2 * (tile_y / TILE_MAP_SIZE);

        let tile_map_index = self.tile_map_locations[tile_map_offset] % TILE_MAP_COUNT;

        let tile = ppu.vram().tile_map(tile_map_index).tile_at(tile_x % TILE_MAP_SIZE, tile_y % TILE_MAP_SIZE);

        let mut pixel_x = if tile.flip_x { 7 - (pos_x % 8) } else { pos_x % 8 };

        // Deal with pseudo-hi-res modes
        if pixel_options.always_wide {
            pixel_x *= 2;
            if screen_layer == ScreenLayer::MainScreen {
                pixel_x += 1;
            }
        }

        let pixel_y = if tile.flip_y { 7 - (pos_y % 8) } else { pos_y % 8 };

        let chr_index = if pixel_x > 7 {
            tile.chr_index + 1
        } else {
            tile.chr_index
        };

        let (character, palette_size) = match pixel_options.color_mode {
            ColorMode::Color4 => (ppu.vram().chr_4(self.chr_4_offset + chr_index), 4),
            ColorMode::Color16 => (ppu.vram().chr_16(self.chr_16_offset + chr_index), 16),
            ColorMode::Color256 => (ppu.vram().chr_256(self.chr_256_offset + chr_index), 0)
        };

        let color_index = character.pixel_at(pixel_x % 8, pixel_y % 8);

        if color_index != 0 {
            let palette_offset = pixel_options.palette_offset + tile.palette_index * palette_size;
            let color = ppu.cgram().color(palette_offset + (color_index as usize));
            Some((color, tile.priority, self.color_math_enabled))
        } else {
            None
        }
    }
}

impl Default for PixelOptions {
    fn default() -> PixelOptions {
        PixelOptions {
            color_mode: ColorMode::Color256,
            palette_offset: 0,
            always_wide: false
        }
    }
}
