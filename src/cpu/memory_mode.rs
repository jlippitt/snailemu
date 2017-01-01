use cpu::Cpu;
use hardware::HardwareAddress;
use std::fmt::{self, Formatter};
use util::byte_access::ByteAccess;

pub trait MemoryMode {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress);
    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result;
}

#[derive(Default)]
pub struct Absolute;

#[derive(Default)]
pub struct AbsoluteIndexedX;

#[derive(Default)]
pub struct AbsoluteIndexedXIndirect;

#[derive(Default)]
pub struct AbsoluteIndexedY;

#[derive(Default)]
pub struct AbsoluteIndirect;

#[derive(Default)]
pub struct AbsoluteIndirectLong;

#[derive(Default)]
pub struct AbsoluteLong;

#[derive(Default)]
pub struct AbsoluteLongIndexedX;

#[derive(Default)]
pub struct DirectPage;

#[derive(Default)]
pub struct DirectPageIndexedX;

#[derive(Default)]
pub struct DirectPageIndexedXIndirect;

#[derive(Default)]
pub struct DirectPageIndexedY;

#[derive(Default)]
pub struct DirectPageIndirect;

#[derive(Default)]
pub struct DirectPageIndirectIndexedY;

#[derive(Default)]
pub struct DirectPageIndirectLong;

#[derive(Default)]
pub struct DirectPageIndirectLongIndexedY;

#[derive(Default)]
pub struct ProgramCounterRelative;

#[derive(Default)]
pub struct StackRelative;

#[derive(Default)]
pub struct StackRelativeIndirectIndexedY;

impl MemoryMode for Absolute {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let bank = cpu.regs().data_bank;
        let immediate = HardwareAddress::new(bank, cpu.read_next::<u16>());
        (immediate, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:04X}", immediate.offset())
    }
}

impl MemoryMode for AbsoluteIndexedX {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let bank = cpu.regs().data_bank;
        let immediate = HardwareAddress::new(bank, cpu.read_next::<u16>());
        let resolved = HardwareAddress::new(bank, immediate.offset().wrapping_add(cpu.regs().index_x));
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:04X},X", immediate.offset())
    }
}

impl MemoryMode for AbsoluteIndexedXIndirect {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let program_bank = cpu.regs().program_bank;
        let immediate = HardwareAddress::new(program_bank, cpu.read_next::<u16>());
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().index_x);
        let adjusted = HardwareAddress::new(program_bank, adjusted_offset);
        let resolved_offset = cpu.hardware_mut().read::<u16>(adjusted);
        let resolved = HardwareAddress::new(program_bank, resolved_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "(${:04X},X)", immediate.offset())
    }
}

impl MemoryMode for AbsoluteIndexedY {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let bank = cpu.regs().data_bank;
        let immediate = HardwareAddress::new(bank, cpu.read_next::<u16>());
        let resolved = HardwareAddress::new(bank, immediate.offset().wrapping_add(cpu.regs().index_y));
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:04X},Y", immediate.offset())
    }
}

impl MemoryMode for AbsoluteIndirect {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let program_bank = cpu.regs().program_bank;
        // Address lookup is always in bank 0 (for whatever reason)
        let immediate = HardwareAddress::new(0, cpu.read_next::<u16>());
        let resolved_offset = cpu.hardware_mut().read::<u16>(immediate);
        let resolved = HardwareAddress::new(program_bank, resolved_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "(${:04X})", immediate.offset())
    }
}

impl MemoryMode for AbsoluteIndirectLong {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let program_bank = cpu.regs().program_bank;
        let immediate = HardwareAddress::new(program_bank, cpu.read_next::<u16>());
        let resolved = cpu.hardware_mut().read::<HardwareAddress>(immediate);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "[${:04X}]", immediate.offset())
    }
}

impl MemoryMode for AbsoluteLong {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = cpu.read_next::<HardwareAddress>();
        (immediate, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${}", immediate)
    }
}

impl MemoryMode for AbsoluteLongIndexedX {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = cpu.read_next::<HardwareAddress>();
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().index_x);
        let resolved = HardwareAddress::new(immediate.bank(), adjusted_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${},X", immediate)
    }
}

