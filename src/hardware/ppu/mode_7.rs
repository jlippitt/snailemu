use super::background_mode::Priority;
use super::ppu::Ppu;
use util::color::Color;

const CHR_SIZE: usize = 8;

pub struct Mode7;

impl Mode7 {
    pub fn new() -> Mode7 {
        Mode7
    }

    pub fn color_at(&self, ppu: &Ppu, screen_x: usize, screen_y: usize)
        -> Option<(Color, Priority, bool)>
    {
        // TODO: Scrolling, rotation, and all of that jazz
        let tile_x = screen_x / CHR_SIZE;
        let tile_y = screen_y / CHR_SIZE;

        let character = ppu.vram().mode_7_chr_at(tile_x, tile_y);

        let color_index = character.pixel_at(screen_x % CHR_SIZE, screen_y % CHR_SIZE);

        if color_index != 0 {
            Some((ppu.cgram().color(color_index as usize), 0, false))
        } else {
            None
        }
    }
}
