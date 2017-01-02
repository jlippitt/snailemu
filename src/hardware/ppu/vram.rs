use super::background_mode::Priority;
use util::byte_access::ByteAccess;

pub const TILE_MAP_COUNT: usize = VRAM_BYTE_SIZE / TILE_MAP_SIZE;

const VRAM_WORD_SIZE: usize = 32768;
const VRAM_BYTE_SIZE: usize = VRAM_WORD_SIZE * 2;

const TILE_MAP_ROW_WIDTH: usize = 32;
const TILE_MAP_ROW_COUNT: usize = 32;

const TILE_MAP_SIZE: usize = 2048;
const TILE_MAP_ROW_SIZE: usize = TILE_MAP_ROW_WIDTH * 2;

const CHR_ROW_WIDTH: usize = 8;
const CHR_ROW_COUNT: usize = 8;

const BIT_PLANE_SIZE: usize = 16;

const CHR_4_SIZE: usize = BIT_PLANE_SIZE;
const CHR_4_COUNT: usize = VRAM_BYTE_SIZE / CHR_4_SIZE;

const CHR_16_SIZE: usize = BIT_PLANE_SIZE * 2;
const CHR_16_COUNT: usize = VRAM_BYTE_SIZE / CHR_16_SIZE;

const CHR_256_SIZE: usize = BIT_PLANE_SIZE * 4;
const CHR_256_COUNT: usize = VRAM_BYTE_SIZE / CHR_256_SIZE;

const MODE_7_TILE_MAP_ROW_WIDTH: usize = 128;
const MODE_7_TILE_MAP_ROW_COUNT: usize = 128;

const MODE_7_TILE_MAP_SIZE: usize = MODE_7_TILE_MAP_ROW_WIDTH * MODE_7_TILE_MAP_ROW_COUNT;
const MODE_7_CHR_COUNT: usize = 256;

const MODE_7_CHR_COL_SIZE: usize = 2;
const MODE_7_CHR_ROW_SIZE: usize = MODE_7_CHR_COL_SIZE * 8;
const MODE_7_CHR_SIZE: usize = MODE_7_CHR_ROW_SIZE * 8;

pub struct Vram {
    raw_data: Vec<u16>,
    address: usize,
    read_buffer: u16,
    remap_mode: RemapMode,
    increment_mode: IncrementMode,
    increment_amount: usize,
    tile_maps: Vec<TileMap>,
    chr_4_map: Vec<Character>,
    chr_16_map: Vec<Character>,
    chr_256_map: Vec<Character>,
    mode_7_tile_map: Vec<usize>,
    mode_7_chr_map: Vec<Character>
}

#[derive(Copy, Clone, Default)]
pub struct TileMap {
    tiles: [[Tile; TILE_MAP_ROW_WIDTH]; TILE_MAP_ROW_COUNT]
}

#[derive(Copy, Clone, Default)]
pub struct Tile {
    pub chr_index: usize,
    pub palette_index: usize,
    pub priority: Priority,
    pub flip_x: bool,
    pub flip_y: bool
}

