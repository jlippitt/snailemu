use std::slice::Iter;
use util::byte_access::{ByteAccess, ByteSelector};

const LOWER_TABLE_SIZE: usize = 256;
const UPPER_TABLE_SIZE: usize = 16;

const OBJECT_COUNT: usize = 128;

pub struct Oam {
    lower_table: Vec<u16>,
    upper_table: Vec<u16>,
    address: usize,
    lower_table_write_buffer: u8,
    table_selector: TableSelector,
    byte_selector: ByteSelector,
    objects: Vec<Object>
}

#[derive(Copy, Clone, Default)]
pub struct Object {
    pub pos_x: isize,
    pub pos_y: isize,
    pub chr_index: usize,
    pub table_index: usize,
    pub palette_offset: usize,
    pub priority: u8,
    pub flip_x: bool,
    pub flip_y: bool,
    pub size_selector: SizeSelector
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SizeSelector {
    Small,
    Large
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum TableSelector {
    Lower,
    Upper
}

#[inline]
fn absolute_offset(offset: usize, byte_selector: ByteSelector) -> usize {
    (offset << 1) | if byte_selector == ByteSelector::Upper { 1 } else { 0 }
}

impl Oam {
    pub fn new() -> Oam {
        Oam {
            lower_table: vec![0; LOWER_TABLE_SIZE],
            upper_table: vec![0; UPPER_TABLE_SIZE],
            address: 0,
            lower_table_write_buffer: 0x00,
            table_selector: TableSelector::Lower,
            byte_selector: ByteSelector::Lower,
            objects: vec![Default::default(); OBJECT_COUNT]
        }
    }

    pub fn set_address(&mut self, value: u8) {
        self.address = value as usize;
        self.byte_selector = ByteSelector::Lower;
    }

    pub fn set_table(&mut self, value: u8) {
        self.table_selector = match value & 0x01 {
            0x01 => TableSelector::Upper,
            _ => TableSelector::Lower
        };
    }

    pub fn read(&mut self) -> u8 {
        let value = match self.table_selector {
            TableSelector::Lower => {
                self.lower_table[self.address].get(self.byte_selector)
            },
            TableSelector::Upper => {
                let offset = self.address % UPPER_TABLE_SIZE;
                self.upper_table[offset].get(self.byte_selector)
            }
        };

        self.increment_address();

        value
    }

    pub fn write(&mut self, value: u8) {
        debug!("OAM Write ({:?} Table, {:?} Byte): {:02X} <= {:02X}",
            self.table_selector,
            self.byte_selector,
            self.address,
            value);

        match self.table_selector {
            TableSelector::Lower => {
                // Value is not actually written to lower table until upper byte is written
                match self.byte_selector {
                    ByteSelector::Lower => self.lower_table_write_buffer = value,
                    ByteSelector::Upper => {
                        let word_value = &mut self.lower_table[self.address];
                        word_value.set_lower(self.lower_table_write_buffer);
                        word_value.set_upper(value);
                    }
                };
                let absolute_offset = absolute_offset(self.address, self.byte_selector);
                self.update_cache_lower(absolute_offset, value);
            },
            TableSelector::Upper => {
                let offset = self.address % UPPER_TABLE_SIZE;
                self.upper_table[offset].set(self.byte_selector, value);
                let absolute_offset = absolute_offset(offset, self.byte_selector);
                self.update_cache_upper(absolute_offset, value);
            }
        };

        self.increment_address();
    }

    pub fn iter_objects(&self) -> Iter<Object> {
        self.objects.iter()
    }

    fn increment_address(&mut self) {
        match self.byte_selector {
            ByteSelector::Lower => {
                self.byte_selector = ByteSelector::Upper;
            },
            ByteSelector::Upper => {
                self.byte_selector = ByteSelector::Lower;
                self.address = self.address + 1;

                if self.address == LOWER_TABLE_SIZE {
                    self.address = 0;
                    self.table_selector = match self.table_selector {
                        TableSelector::Lower => TableSelector::Upper,
                        TableSelector::Upper => TableSelector::Lower
                    };
                }
            }
        }
    }

    fn update_cache_lower(&mut self, byte_address: usize, value: u8) {
        let object_index = byte_address / 4;
        let object = &mut self.objects[object_index];

        match byte_address % 4 {
            // Avoid setting sign bit for x position (this is a bit awkward)
            0 => object.pos_x = (((object.pos_x as usize) & !0xFF) | (value as usize)) as isize,
            1 => object.pos_y = if value < 240 { value as isize } else { (value as isize) - 303 },
            2 => object.chr_index = value as usize,
            3 => {
                object.table_index = (value & 0x01) as usize;
                object.palette_offset = 128 + (((value & 0x0E) << 3) as usize);
                object.priority = (value & 0x30) >> 4;
                object.flip_x = value & 0x40 != 0;
                object.flip_y = value & 0x80 != 0;
            },
            _ => unreachable!()
        }

        debug!("OBJ {}: X={}, Y={}, C={}, N={}, PL={}, PR={}, FX={}, FY={} S={:?}",
            object_index,
            object.pos_x,
            object.pos_y,
            object.chr_index,
            object.table_index,
            object.palette_offset,
            object.priority,
            object.flip_x,
            object.flip_y,
            object.size_selector);
    }

    fn update_cache_upper(&mut self, byte_address: usize, value: u8) {
        let first_object_index = byte_address * 4;

        for i in 0..4 {
            let object = &mut self.objects[first_object_index + i];
            let bits = (value & (0x03 << (i * 2))) >> (i * 2);

            // Set only sign bit for x position (this is even more awkward)
            object.pos_x = (((((bits & 0x01) as u16) << 15) | (((object.pos_x as i16) as u16) & 0x00FF)) as i16) as isize;

            object.size_selector = match bits & 0x02 {
                0x02 => SizeSelector::Large,
                _ => SizeSelector::Small
            };

            debug!("OBJ {}: X={}, Y={}, C={}, N={}, PL={}, PR={}, FX={}, FY={} S={:?}",
                first_object_index + i,
                object.pos_x,
                object.pos_y,
                object.chr_index,
                object.table_index,
                object.palette_offset,
                object.priority,
                object.flip_x,
                object.flip_y,
                object.size_selector);
        }
    }
}

impl Default for SizeSelector {
    fn default() -> SizeSelector {
        SizeSelector::Small
    }
}
