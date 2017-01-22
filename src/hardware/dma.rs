use super::hardware::{Hardware, HardwareAddress, HardwareBus};
use util::byte_access::ByteAccess;

pub const DMA_CHANNEL_COUNT: usize = 8;

const DMA_CYCLES: u64 = 8;

#[derive(Clone)]
pub struct DmaChannel {
    reverse_transfer: bool,
    hdma_indirect_mode: bool,
    increment_type: IncrementType,
    transfer_mode: TransferMode,
    raw_control_value: u8,
    destination: u16,
    source: HardwareAddress,
    hdma_indirect_address: HardwareAddress,
    hdma_table_address: HardwareAddress,
    hdma_line_counter: HdmaLineCounter,
    hdma_active: bool
}

#[derive(Copy, Clone)]
enum IncrementType {
    Increment,
    Decrement,
    Fixed
}

#[derive(Copy, Clone)]
enum TransferMode {
    A,
    AB,
    AA,
    AABB,
    ABCD,
    ABAB
}

#[derive(Clone)]
enum HdmaLineCounter {
    Count(u8),
    Repeat(u8)
}

struct TransferModeIterator {
    transfer_mode: TransferMode,
    phase: u16
}

impl DmaChannel {
    pub fn new() -> DmaChannel {
        DmaChannel {
            reverse_transfer: false,
            hdma_indirect_mode: false,
            increment_type: IncrementType::Increment,
            transfer_mode: TransferMode::A,
            raw_control_value: 0,
            destination: 0x2100,
            source: HardwareAddress::new(0x00, 0x0000),
            hdma_indirect_address: HardwareAddress::new(0x00, 0x0000),
            hdma_table_address: HardwareAddress::new(0x00, 0x0000),
            hdma_line_counter: HdmaLineCounter::Repeat(0x7F),
            hdma_active: false
        }
    }
}

impl HardwareBus for DmaChannel {
    fn read(&mut self, offset: usize) -> u8 {
        match offset {
            0x00 => self.raw_control_value,
            0x01 => self.destination.lower(),
            0x02 => self.source.offset().lower(),
            0x03 => self.source.offset().upper(),
            0x04 => self.source.bank(),
            0x05 => self.hdma_indirect_address.offset().lower(),
            0x06 => self.hdma_indirect_address.offset().upper(),
            0x07 => self.hdma_indirect_address.bank(),
            0x08 => self.hdma_table_address.offset().lower(),
            0x09 => self.hdma_table_address.offset().upper(),
            0x0A => {
                match self.hdma_line_counter {
                    HdmaLineCounter::Repeat(count) => 0x80_u8.wrapping_add(count),
                    HdmaLineCounter::Count(count) => 0x00_u8.wrapping_add(count)
                }
            },
            _ => 0x00 // TODO: Open bus
        }
    }

    fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0x00 => {
                self.reverse_transfer = value & 0x80 != 0;
                self.hdma_indirect_mode = value & 0x40 != 0;

                self.increment_type = match value & 0x18 {
                    0x00 => IncrementType::Increment,
                    0x10 => IncrementType::Decrement,
                    _ => IncrementType::Fixed
                };

                self.transfer_mode = match value & 0x07 {
                    0x00 => TransferMode::A,
                    0x01 => TransferMode::AB,
                    0x02 | 0x06 => TransferMode::AA,
                    0x03 | 0x07 => TransferMode::AABB,
                    0x04 => TransferMode::ABCD,
                    0x05 => TransferMode::ABAB,
                    _ => unreachable!()
                };

                self.raw_control_value = value;
            },
            0x01 => self.destination.set_lower(value),
            0x02 => self.source.offset_mut().set_lower(value),
            0x03 => self.source.offset_mut().set_upper(value),
            0x04 => self.source.set_bank(value),
            0x05 => self.hdma_indirect_address.offset_mut().set_lower(value),
            0x06 => self.hdma_indirect_address.offset_mut().set_upper(value),
            0x07 => self.hdma_indirect_address.set_bank(value),
            0x08 => self.hdma_table_address.offset_mut().set_lower(value),
            0x09 => self.hdma_table_address.offset_mut().set_upper(value),
            0x0A => {
                self.hdma_line_counter = if value.wrapping_sub(0x01) & 0x80 != 0 {
                    HdmaLineCounter::Repeat(value.wrapping_sub(0x80))
                } else {
                    HdmaLineCounter::Count(value)
                }
            },
            _ => ()
        }
    }
}

pub fn dma_transfer(hardware: &mut Hardware, channel_mask: u8) {
    hardware.tick(DMA_CYCLES);

    for i in 0..DMA_CHANNEL_COUNT {
        if channel_mask & (0x01 << i) == 0 {
            continue;
        }

        let mut channel = hardware.dma_channel(i).clone();
        
        if channel.hdma_active {
            continue;
        }

        hardware.tick(DMA_CYCLES);

        let mut count = channel.hdma_indirect_address.offset();

        debug!("DMA Transfer Start (Channel {}): C={:02X} D={:02X} S={} C={:04X}",
            i + 1,
            channel.raw_control_value,
            channel.destination as u8,
            channel.source,
            count);

        for offset in channel.transfer_mode.iter() {
            let destination = HardwareAddress::new(0x00, channel.destination + offset);

            let (src, dst) = if channel.reverse_transfer {
                (destination, channel.source)
            } else {
                (channel.source, destination)
            };

            hardware.transfer(src, dst);
            hardware.tick(DMA_CYCLES);

            count = count.wrapping_sub(1);

            if count == 0x00 {
                break
            }

            match channel.increment_type {
                IncrementType::Increment => {
                    let offset = channel.source.offset();
                    channel.source.set_offset(offset.wrapping_add(1));
                },
                IncrementType::Decrement => {
                    let offset = channel.source.offset();
                    channel.source.set_offset(offset.wrapping_sub(1));
                },
                _ => ()
            }
        }

        channel.hdma_indirect_address.set_offset(count);

        *hardware.dma_channel_mut(i) = channel;

        debug!("DMA Transfer End (Channel {})", i);
    }
}

impl TransferMode {
    fn iter(&self) -> TransferModeIterator {
        TransferModeIterator {
            transfer_mode: *self,
            phase: 0
        }
    }

    fn len(&self) -> u16 {
        match *self {
            TransferMode::A => 1,
            TransferMode::AB => 2,
            TransferMode::AA => 2,
            TransferMode::AABB => 4,
            TransferMode::ABCD => 4,
            TransferMode::ABAB => 4
        }
    }
}

impl Iterator for TransferModeIterator {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        let offset = match self.transfer_mode {
            TransferMode::A => self.phase,
            TransferMode::AB => self.phase, 
            TransferMode::AA => self.phase / 2,
            TransferMode::AABB => self.phase / 2,
            TransferMode::ABCD => self.phase,
            TransferMode::ABAB => self.phase % 2
        };

        self.phase = (self.phase + 1) % self.transfer_mode.len();

        Some(offset)
    }
}