impl MemoryMode for DirectPage {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().direct_page);
        let resolved = HardwareAddress::new(0, adjusted_offset);
        cpu.direct_page_cycle();
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:02X}", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndexedX {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset()
            .wrapping_add(cpu.regs().direct_page)
            .wrapping_add(cpu.regs().index_x);
        cpu.direct_page_cycle();
        let resolved = HardwareAddress::new(0, adjusted_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:02X},X", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndexedXIndirect {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset()
            .wrapping_add(cpu.regs().direct_page)
            .wrapping_add(cpu.regs().index_x);
        cpu.direct_page_cycle();
        let indirect = HardwareAddress::new(0, adjusted_offset);
        let resolved_offset = cpu.hardware_mut().read::<u16>(indirect);
        let resolved = HardwareAddress::new(0, resolved_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "(${:02X},X)", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndexedY {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset()
            .wrapping_add(cpu.regs().direct_page)
            .wrapping_add(cpu.regs().index_y);
        cpu.direct_page_cycle();
        let resolved = HardwareAddress::new(0, adjusted_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:02X},Y", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndirect {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().direct_page);
        cpu.direct_page_cycle();
        let indirect = HardwareAddress::new(0, adjusted_offset);
        let resolved_offset = cpu.hardware_mut().read::<u16>(indirect);
        let resolved = HardwareAddress::new(cpu.regs().data_bank, resolved_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "(${:02X})", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndirectIndexedY {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().direct_page);
        cpu.direct_page_cycle();
        let indirect = HardwareAddress::new(0, adjusted_offset);
        let resolved_offset = cpu.hardware_mut().read::<u16>(indirect);
        let indexed_offset = resolved_offset.wrapping_add(cpu.regs().index_y);
        let indexed = HardwareAddress::new(cpu.regs().data_bank, indexed_offset);
        (indexed, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "(${:02X}),Y", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndirectLong {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().direct_page);
        cpu.direct_page_cycle();
        let indirect = HardwareAddress::new(0, adjusted_offset);
        let resolved = cpu.hardware_mut().read::<HardwareAddress>(indirect);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "[${:02X}]", immediate.offset().lower())
    }
}

impl MemoryMode for DirectPageIndirectLongIndexedY {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        let adjusted_offset = immediate.offset().wrapping_add(cpu.regs().direct_page);
        cpu.direct_page_cycle();
        let indirect = HardwareAddress::new(0, adjusted_offset);
        let resolved = cpu.hardware_mut().read::<HardwareAddress>(indirect);
        let indexed_offset = resolved.offset().wrapping_add(cpu.regs().index_y);
        let indexed = HardwareAddress::new(resolved.bank(), indexed_offset);
        (indexed, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "[${:02X}],Y", immediate.offset().lower())
    }
}

impl MemoryMode for ProgramCounterRelative {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let bank = cpu.regs().data_bank;
        let immediate = HardwareAddress::new(bank, cpu.read_next::<u16>());
        let adjusted_offset = cpu.regs().program_counter.wrapping_add(immediate.offset());
        let resolved = HardwareAddress::new(bank, adjusted_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:04X},PC", immediate.offset())
    }
}

impl MemoryMode for StackRelative {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        // TODO: Emulation mode stack location
        let adjusted_offset = cpu.regs().stack_pointer.wrapping_add(immediate.offset());
        let resolved = HardwareAddress::new(0, adjusted_offset);
        (resolved, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "${:02X},S", immediate.offset().lower())
    }
}

impl MemoryMode for StackRelativeIndirectIndexedY {
    fn resolve(cpu: &mut Cpu) -> (HardwareAddress, HardwareAddress) {
        let immediate = HardwareAddress::new(0, cpu.read_next::<u8>() as u16);
        // TODO: Emulation mode stack location
        let adjusted_offset = cpu.regs().stack_pointer.wrapping_add(immediate.offset());
        let indirect = HardwareAddress::new(0, adjusted_offset);
        let resolved_offset = cpu.hardware_mut().read::<u16>(indirect);
        let indexed_offset = resolved_offset.wrapping_add(cpu.regs().index_y);
        let indexed = HardwareAddress::new(cpu.regs().data_bank, indexed_offset);
        (indexed, immediate)
    }

    fn format(f: &mut Formatter, immediate: HardwareAddress) -> fmt::Result {
        write!(f, "(${:02X},S),Y", immediate.offset().lower())
    }
}
