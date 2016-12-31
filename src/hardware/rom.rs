use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use super::hardware::HardwareBus;

const SMC_HEADER_SIZE: usize = 512;

pub struct Rom {
    mode: RomMode,
    data: DataBus,
    sram: SramBus
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RomMode {
    LoRom,
    HiRom
}

pub struct DataBus(Vec<u8>);

pub struct SramBus(Vec<u8>);

struct RomHeader {
    mode: RomMode,
    score: u32,
    rom_size: usize,
    sram_size: usize
}

impl Rom {
    pub fn new(path: &Path) -> Rom {
        let mut file = File::open(path).unwrap();
        let mut buffer = Vec::<u8>::new();
        let rom_size = file.read_to_end(&mut buffer).unwrap();

        let rom_data = match rom_size % 1024 {
            SMC_HEADER_SIZE => {
                info!("Valid SMC header found");
                buffer.split_off(SMC_HEADER_SIZE)
            },
            0 => {
                info!("No SMC header found");
                buffer
            },
            length @ _ => panic!("Invalid SMC header length: {}", length)
        };

        let lo_rom_header = RomHeader::new(&rom_data, RomMode::LoRom);
        let hi_rom_header = RomHeader::new(&rom_data, RomMode::HiRom);

        let header = if hi_rom_header.score() >= lo_rom_header.score() {
            hi_rom_header
        } else {
            lo_rom_header
        };

        if header.score() > 0 {
            info!("{} mode detected", header.mode());
            info!("ROM size: {}", header.rom_size());
            info!("SRAM size: {}", header.sram_size());

            Rom {
                mode: header.mode(),
                data: DataBus(rom_data),
                sram: SramBus(vec![0; header.sram_size()])
            }
        } else {
            panic!("Could not locate valid LoROM or HiROM header");
        }
    }

    pub fn mode(&self) -> RomMode {
        self.mode
    }

    pub fn data(&mut self) -> &mut DataBus {
        &mut self.data
    }

    pub fn sram(&mut self) -> &mut SramBus {
        &mut self.sram
    }
}

impl Display for RomMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            RomMode::LoRom => "LoROM",
            RomMode::HiRom => "HiROM"
        })
    }
}

impl HardwareBus for DataBus {
    fn read(&mut self, offset: usize) -> u8 {
        self.0[offset]
    }

    fn write(&mut self, _offset: usize, _value: u8) {
        // Not writable
    }
}

impl HardwareBus for SramBus {
    fn read(&mut self, offset: usize) -> u8 {
        self.0[offset]
    }

    fn write(&mut self, offset: usize, value: u8) {
        self.0[offset] = value;
    }
}

impl RomHeader {
    fn new(rom_data: &Vec<u8>, mode: RomMode) -> RomHeader {
        let mut valid = true;
        let mut score = 0;

        let header = match mode {
            RomMode::LoRom => &rom_data[0x7F00..0x8000],
            RomMode::HiRom => &rom_data[0xFF00..0x10000]
        };

        // Check for valid reset vector
        let reset_vector = header[0xFD];

        if reset_vector >= 0x80 && reset_vector != 0xFF {
            score += 1;
        } else {
            // Even if other bits are (coincidentally) correct, the ROM is still not valid
            valid = false;
        }

        // Check the reported ROM mode matches the mode we're expecting
        let expected_rom_mode = match header[0xD5] & 0x01 {
            0 => RomMode::LoRom,
            1 => RomMode::HiRom,
            _ => unreachable!()
        };

        if expected_rom_mode == mode {
            score += 1;
        }

        // Check the ROM size is correct
        let rom_size = match 0x400_usize.checked_shl(header[0xD7] as u32) {
            Some(rom_size) => {
                if rom_size == rom_data.len() {
                    score += 1;
                }
                rom_size
            },
            None => 0
        };

        let sram_size = match header[0xD6] & 0x0F {
            0x01 | 0x02 => 0x400_usize.checked_shl(header[0xD8] as u32).unwrap_or(0),
            _ => 0
        };

        if !valid {
            score = 0;
        }

        debug!("{} score: {}", mode, score);

        RomHeader {
            mode: mode,
            score: score,
            rom_size: rom_size,
            sram_size: sram_size
        }
    }

    fn mode(&self) -> RomMode {
        self.mode
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn rom_size(&self) -> usize {
        self.rom_size
    }

    fn sram_size(&self) -> usize {
        self.sram_size
    }
}