#[derive(Copy, Clone, Default)]
pub struct Character {
    pixels: [[u8; CHR_ROW_WIDTH]; CHR_ROW_COUNT]
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum IncrementMode {
    LowByte,
    HighByte
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum RemapMode {
    NoRemap,
    Remap1,
    Remap2,
    Remap3
}

impl Vram {
    pub fn new() -> Vram {
        Vram {
            raw_data: vec![0; VRAM_WORD_SIZE],
            address: 0,
            read_buffer: 0x0000,
            remap_mode: RemapMode::NoRemap,
            increment_mode: IncrementMode::LowByte,
            increment_amount: 1,
            tile_maps: vec![Default::default(); TILE_MAP_COUNT],
            chr_4_map: vec![Default::default(); CHR_4_COUNT],
            chr_16_map: vec![Default::default(); CHR_16_COUNT],
            chr_256_map: vec![Default::default(); CHR_256_COUNT],
            mode_7_tile_map: vec![Default::default(); MODE_7_TILE_MAP_SIZE],
            mode_7_chr_map: vec![Default::default(); MODE_7_CHR_COUNT]
        }
    }
    
    pub fn set_port_control(&mut self, value: u8) {
        self.remap_mode = match value & 0x0C {
            0x00 => RemapMode::NoRemap,
            0x04 => RemapMode::Remap1,
            0x08 => RemapMode::Remap2,
            0x0C => RemapMode::Remap3,
            _ => unreachable!()
        };

        self.increment_mode = match value & 0x80 {
            0x80 => IncrementMode::HighByte,
            _ => IncrementMode::LowByte
        };

        self.increment_amount = match value & 0x03 {
            00 => 1,
            01 => 32,
            _ => 128
        };
    }

    pub fn set_lower_address_byte(&mut self, value: u8) {
        self.address = (self.address & 0xFF00) | (value as usize);
    }

    pub fn set_upper_address_byte(&mut self, value: u8) {
        self.address = (self.address & 0x00FF) | ((value as usize) << 8);
    }

    pub fn read_low_byte(&mut self) -> u8 {
        let value = self.read_buffer.lower();
        if self.increment_mode == IncrementMode::LowByte {
            let mapped_address = self.mapped_address();
            self.read_buffer = self.raw_data[mapped_address];
            self.address += self.increment_amount;
        }
        value
    }

    pub fn read_high_byte(&mut self) -> u8 {
        let value = self.read_buffer.upper();
        if self.increment_mode == IncrementMode::HighByte {
            let mapped_address = self.mapped_address();
            self.read_buffer = self.raw_data[mapped_address];
            self.address += self.increment_amount;
        }
        value
    }

    pub fn write_low_byte(&mut self, value: u8) {
        let mapped_address = self.mapped_address();
        debug!("VRAM Write (Low): {:04X} <= {:02X}", mapped_address, value);
        self.raw_data[mapped_address].set_lower(value);
        self.update_cache(mapped_address << 1, value);
        if self.increment_mode == IncrementMode::LowByte {
            self.address += self.increment_amount;
        }
    }

    pub fn write_high_byte(&mut self, value: u8) {
        let mapped_address = self.mapped_address();
        debug!("VRAM Write (High): {:04X} <= {:02X}", mapped_address, value);
        self.raw_data[mapped_address].set_upper(value);
        self.update_cache((mapped_address << 1) + 1, value);
        if self.increment_mode == IncrementMode::HighByte {
            self.address += self.increment_amount;
        }
    }

    pub fn tile_map(&self, index: usize) -> &TileMap {
        &self.tile_maps[index]
    }

    pub fn chr_4(&self, index: usize) -> &Character {
        &self.chr_4_map[index % CHR_4_COUNT]
    }

    pub fn chr_16(&self, index: usize) -> &Character {
        &self.chr_16_map[index % CHR_16_COUNT]
    }

    pub fn chr_256(&self, index: usize) -> &Character {
        &self.chr_256_map[index % CHR_256_COUNT]
    }

    // TODO: Should this return an option?
    pub fn mode_7_chr_at(&self, x: usize, y: usize) -> &Character {
        &self.mode_7_chr_map[self.mode_7_tile_map[y * MODE_7_TILE_MAP_ROW_WIDTH + x]]
    }

    fn mapped_address(&self) -> usize {
        let mapped_address = match self.remap_mode {
            RemapMode::NoRemap => self.address,
            RemapMode::Remap1 => {
                (self.address & 0xFF00) | ((self.address & 0x00E0) >> 5) | ((self.address & 0x001F) << 3)
            },
            RemapMode::Remap2 => {
                (self.address & 0xFE00) | ((self.address & 0x01C0) >> 6) | ((self.address & 0x003F) << 3)
            },
            RemapMode::Remap3 => {
                (self.address & 0xFC00) | ((self.address & 0x0380) >> 7) | ((self.address & 0x007F) << 3)
            }
        };

        // There is only 64K of VRAM, so high bit must wrap
        mapped_address & 0x7FFF
    }

    fn update_cache(&mut self, byte_address: usize, value: u8) {
        // Update background tile maps
        let tile_map_index = byte_address / TILE_MAP_SIZE;
        let tile_map = &mut self.tile_maps[tile_map_index];

        let row_index = (byte_address % TILE_MAP_SIZE) / TILE_MAP_ROW_SIZE;
        let row = &mut tile_map.tiles[row_index];

        let tile_index = (byte_address % TILE_MAP_ROW_SIZE) / 2;
        let tile = &mut row[tile_index];

        match byte_address % 2 {
            0 => tile.chr_index = (tile.chr_index & !0xFF) | (value as usize),
            1 => {
                // Set upper two bits of character index
                tile.chr_index = (((value & 0x03) as usize) << 8) | (tile.chr_index & 0xFF);
                tile.palette_index = ((value & 0x1C) >> 2) as usize;
                tile.priority = (value & 0x20) >> 5;
                tile.flip_x = (value & 0x40) != 0;
                tile.flip_y = (value & 0x80) != 0;
            },
            _ => unreachable!()
        }

        // Update character maps
        update_chr_cache(&mut self.chr_4_map, CHR_4_SIZE, byte_address, value);
        update_chr_cache(&mut self.chr_16_map, CHR_16_SIZE, byte_address, value);
        update_chr_cache(&mut self.chr_256_map, CHR_256_SIZE, byte_address, value);

        if byte_address < (VRAM_BYTE_SIZE / 2) {
            // Update Mode 7 maps
            match byte_address % 2 {
                0 => {
                    // Tile map data is in lower byte of each word
                    self.mode_7_tile_map[byte_address / 2] = value as usize;
                },
                1 => {
                    // Character data is in upper byte of each word
                    let chr_index = byte_address / MODE_7_CHR_SIZE;
                    let row_index = (byte_address % MODE_7_CHR_SIZE) / MODE_7_CHR_ROW_SIZE;
                    let column_index = (byte_address % MODE_7_CHR_ROW_SIZE) / MODE_7_CHR_COL_SIZE;
                    self.mode_7_chr_map[chr_index].pixels[row_index][column_index] = value;
                },
                _ => unreachable!()
            }
        }
    }
}

fn update_chr_cache(chr_map: &mut Vec<Character>, chr_size: usize, byte_address: usize, value: u8) {
    let chr_index = byte_address / chr_size;
    let character = &mut chr_map[chr_index];

    let byte_index = byte_address % chr_size;

    let row_index = (byte_index % BIT_PLANE_SIZE) / 2;
    let row = &mut character.pixels[row_index];

    let bit_index = (byte_index / BIT_PLANE_SIZE) * 2 + byte_index % 2;
    let bit_mask = 0x01 << bit_index;

    for (column_index, pixel) in row.iter_mut().enumerate() {
        if value & (0x80 >> column_index) != 0 {
            *pixel |= bit_mask;
        } else {
            *pixel &= !bit_mask;
        }
    }
}

impl TileMap {
    pub fn tile_at(&self, x: usize, y: usize) -> &Tile {
        &self.tiles[y][x]
    }
}

impl Character {
    pub fn pixel_at(&self, x: usize, y: usize) -> u8 {
        self.pixels[y][x]
    }
}
