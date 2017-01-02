use std::fmt::{self, Display, Formatter};
use std::rc::Rc;
use super::apu::Apu;
use super::dma::{self, DmaChannel, DMA_CHANNEL_COUNT};
use super::io_port::IoPort;
use super::joypad::Joypad;
use super::ppu::Ppu;
use super::registers::HardwareRegs;
use super::rom::{Rom, RomMode};
use super::wram::Wram;
use util::byte_access::ByteAccess;

const FAST_CYCLES: u64 = 6;
const SLOW_CYCLES: u64 = 8;
const EXTRA_SLOW_CYCLES: u64 = 12;

pub trait MemoryAccess {
    fn read(hardware: &mut Hardware, address: HardwareAddress) -> Self;
    fn write(hardware: &mut Hardware, address: HardwareAddress, value: Self);
    fn size(&self) -> u16;
}

pub trait HardwareBus {
    fn read(&mut self, offset: usize) -> u8;
    fn write(&mut self, offset: usize, value: u8);
}

pub struct Hardware {
    rom: Rom,
    wram: Wram,
    ppu: Ppu,
    apu: Apu,
    joypad: Joypad,
    regs: HardwareRegs,
    dma_channels: [DmaChannel; DMA_CHANNEL_COUNT],
    open_bus: OpenBus,
    clock: u64
}

#[derive(Copy, Clone)]
pub struct HardwareAddress {
    bank: u8,
    offset: u16
}

struct MemoryLocation<'a> {
    bus: &'a mut HardwareBus,
    offset: usize,
    cycles: u64
}

struct OpenBus;

#[inline]
fn rom20(address: HardwareAddress) -> usize {
    0x8000 * (address.bank() & 0x7F) as usize + (address.offset() & 0x7FFF) as usize
}

#[inline]
fn rom21(address: HardwareAddress) -> usize {
    0x10000 * (address.bank() & 0x3F) as usize + address.offset() as usize
}

#[inline]
fn sram20(address: HardwareAddress) -> usize {
    0x8000 * (address.bank() & 0x0F) as usize + (address.offset() & 0x7FFF) as usize
}

#[inline]
fn sram21(address: HardwareAddress) -> usize {
    0x2000 * (address.bank() & 0x1F) as usize + (address.offset() & 0x1FFF) as usize
}

impl Hardware {
    pub fn new(rom: Rom, wram: Wram, ppu: Ppu, apu: Apu, joypad: Joypad, io_port: Rc<IoPort>) -> Hardware {
        Hardware {
            rom: rom,
            wram: wram,
            ppu: ppu,
            apu: apu,
            joypad: joypad,
            regs: HardwareRegs::new(io_port),
            dma_channels: [
                DmaChannel::new(), DmaChannel::new(),
                DmaChannel::new(), DmaChannel::new(),
                DmaChannel::new(), DmaChannel::new(),
                DmaChannel::new(), DmaChannel::new()
            ],
            open_bus: OpenBus,
            clock: 0
        }
    }

    pub fn regs(&self) -> &HardwareRegs {
        &self.regs
    }

    pub fn regs_mut(&mut self) -> &mut HardwareRegs {
        &mut self.regs
    }

    pub fn joypad(&self) -> &Joypad {
        &self.joypad
    }

    pub fn joypad_mut(&mut self) -> &mut Joypad {
        &mut self.joypad
    }

    pub fn dma_channel(&self, index: usize) -> &DmaChannel {
        &self.dma_channels[index]
    }

    pub fn dma_channel_mut(&mut self, index: usize) -> &mut DmaChannel {
        &mut self.dma_channels[index]
    }

    pub fn clock(&self) -> u64 {
        self.clock
    }

    pub fn read<T: MemoryAccess>(&mut self, address: HardwareAddress) -> T {
        T::read(self, address)
    }

    pub fn write<T: MemoryAccess>(&mut self, address: HardwareAddress, value: T) {
        T::write(self, address, value);
    }

    pub fn dma_transfer(&mut self, channel_mask: u8) {
        dma::dma_transfer(self, channel_mask)
    }

    pub fn tick(&mut self, cycles: u64) {
        self.ppu.add_cycles(cycles);

        while self.ppu.next_pixel() {
            self.regs.update(&mut self.ppu, &self.joypad);
        }

        self.clock = self.clock.wrapping_add(cycles);
    }

    fn read_u8(&mut self, address: HardwareAddress) -> u8 {
        let (value, cycles) = {
            let mut location = self.byte_at(address);
            (location.read(), location.cycles())
        };
        debug!("Read: {} => {:02X}", address, value);
        self.tick(cycles);
        value
    }

    fn write_u8(&mut self, address: HardwareAddress, value: u8) {
        debug!("Write: {} <= {:02X}", address, value);
        let cycles = {
            let mut location = self.byte_at(address);
            location.write(value);
            location.cycles()
        };
        self.tick(cycles);
    }

    fn byte_at(&mut self, address: HardwareAddress) -> MemoryLocation {
        let bank = address.bank();
        let offset = address.offset();

        let (bus, offset, cycles): (&mut HardwareBus, usize, u64) = if bank & 0x40 != 0 {
            // Full ROM/RAM mode
            match bank {
                0x7E => (self.wram.data(), offset as usize, SLOW_CYCLES),
                0x7F => (self.wram.data(), 0x10000 | (offset as usize), SLOW_CYCLES),
                _ => {
                    // TODO: ROM speed
                    match self.rom.mode() {
                        RomMode::LoRom => {
                            if offset & 0x8000 != 0 {
                                (self.rom.data(), rom20(address), SLOW_CYCLES)
                            } else if (bank & 0x70) == 0x70 {
                                (self.rom.sram(), sram20(address), SLOW_CYCLES)
                            } else {
                                (&mut self.open_bus, 0, FAST_CYCLES)
                            }
                        },
                        RomMode::HiRom => (self.rom.data(), rom21(address), SLOW_CYCLES)
                    }
                }
            }
        } else {
            // Hybrid mode
            match offset & 0xE000 {
                0x0000 => (self.wram.data(), offset as usize, SLOW_CYCLES),
                0x2000 => {
                    // APU, PPU, etc.
                    match offset & 0xFFC0 {
                        0x2100 => (&mut self.ppu, (offset & 0x003F) as usize, FAST_CYCLES),
                        0x2140 => (&mut self.apu, (offset & 0x0003) as usize, FAST_CYCLES),
                        0x2180 => (&mut self.wram, (offset & 0x003F) as usize, FAST_CYCLES),
                        _ => (&mut self.open_bus, 0, FAST_CYCLES)
                    }
                },
                0x4000 => {
                    // System registers, DMA control and NES-style joypad registers
                    match offset & 0xFF80 {
                        0x4200 => (&mut self.regs, (offset & 0x007F) as usize, FAST_CYCLES),
                        0x4300 => {
                            let index = ((offset & 0x0070) >> 4) as usize;
                            (&mut self.dma_channels[index], (offset & 0x000F) as usize, FAST_CYCLES)
                        },
                        0x4000 => (&mut self.joypad, (offset & 0x007F) as usize, EXTRA_SLOW_CYCLES),
                        _ => (&mut self.open_bus, 0, FAST_CYCLES)
                    }
                },
                0x6000 => {
                    // SRAM (but only in HiROM mode)
                    if self.rom.mode() == RomMode::HiRom && bank & 0x20 == 0x20 {
                        (self.rom.sram(), sram21(address), SLOW_CYCLES)
                    } else {
                        (&mut self.open_bus, 0, SLOW_CYCLES)
                    }
                },
                _ => {
                    // ROM data
                    // TODO: ROM speed
                    let rom_offset = match self.rom.mode() {
                        RomMode::LoRom => rom20(address),
                        RomMode::HiRom => rom21(address)
                    };
                    (self.rom.data(), rom_offset, SLOW_CYCLES)
                }
            }
        };

        MemoryLocation::new(bus, offset, cycles)
    }
}

impl HardwareAddress {
    pub fn new(bank: u8, offset: u16) -> HardwareAddress {
        HardwareAddress {
            bank: bank,
            offset: offset
        }
    }

    pub fn bank(&self) -> u8 {
        self.bank
    }

    pub fn set_bank(&mut self, bank: u8) {
        self.bank = bank;
    }

    pub fn offset(&self) -> u16 {
        self.offset
    }
    
    pub fn set_offset(&mut self, offset: u16) {
        self.offset = offset;
    }

    pub fn offset_mut(&mut self) -> &mut u16 {
        &mut self.offset
    }

    pub fn wrapping_add(self, rhs: u16) -> Self {
        let mut bank = self.bank;
        let offset = self.offset.wrapping_add(rhs);
        if offset < self.offset {
            bank = bank.wrapping_add(1);
        }
        Self::new(bank, offset)
    }
}

impl Display for HardwareAddress {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:02X}:{:04X}", self.bank, self.offset)
    }
}

impl<'a> MemoryLocation<'a> {
    pub fn new(bus: &'a mut HardwareBus, offset: usize, cycles: u64) -> MemoryLocation<'a> {
        MemoryLocation {
            bus: bus,
            offset: offset,
            cycles: cycles
        }
    }

    pub fn read(&mut self) -> u8 {
        self.bus.read(self.offset)
    }

    pub fn write(&mut self, value: u8) {
        self.bus.write(self.offset, value);
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }
}

impl HardwareBus for OpenBus {
    fn read(&mut self, _offset: usize) -> u8 {
        0
    }

    fn write(&mut self, _offset: usize, _value: u8) {
        // Nothing
    }
}

impl MemoryAccess for u8 {
    fn read(hardware: &mut Hardware, address: HardwareAddress) -> u8 {
        hardware.read_u8(address)
    }

    fn write(hardware: &mut Hardware, address: HardwareAddress, value: u8) {
        hardware.write_u8(address, value);
    }

    fn size(&self) -> u16 {
        1
    }
}

// TODO: Wrapping
impl MemoryAccess for u16 {
    fn read(hardware: &mut Hardware, address: HardwareAddress) -> u16 {
        let lower = hardware.read_u8(address);
        let upper_offset = address.offset().wrapping_add(1);
        let upper = hardware.read_u8(HardwareAddress::new(address.bank(), upper_offset));
        ((upper as u16) << 8) | (lower as u16)
    }

    fn write(hardware: &mut Hardware, address: HardwareAddress, value: u16) {
        hardware.write_u8(address, value.lower());
        let upper_offset = address.offset().wrapping_add(1);
        hardware.write_u8(HardwareAddress::new(address.bank(), upper_offset), value.upper());
    }

    fn size(&self) -> u16 {
        2
    }
}

impl MemoryAccess for HardwareAddress {
    fn read(hardware: &mut Hardware, address: HardwareAddress) -> HardwareAddress {
        let offset = hardware.read::<u16>(address);
        let bank_address = HardwareAddress::new(address.bank(), address.offset().wrapping_add(2));
        let bank = hardware.read::<u8>(bank_address);
        HardwareAddress::new(bank, offset)
    }

    fn write(hardware: &mut Hardware, address: HardwareAddress, value: HardwareAddress) {
        hardware.write(address, value.offset());
        let bank_address = HardwareAddress::new(address.bank(), address.offset().wrapping_add(2));
        hardware.write(bank_address, value.bank());
    }

    fn size(&self) -> u16 {
        3
    }
}
