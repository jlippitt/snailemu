use cpu::accessor::*;
use cpu::address_mode::*;
use cpu::interrupt::*;
use cpu::register::*;
use cpu::value::Value;
use hardware::{Hardware, HardwareAddress, MemoryAccess};
use std::fmt::{self, Display, Formatter};
use std::mem;
use util::byte_access::ByteAccess;

const IO_CYCLES: u64 = 6;

const RESET_VECTOR: u16 = 0xFFFC;

pub struct Cpu {
    hardware: Hardware,
    regs: CpuRegisters,
    flags: CpuFlags
}

pub struct CpuRegisters {
    pub accumulator: u16,
    pub index_x: u16,
    pub index_y: u16,
    pub data_bank: u8,
    pub direct_page: u16,
    pub program_bank: u8,
    pub program_counter: u16,
    pub stack_pointer: u16,
}

pub struct CpuFlags {
    pub negative: bool,
    pub overflow: bool,
    pub memory_size: bool,
    pub index_size: bool,
    pub unused_flag: bool,
    pub break_flag: bool,
    pub decimal_mode: bool,
    pub interrupt_disable: bool,
    pub zero: bool,
    pub carry: bool,
    pub emulation_mode: bool
}

enum BranchCondition {
    CarrySet,
    CarryClear,
    Equal,
    NotEqual,
    Minus,
    Plus,
    OverflowSet,
    OverflowClear,
    Always
}

enum BlockMove {
    Negative,
    Positive
}

macro_rules! memory_size {
    ($cpu:ident, $method:ident, $($param:ident),*) => {{
        if $cpu.flags.memory_size {
            $cpu.$method::<u8, $($param<u8>),*>($($param::<u8>::default()),*);
        } else {
            $cpu.$method::<u16, $($param<u16>),*>($($param::<u16>::default()),*);
        }
    }}
}

macro_rules! index_size {
    ($cpu:ident, $method:ident, $($param:ident),*) => {{
        if $cpu.flags.index_size {
            $cpu.$method::<u8, $($param<u8>),*>($($param::<u8>::default()),*);
        } else {
            $cpu.$method::<u16, $($param<u16>),*>($($param::<u16>::default()),*);
        }
    }}
}

macro_rules! push_value {
    ($cpu:ident, $value:expr) => {{
        let value = $value;
        $cpu.regs.stack_pointer = $cpu.regs.stack_pointer.wrapping_sub(value.size());
        // TODO: Emulation mode stack location
        let address = HardwareAddress::new(0, $cpu.regs.stack_pointer.wrapping_add(1));
        $cpu.hardware.write(address, value);
    }}
}

impl Cpu {
    pub fn new(mut hardware: Hardware) -> Cpu {
        let program_counter = hardware.read::<u16>(HardwareAddress::new(0, RESET_VECTOR));

        Cpu {
            hardware: hardware,
            regs: CpuRegisters {
                accumulator: 0,
                index_x: 0,
                index_y: 0,
                data_bank: 0,
                direct_page: 0,
                program_bank: 0,
                program_counter: program_counter,
                stack_pointer: 0,
            },
            flags: CpuFlags {
                negative: false,
                overflow: false,
                memory_size: true,
                index_size: true,
                unused_flag: false,
                break_flag: false,
                decimal_mode: false,
                interrupt_disable: false,
                zero: false,
                carry: false,
                emulation_mode: true
            }
        }
    }

    pub fn tick(&mut self) {
        if self.hardware.regs().cpu_action_ready() {
            // Check for interrupts and things
            if self.hardware.regs_mut().check_and_reset_nmi() {
                self.interrupt::<Nmi>();
            } else if self.hardware.regs_mut().check_and_reset_irq() {
                if !self.flags.interrupt_disable {
                    self.interrupt::<Irq>();
                } else {
                    debug!("IRQ prevented by CPU 'I' flag");
                }
            } else if let Some(mask) = self.hardware.regs_mut().check_and_reset_dma() {
                self.hardware.dma_transfer(mask);
            } else {
                panic!("Unknown CPU action requested");
            }
        } else {
            // Otherwise, read an instruction from the PC location as normal
            match self.read_next::<u8>() {
                0x00 => self.interrupt::<Break>(),
                0x01 => memory_size!(self, or, MemoryDirectPageIndexedXIndirect),
                0x02 => self.interrupt::<Coprocessor>(),
                0x03 => memory_size!(self, or, MemoryStackRelative),
                0x04 => memory_size!(self, test_and_set_bits, MemoryDirectPage),
                0x05 => memory_size!(self, or, MemoryDirectPage),
                0x06 => memory_size!(self, arithmetic_shift_left, MemoryDirectPage),
                0x07 => memory_size!(self, or, MemoryDirectPageIndirectLong),
                0x08 => self.push::<u8, ProcessorState>(Default::default()),
                0x09 => memory_size!(self, or, Immediate),
                0x0A => memory_size!(self, arithmetic_shift_left, Accumulator),
                0x0B => self.push::<u16, DirectPage>(Default::default()),
                0x0C => memory_size!(self, test_and_set_bits, MemoryAbsolute),
                0x0D => memory_size!(self, or, MemoryAbsolute),
                0x0E => memory_size!(self, arithmetic_shift_left, MemoryAbsolute),
                0x0F => memory_size!(self, or, MemoryAbsoluteLong),
                0x10 => self.branch(BranchCondition::Plus),
                0x11 => memory_size!(self, or, MemoryDirectPageIndirectIndexedY),
                0x12 => memory_size!(self, or, MemoryDirectPageIndirect),
                0x13 => memory_size!(self, or, MemoryStackRelativeIndirectIndexedY),
                0x14 => memory_size!(self, test_and_reset_bits, MemoryDirectPage),
                0x15 => memory_size!(self, or, MemoryDirectPageIndexedX),
                0x16 => memory_size!(self, arithmetic_shift_left, MemoryDirectPageIndexedX),
                0x17 => memory_size!(self, or, MemoryDirectPageIndirectLongIndexedY),
                0x18 => self.clear_carry(),
                0x19 => memory_size!(self, or, MemoryAbsoluteIndexedY),
                0x1A => memory_size!(self, increment, Accumulator),
                0x1B => self.transfer::<u16, Accumulator, StackPointer>(Default::default(), Default::default()),
                0x1C => memory_size!(self, test_and_reset_bits, MemoryAbsolute),
                0x1D => memory_size!(self, or, MemoryAbsoluteIndexedX),
                0x1E => memory_size!(self, arithmetic_shift_left, MemoryAbsoluteIndexedX),
                0x1F => memory_size!(self, or, MemoryAbsoluteLongIndexedX),
                0x20 => self.jump_to_subroutine(MemoryAbsolute::<u16>::default()),
                0x21 => memory_size!(self, and, MemoryDirectPageIndexedXIndirect),
                0x22 => self.jump_to_subroutine_long(MemoryAbsoluteLong::<u16>::default()),
                0x23 => memory_size!(self, and, MemoryStackRelative),
                0x24 => memory_size!(self, bit_test, MemoryDirectPage),
                0x25 => memory_size!(self, and, MemoryDirectPage),
                0x26 => memory_size!(self, rotate_left, MemoryDirectPage),
                0x27 => memory_size!(self, and, MemoryDirectPageIndirectLong),
                0x28 => self.pull::<u8, ProcessorState>(Default::default()),
                0x29 => memory_size!(self, and, Immediate),
                0x2A => memory_size!(self, rotate_left, Accumulator),
                0x2B => self.pull::<u16, DirectPage>(Default::default()),
                0x2C => memory_size!(self, bit_test, MemoryAbsolute),
                0x2D => memory_size!(self, and, MemoryAbsolute),
                0x2E => memory_size!(self, rotate_left, MemoryAbsolute),
                0x2F => memory_size!(self, and, MemoryAbsoluteLong),
                0x30 => self.branch(BranchCondition::Minus),
                0x31 => memory_size!(self, and, MemoryDirectPageIndirectIndexedY),
                0x32 => memory_size!(self, and, MemoryDirectPageIndirect),
                0x33 => memory_size!(self, and, MemoryStackRelativeIndirectIndexedY),
                0x34 => memory_size!(self, bit_test, MemoryDirectPageIndexedX),
                0x35 => memory_size!(self, and, MemoryDirectPageIndexedX),
                0x36 => memory_size!(self, rotate_left, MemoryDirectPageIndexedX),
                0x37 => memory_size!(self, and, MemoryDirectPageIndirectLongIndexedY),
                0x38 => self.set_carry(),
                0x39 => memory_size!(self, and, MemoryAbsoluteIndexedY),
                0x3A => memory_size!(self, decrement, Accumulator),
                0x3B => self.transfer::<u16, StackPointer, Accumulator>(Default::default(), Default::default()),
                0x3C => memory_size!(self, bit_test, MemoryAbsoluteIndexedX),
                0x3D => memory_size!(self, and, MemoryAbsoluteIndexedX),
                0x3E => memory_size!(self, rotate_left, MemoryAbsoluteIndexedX),
                0x3F => memory_size!(self, and, MemoryAbsoluteLongIndexedX),
                0x40 => self.return_from_interrupt(),
                0x41 => memory_size!(self, exclusive_or, MemoryDirectPageIndexedXIndirect),
                0x42 => { debug!("WDM"); self.io_cycle(); },
                0x43 => memory_size!(self, exclusive_or, MemoryStackRelative),
                0x44 => self.move_block(BlockMove::Positive),
                0x45 => memory_size!(self, exclusive_or, MemoryDirectPage),
                0x46 => memory_size!(self, logical_shift_right, MemoryDirectPage),
                0x47 => memory_size!(self, exclusive_or, MemoryDirectPageIndirectLong),
                0x48 => memory_size!(self, push, Accumulator),
                0x49 => memory_size!(self, exclusive_or, Immediate),
                0x4A => memory_size!(self, logical_shift_right, Accumulator),
                0x4B => self.push::<u8, ProgramBank>(Default::default()),
                0x4C => self.jump(MemoryAbsolute::<u16>::default()),
                0x4D => memory_size!(self, exclusive_or, MemoryAbsolute),
                0x4E => memory_size!(self, logical_shift_right, MemoryAbsolute),
                0x4F => memory_size!(self, exclusive_or, MemoryAbsoluteLong),
                0x50 => self.branch(BranchCondition::OverflowClear),
                0x51 => memory_size!(self, exclusive_or, MemoryDirectPageIndirectIndexedY),
                0x52 => memory_size!(self, exclusive_or, MemoryDirectPageIndirect),
                0x53 => memory_size!(self, exclusive_or, MemoryStackRelativeIndirectIndexedY),
                0x54 => self.move_block(BlockMove::Negative),
                0x55 => memory_size!(self, exclusive_or, MemoryDirectPageIndexedX),
                0x56 => memory_size!(self, logical_shift_right, MemoryDirectPageIndexedX),
                0x57 => memory_size!(self, exclusive_or, MemoryDirectPageIndirectLongIndexedY),
                0x58 => self.clear_interrupt_disable(),
                0x59 => memory_size!(self, exclusive_or, MemoryAbsoluteIndexedY),
                0x5A => index_size!(self, push, IndexY),
                0x5B => self.transfer::<u16, Accumulator, DirectPage>(Default::default(), Default::default()),
                0x5C => self.jump_long(MemoryAbsoluteLong::<u16>::default()),
                0x5D => memory_size!(self, exclusive_or, MemoryAbsoluteIndexedX),
                0x5E => memory_size!(self, logical_shift_right, MemoryAbsoluteIndexedX),
                0x5F => memory_size!(self, exclusive_or, MemoryAbsoluteLongIndexedX),
                0x60 => self.return_from_subroutine(),
                0x61 => memory_size!(self, add_with_carry, MemoryDirectPageIndexedXIndirect),
                0x62 => self.push_effective_address(MemoryProgramCounterRelative::<u16>::default()),
                0x63 => memory_size!(self, add_with_carry, MemoryStackRelative),
                0x64 => memory_size!(self, store_zero, MemoryDirectPage),
                0x65 => memory_size!(self, add_with_carry, MemoryDirectPage),
                0x66 => memory_size!(self, rotate_right, MemoryDirectPage),
                0x67 => memory_size!(self, add_with_carry, MemoryDirectPageIndirectLong),
                0x68 => memory_size!(self, pull, Accumulator),
                0x69 => memory_size!(self, add_with_carry, Immediate),
                0x6A => memory_size!(self, rotate_right, Accumulator),
                0x6B => self.return_from_subroutine_long(),
                0x6C => self.jump(MemoryAbsoluteIndirect::<u16>::default()),
                0x6D => memory_size!(self, add_with_carry, MemoryAbsolute),
                0x6E => memory_size!(self, rotate_right, MemoryAbsolute),
                0x6F => memory_size!(self, add_with_carry, MemoryAbsoluteLong),
                0x70 => self.branch(BranchCondition::OverflowSet),
                0x71 => memory_size!(self, add_with_carry, MemoryDirectPageIndirectIndexedY),
                0x72 => memory_size!(self, add_with_carry, MemoryDirectPageIndirect),
                0x73 => memory_size!(self, add_with_carry, MemoryStackRelativeIndirectIndexedY),
                0x74 => memory_size!(self, store_zero, MemoryDirectPageIndexedX),
                0x75 => memory_size!(self, add_with_carry, MemoryDirectPageIndexedX),
                0x76 => memory_size!(self, rotate_right, MemoryDirectPageIndexedX),
                0x77 => memory_size!(self, add_with_carry, MemoryDirectPageIndirectLongIndexedY),
                0x78 => self.set_interrupt_disable(),
                0x79 => memory_size!(self, add_with_carry, MemoryAbsoluteIndexedY),
                0x7A => index_size!(self, pull, IndexY),
                0x7B => self.transfer::<u16, DirectPage, Accumulator>(Default::default(), Default::default()),
                0x7C => self.jump(MemoryAbsoluteIndexedXIndirect::<u16>::default()),
                0x7D => memory_size!(self, add_with_carry, MemoryAbsoluteIndexedX),
                0x7E => memory_size!(self, rotate_right, MemoryAbsoluteIndexedX),
                0x7F => memory_size!(self, add_with_carry, MemoryAbsoluteLongIndexedX),
                0x80 => self.branch(BranchCondition::Always),
                0x81 => memory_size!(self, store, Accumulator, MemoryDirectPageIndexedXIndirect),
                0x82 => self.branch_always_long(),
                0x83 => memory_size!(self, store, Accumulator, MemoryStackRelative),
                0x84 => index_size!(self, store, IndexY, MemoryDirectPage),
                0x85 => memory_size!(self, store, Accumulator, MemoryDirectPage),
                0x86 => index_size!(self, store, IndexX, MemoryDirectPage),
                0x87 => memory_size!(self, store, Accumulator, MemoryDirectPageIndirectLong),
                0x88 => index_size!(self, decrement, IndexY),
                0x89 => memory_size!(self, bit_test, Immediate),
                0x8A => memory_size!(self, transfer, IndexX, Accumulator),
                0x8B => self.push::<u8, DataBank>(Default::default()),
                0x8C => index_size!(self, store, IndexY, MemoryAbsolute),
                0x8D => memory_size!(self, store, Accumulator, MemoryAbsolute),
                0x8E => index_size!(self, store, IndexX, MemoryAbsolute),
                0x8F => memory_size!(self, store, Accumulator, MemoryAbsoluteLong),
                0x90 => self.branch(BranchCondition::CarryClear),
                0x91 => memory_size!(self, store, Accumulator, MemoryDirectPageIndirectIndexedY),
                0x92 => memory_size!(self, store, Accumulator, MemoryDirectPageIndirect),
                0x93 => memory_size!(self, store, Accumulator, MemoryStackRelativeIndirectIndexedY),
                0x94 => index_size!(self, store, IndexY, MemoryDirectPageIndexedX),
                0x95 => memory_size!(self, store, Accumulator, MemoryDirectPageIndexedX),
                0x96 => index_size!(self, store, IndexX, MemoryDirectPageIndexedY),
                0x97 => memory_size!(self, store, Accumulator, MemoryDirectPageIndirectLongIndexedY),
                0x98 => memory_size!(self, transfer, IndexY, Accumulator),
                0x99 => memory_size!(self, store, Accumulator, MemoryAbsoluteIndexedY),
                0x9A => self.transfer::<u16, IndexX, StackPointer>(Default::default(), Default::default()),
                0x9B => index_size!(self, transfer, IndexX, IndexY),
                0x9C => memory_size!(self, store_zero, MemoryAbsolute),
                0x9D => memory_size!(self, store, Accumulator, MemoryAbsoluteIndexedX),
                0x9E => memory_size!(self, store_zero, MemoryAbsoluteIndexedX),
                0x9F => memory_size!(self, store, Accumulator, MemoryAbsoluteLongIndexedX),
                0xA0 => index_size!(self, load, IndexY, Immediate),
                0xA1 => memory_size!(self, load, Accumulator, MemoryDirectPageIndexedXIndirect),
                0xA2 => index_size!(self, load, IndexX, Immediate),
                0xA3 => memory_size!(self, load, Accumulator, MemoryStackRelative),
                0xA4 => index_size!(self, load, IndexY, MemoryDirectPage),
                0xA5 => memory_size!(self, load, Accumulator, MemoryDirectPage),
                0xA6 => index_size!(self, load, IndexX, MemoryDirectPage),
                0xA7 => memory_size!(self, load, Accumulator, MemoryDirectPageIndirectLong),
                0xA8 => index_size!(self, transfer, Accumulator, IndexY),
                0xA9 => memory_size!(self, load, Accumulator, Immediate),
                0xAA => index_size!(self, transfer, Accumulator, IndexX),
                0xAB => self.pull::<u8, DataBank>(Default::default()),
                0xAC => index_size!(self, load, IndexY, MemoryAbsolute),
                0xAD => memory_size!(self, load, Accumulator, MemoryAbsolute),
                0xAE => index_size!(self, load, IndexX, MemoryAbsolute),
                0xAF => memory_size!(self, load, Accumulator, MemoryAbsoluteLong),
                0xB0 => self.branch(BranchCondition::CarrySet),
                0xB1 => memory_size!(self, load, Accumulator, MemoryDirectPageIndirectIndexedY),
                0xB2 => memory_size!(self, load, Accumulator, MemoryDirectPageIndirect),
                0xB3 => memory_size!(self, load, Accumulator, MemoryStackRelativeIndirectIndexedY),
                0xB4 => index_size!(self, load, IndexY, MemoryDirectPageIndexedX),
                0xB5 => memory_size!(self, load, Accumulator, MemoryDirectPageIndexedX),
                0xB6 => index_size!(self, load, IndexX, MemoryDirectPageIndexedY),
                0xB7 => memory_size!(self, load, Accumulator, MemoryDirectPageIndirectLongIndexedY),
                0xB8 => self.clear_overflow(),
                0xB9 => memory_size!(self, load, Accumulator, MemoryAbsoluteIndexedY),
                0xBA => index_size!(self, transfer, StackPointer, IndexX),
                0xBB => index_size!(self, transfer, IndexY, IndexX),
                0xBC => index_size!(self, load, IndexY, MemoryAbsoluteIndexedX),
                0xBD => memory_size!(self, load, Accumulator, MemoryAbsoluteIndexedX),
                0xBE => index_size!(self, load, IndexX, MemoryAbsoluteIndexedY),
                0xBF => memory_size!(self, load, Accumulator, MemoryAbsoluteLongIndexedX),
                0xC0 => index_size!(self, compare, IndexY, Immediate),
                0xC1 => memory_size!(self, compare, Accumulator, MemoryDirectPageIndexedXIndirect),
                0xC2 => self.reset_processor_state(),
                0xC3 => memory_size!(self, compare, Accumulator, MemoryStackRelative),
                0xC4 => index_size!(self, compare, IndexY, MemoryDirectPage),
                0xC5 => memory_size!(self, compare, Accumulator, MemoryDirectPage),
                0xC6 => memory_size!(self, decrement, MemoryDirectPage),
                0xC7 => memory_size!(self, compare, Accumulator, MemoryDirectPageIndirectLong),
                0xC8 => index_size!(self, increment, IndexY),
                0xC9 => memory_size!(self, compare, Accumulator, Immediate),
                0xCA => index_size!(self, decrement, IndexX),
                0xCB => self.wait_for_interrupt(),
                0xCC => index_size!(self, compare, IndexY, MemoryAbsolute),
                0xCD => memory_size!(self, compare, Accumulator, MemoryAbsolute),
                0xCE => memory_size!(self, decrement, MemoryAbsolute),
                0xCF => memory_size!(self, compare, Accumulator, MemoryAbsoluteLong),
                0xD0 => self.branch(BranchCondition::NotEqual),
                0xD1 => memory_size!(self, compare, Accumulator, MemoryDirectPageIndirectIndexedY),
                0xD2 => memory_size!(self, compare, Accumulator, MemoryDirectPageIndirect),
                0xD3 => memory_size!(self, compare, Accumulator, MemoryStackRelativeIndirectIndexedY),
                0xD4 => self.push_effective_address(MemoryDirectPageIndirect::<u16>::default()),
                0xD5 => memory_size!(self, compare, Accumulator, MemoryDirectPageIndexedX),
                0xD6 => memory_size!(self, decrement, MemoryDirectPageIndexedX),
                0xD7 => memory_size!(self, compare, Accumulator, MemoryDirectPageIndirectLongIndexedY),
                0xD8 => self.clear_decimal_mode(),
                0xD9 => memory_size!(self, compare, Accumulator, MemoryAbsoluteIndexedY),
                0xDA => index_size!(self, push, IndexX),
                0xDB => self.stop(),
                0xDC => self.jump_long(MemoryAbsoluteIndirectLong::<u16>::default()),
                0xDD => memory_size!(self, compare, Accumulator, MemoryAbsoluteIndexedX),
                0xDE => memory_size!(self, decrement, MemoryAbsoluteIndexedX),
                0xDF => memory_size!(self, compare, Accumulator, MemoryAbsoluteLongIndexedX),
                0xE0 => index_size!(self, compare, IndexX, Immediate),
                0xE1 => memory_size!(self, subtract_with_carry, MemoryDirectPageIndexedXIndirect),
                0xE2 => self.set_processor_state(),
                0xE3 => memory_size!(self, subtract_with_carry, MemoryStackRelative),
                0xE4 => index_size!(self, compare, IndexX, MemoryDirectPage),
                0xE5 => memory_size!(self, subtract_with_carry, MemoryDirectPage),
                0xE6 => memory_size!(self, increment, MemoryDirectPage),
                0xE7 => memory_size!(self, subtract_with_carry, MemoryDirectPageIndirectLong),
                0xE8 => index_size!(self, increment, IndexX),
                0xE9 => memory_size!(self, subtract_with_carry, Immediate),
                0xEA => { debug!("NOP"); self.io_cycle(); },
                0xEB => self.exchange_accumulators(),
                0xEC => index_size!(self, compare, IndexX, MemoryAbsolute),
                0xED => memory_size!(self, subtract_with_carry, MemoryAbsolute),
                0xEE => memory_size!(self, increment, MemoryAbsolute),
                0xEF => memory_size!(self, subtract_with_carry, MemoryAbsoluteLong),
                0xF0 => self.branch(BranchCondition::Equal),
                0xF1 => memory_size!(self, subtract_with_carry, MemoryDirectPageIndirectIndexedY),
                0xF2 => memory_size!(self, subtract_with_carry, MemoryDirectPageIndirect),
                0xF3 => memory_size!(self, subtract_with_carry, MemoryStackRelativeIndirectIndexedY),
                0xF4 => self.push_effective_address(MemoryAbsolute::<u16>::default()),
                0xF5 => memory_size!(self, subtract_with_carry, MemoryDirectPageIndexedX),
                0xF6 => memory_size!(self, increment, MemoryDirectPageIndexedX),
                0xF7 => memory_size!(self, subtract_with_carry, MemoryDirectPageIndirectLongIndexedY),
                0xF8 => self.set_decimal_mode(),
                0xF9 => memory_size!(self, subtract_with_carry, MemoryAbsoluteIndexedY),
                0xFA => index_size!(self, pull, IndexX),
                0xFB => self.exchange_carry_and_emulation_bits(),
                0xFC => self.jump_to_subroutine(MemoryAbsoluteIndexedXIndirect::<u16>::default()),
                0xFD => memory_size!(self, subtract_with_carry, MemoryAbsoluteIndexedX),
                0xFE => memory_size!(self, increment, MemoryAbsoluteIndexedX),
                0xFF => memory_size!(self, subtract_with_carry, MemoryAbsoluteLongIndexedX),
                op_code @ _ => panic!("Unrecognised op code: {:02X}", op_code)
            };
        }

        debug!("A={:04X} X={:04X} Y={:04X} PC={:02X}:{:04X} DP={:04X} DB={:02X} SP={:04X} P={} E={} T={}",
            self.regs.accumulator,
            self.regs.index_x,
            self.regs.index_y,
            self.regs.program_bank,
            self.regs.program_counter,
            self.regs.direct_page,
            self.regs.data_bank,
            self.regs.stack_pointer,
            self.flags,
            self.flags.emulation_mode as u8,
            self.hardware.clock());
    }

    /*
     * ACCESSORS
     */

    pub fn hardware(&self) -> &Hardware {
        &self.hardware
    }

    pub fn hardware_mut(&mut self) -> &mut Hardware {
        &mut self.hardware
    }

    pub fn regs(&self) -> &CpuRegisters {
        &self.regs
    }

    pub fn regs_mut(&mut self) -> &mut CpuRegisters {
        &mut self.regs
    }

    pub fn flags(&self) -> &CpuFlags {
        &self.flags
    }

    pub fn flags_mut(&mut self) -> &mut CpuFlags {
        &mut self.flags
    }

    /*
     * MEMORY READ/WRITE
     */

    pub fn read_next<T: MemoryAccess>(&mut self) -> T {
        let address = HardwareAddress::new(self.regs.program_bank, self.regs.program_counter);
        let value = self.hardware.read::<T>(address);
        self.regs.program_counter = self.regs.program_counter.wrapping_add(value.size());
        value
    }

    /*
     * UTILITY METHODS
     */

    pub fn io_cycle(&mut self) {
        self.hardware.tick(IO_CYCLES);
    }

    pub fn direct_page_cycle(&mut self) {
        if self.regs.direct_page.lower() != 0 {
            self.io_cycle();
        }
    }

    fn set_zero_and_negative<T>(&mut self, value: T) where T: Value {
        self.flags.zero = value.is_zero();
        self.flags.negative = value.is_negative();
    }

    fn pull_value<T: MemoryAccess>(&mut self) -> T {
        // TODO: Emulation mode stack location
        let address = HardwareAddress::new(0, self.regs.stack_pointer.wrapping_add(1));
        let value = self.hardware.read::<T>(address);
        self.regs.stack_pointer = self.regs.stack_pointer.wrapping_add(value.size());
        value
    }

    /*
     * INTERRUPTS
     */

    fn interrupt<I: Interrupt>(&mut self) {
        if I::has_signature() {
            let signature = self.read_next::<u8>();
            debug!("{} {:02X}", I::as_str(), signature);
        } else {
            debug!("{}", I::as_str());
        }

        let processor_state = ProcessorState::default();

        self.io_cycle();
        self.io_cycle();

        let vector_offset = if self.flags.emulation_mode {
            self.flags.break_flag = true;
            I::emulation_vector()
        } else {
            push_value!(self, self.regs.program_bank);
            self.regs.program_bank = 0x00;
            I::native_vector()
        };

        push_value!(self, self.regs.program_counter);

        if I::set_break() {
            self.flags.break_flag = true;
        }

        push_value!(self, processor_state.get(self));
        
        let vector_address = HardwareAddress::new(0x00, vector_offset);
        self.regs.program_counter = self.hardware.read::<u16>(vector_address);

        self.flags.decimal_mode = false;

        if I::set_interrupt_disable() {
            self.flags.interrupt_disable = true;
        }
    }

    /*
     * INSTRUCTIONS
     */

    fn add_with_carry<T: Value, A: AddressMode<T>>(&mut self, parameter: A) {
        let accessor = parameter.resolve(self);
        debug!("ADC {}", accessor);
        let accumulator = Accumulator::<T>::default();
        let lhs = accumulator.get(self);
        let rhs = accessor.get(self);

        if self.flags.decimal_mode {
            panic!("Decimal mode not supported yet!");
        } else {
            let result = lhs.add_value(rhs).add_value(T::from_bool(self.flags.carry));
            accumulator.set(self, result);
            self.flags.carry = result < lhs;
            self.flags.overflow = (!(lhs ^ rhs) & (rhs ^ result)).is_negative();
            self.set_zero_and_negative(result);
        }
    }

    fn and<T: Value, A: AddressMode<T>>(&mut self, parameter: A) {
        let accessor = parameter.resolve(self);
        debug!("AND {}", accessor);
        let accumulator = Accumulator::<T>::default();
        let lhs = accumulator.get(self);
        let rhs = accessor.get(self);
        let result = lhs & rhs;
        accumulator.set(self, result);
        self.set_zero_and_negative(result);
    }
    
    fn arithmetic_shift_left<T: Value, A: AddressMode<T>>(&mut self, parameter: A) 
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("ASL {}", accessor);
        let (result, carry) = accessor.get(self).left_shift_value();
        self.io_cycle();
        accessor.set(self, result);
        self.flags.carry = carry;
        self.set_zero_and_negative(result);
    }

    fn bit_test<T: Value, A: AddressMode<T>>(&mut self, parameter: A) {
        let accessor = parameter.resolve(self);
        debug!("BIT {}", accessor);
        let lhs = Accumulator::<T>::default().get(self);
        let rhs = accessor.get(self);
        self.flags.negative = rhs.is_negative();
        self.flags.overflow = rhs.is_overflow();
        self.flags.zero = (lhs & rhs).is_zero();
    }

    fn branch(&mut self, condition: BranchCondition) {
        let offset = self.read_next::<u8>() as i8;

        debug!("B{} {:+}", condition, offset);

        let should_branch = match condition {
            BranchCondition::CarrySet => self.flags.carry,
            BranchCondition::CarryClear => !self.flags.carry,
            BranchCondition::Equal => self.flags.zero,
            BranchCondition::NotEqual => !self.flags.zero,
            BranchCondition::Minus => self.flags.negative,
            BranchCondition::Plus => !self.flags.negative,
            BranchCondition::OverflowSet => self.flags.overflow,
            BranchCondition::OverflowClear => !self.flags.overflow,
            BranchCondition::Always => true
        };

        if should_branch {
            self.regs.program_counter = (self.regs.program_counter as i16).wrapping_add(offset as i16) as u16;
            debug!("Branched to {:04X}", self.regs.program_counter);
            self.io_cycle();
            // TODO: Emulation mode extra cycle?
        } else {
            debug!("Branch not taken");
        }
    }

    fn branch_always_long(&mut self) {
        let offset = self.read_next::<u16>() as i16;
        debug!("BRL {:+}", offset);
        self.regs.program_counter = (self.regs.program_counter as i16).wrapping_add(offset) as u16;
        debug!("Branched to {:04X}", self.regs.program_counter);
        self.io_cycle();
    }

    fn clear_carry(&mut self) {
        debug!("CLC");
        self.flags.carry = false;
        self.io_cycle();
    }

    fn clear_decimal_mode(&mut self) {
        debug!("CLD");
        self.flags.decimal_mode = false;
        self.io_cycle();
    }

    fn clear_interrupt_disable(&mut self) {
        debug!("CLI");
        self.flags.interrupt_disable = false;
        self.io_cycle();
    }

    fn clear_overflow(&mut self) {
        debug!("CLV");
        self.flags.overflow = false;
        self.io_cycle();
    }

    fn compare<T: Value, A: Read<T>, B: AddressMode<T>>(&mut self, register: A, parameter: B) {
        let accessor = parameter.resolve(self);
        debug!("CP{} {}", register, accessor);
        let lhs = register.get(self);
        let rhs = accessor.get(self);
        let result = lhs.subtract_value(rhs);
        self.flags.carry = result <= lhs;
        self.set_zero_and_negative(result);
    }

    fn decrement<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("DEC {}", accessor);
        let result = accessor.get(self).subtract_value(T::from(1));
        self.io_cycle();
        accessor.set(self, result);
        self.set_zero_and_negative(result);
    }

    fn exclusive_or<T: Value, A: AddressMode<T>>(&mut self, parameter: A) {
        let accessor = parameter.resolve(self);
        debug!("EOR {}", accessor);
        let accumulator = Accumulator::<T>::default();
        let lhs = accumulator.get(self);
        let rhs = accessor.get(self);
        let result = lhs ^ rhs;
        accumulator.set(self, result);
        self.set_zero_and_negative(result);
    }

    fn increment<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("INC {}", accessor);
        let result = accessor.get(self).add_value(T::from(1));
        self.io_cycle();
        accessor.set(self, result);
        self.set_zero_and_negative(result);
    }

    fn jump<A: AddressMode<u16>>(&mut self, parameter: A)
        where A::Output: Address
    {
        let address = parameter.resolve(self);
        debug!("JMP {}", address);
        self.regs.program_counter = address.offset();
    }

    fn jump_long<A: AddressMode<u16>>(&mut self, parameter: A)
        where A::Output: Address
    {
        let address = parameter.resolve(self);
        debug!("JML {}", address);
        self.regs.program_bank = address.bank();
        self.regs.program_counter = address.offset();
    }

    fn jump_to_subroutine<A: AddressMode<u16>>(&mut self, parameter: A)
        where A::Output: Address
    {
        let address = parameter.resolve(self);
        debug!("JSR {}", address);
        push_value!(self, self.regs.program_counter - 1);
        self.regs.program_counter = address.offset();
    }

    fn jump_to_subroutine_long<A: AddressMode<u16>>(&mut self, parameter: A)
        where A::Output: Address
    {
        let address = parameter.resolve(self);
        debug!("JSL {}", address);
        self.io_cycle();
        push_value!(self, self.regs.program_bank);
        push_value!(self, self.regs.program_counter - 1);
        self.regs.program_bank = address.bank();
        self.regs.program_counter = address.offset();
    }

    fn load<T: Value, A: Write<T>, B: AddressMode<T>>(&mut self, register: A, parameter: B)
    {
        let accessor = parameter.resolve(self);
        debug!("LD{} {}", register, accessor);
        let value = accessor.get(self);
        register.set(self, value);
        self.set_zero_and_negative(value);
    }

    fn logical_shift_right<T: Value, A: AddressMode<T>>(&mut self, parameter: A) 
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("LSR {}", accessor);
        let (result, carry) = accessor.get(self).right_shift_value();
        self.io_cycle();
        accessor.set(self, result);
        self.flags.carry = carry;
        self.set_zero_and_negative(result);
    }

    fn move_block(&mut self, block_move: BlockMove) {
        let dst_bank = self.read_next::<u8>();
        let src_bank = self.read_next::<u8>();

        debug!("MV{} ${:02X},${:02X}", block_move, src_bank, dst_bank);

        let value = self.hardware.read::<u8>(HardwareAddress::new(src_bank, self.regs.index_x));
        self.hardware.write(HardwareAddress::new(dst_bank, self.regs.index_y), value);

        match block_move {
            BlockMove::Negative => {
                self.regs.index_x = self.regs.index_x.wrapping_add(1);
                self.regs.index_y = self.regs.index_y.wrapping_add(1);
            },
            BlockMove::Positive => {
                self.regs.index_x = self.regs.index_x.wrapping_sub(1);
                self.regs.index_y = self.regs.index_x.wrapping_sub(1);
            }
        };

        self.regs.accumulator = self.regs.accumulator.wrapping_sub(1);

        self.io_cycle();
        self.io_cycle();

        if self.regs.accumulator != 0xFFFF {
            // Repeat this operation next tick instead of advancing the program counter
            self.regs.program_counter = self.regs.program_counter.wrapping_sub(3);
        }
    }

    fn or<T: Value, A: AddressMode<T>>(&mut self, parameter: A) {
        let accessor = parameter.resolve(self);
        debug!("ORA {}", accessor);
        let accumulator = Accumulator::<T>::default();
        let lhs = accumulator.get(self);
        let rhs = accessor.get(self);
        let result = lhs | rhs;
        accumulator.set(self, result);
        self.set_zero_and_negative(result);
    }

    fn pull<T: Value, A: Write<T>>(&mut self, register: A) {
        debug!("PL{}", register);
        self.io_cycle();
        self.io_cycle();
        let value = self.pull_value::<T>();
        self.set_zero_and_negative(value);
        register.set(self, value);
    }

    fn push<T: Value, A: Read<T>>(&mut self, register: A) {
        debug!("PH{}", register);
        self.io_cycle();
        push_value!(self, register.get(self));
    }

    fn push_effective_address<A: AddressMode<u16>>(&mut self, parameter: A)
        where A::Output: Address
    {
        let address = parameter.resolve(self);
        debug!("PEA {}", address);
        push_value!(self, address.offset());
    }
    
    fn reset_processor_state(&mut self) {
        let value = self.read_next::<u8>();
        debug!("REP #%{:08b}", value);
        let processor_state = ProcessorState::default();
        let result = processor_state.get(self) & !value;
        processor_state.set(self, result);
        self.io_cycle();
    }

    fn return_from_interrupt(&mut self) {
        debug!("RTI");
        self.io_cycle();
        self.io_cycle();
        let processor_state = self.pull_value::<u8>();
        ProcessorState::default().set(self, processor_state);
        self.regs.program_counter = self.pull_value::<u16>();
        if !self.flags.emulation_mode {
            self.regs.program_bank = self.pull_value::<u8>();
        }
    }

    fn return_from_subroutine(&mut self) {
        debug!("RTS");
        self.io_cycle();
        self.io_cycle();
        self.regs.program_counter = self.pull_value::<u16>() + 1;
        self.io_cycle();
    }

    fn return_from_subroutine_long(&mut self) {
        debug!("RTL");
        self.io_cycle();
        self.io_cycle();
        self.regs.program_counter = self.pull_value::<u16>() + 1;
        self.regs.program_bank = self.pull_value::<u8>();
    }

    fn rotate_left<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("ROL {}", accessor);
        let old_carry = self.flags.carry;
        let (result, new_carry) = accessor.get(self).left_rotate_value(old_carry);
        self.io_cycle();
        accessor.set(self, result);
        self.flags.carry = new_carry;
        self.set_zero_and_negative(result);
    }

    fn rotate_right<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("ROR {}", accessor);
        let old_carry = self.flags.carry;
        let (result, new_carry) = accessor.get(self).right_rotate_value(old_carry);
        self.io_cycle();
        accessor.set(self, result);
        self.flags.carry = new_carry;
        self.set_zero_and_negative(result);
    }

    fn set_carry(&mut self) {
        debug!("SEC");
        self.flags.carry = true;
        self.io_cycle();
    }

    fn set_decimal_mode(&mut self) {
        debug!("SED");
        self.flags.decimal_mode = true;
        self.io_cycle();
    }

    fn set_interrupt_disable(&mut self) {
        debug!("SEI");
        self.flags.interrupt_disable = true;
        self.io_cycle();
    }

    fn set_processor_state(&mut self) {
        let value = self.read_next::<u8>();
        debug!("SEP #%{:08b}", value);
        let processor_state = ProcessorState::default();
        let result = processor_state.get(self) | value;
        processor_state.set(self, result);
        self.io_cycle();
    }

    fn stop(&mut self) {
        debug!("STP");
        panic!("Processor stopped!");
    }

    fn store<T: Value, A: Read<T>, B: AddressMode<T>>(&mut self, register: A, parameter: B)
        where B::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("ST{} {}", register, accessor);
        let value = register.get(self);
        accessor.set(self, value);
    }

    fn store_zero<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("STZ {}", accessor);
        accessor.set(self, T::from(0));
    }

    fn subtract_with_carry<T: Value, A: AddressMode<T>>(&mut self, parameter: A) {
        let accessor = parameter.resolve(self);
        debug!("SBC {}", accessor);
        let accumulator = Accumulator::<T>::default();
        let lhs = accumulator.get(self);
        let rhs = accessor.get(self);

        if self.flags.decimal_mode {
            panic!("Decimal mode not supported yet!");
        } else {
            let result = lhs.subtract_value(rhs).subtract_value(T::from_bool(!self.flags.carry));
            accumulator.set(self, result);
            self.flags.carry = result <= lhs;
            self.flags.overflow = ((lhs ^ rhs) & (lhs ^ result)).is_negative();
            self.set_zero_and_negative(result);
        }
    }

    fn test_and_reset_bits<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("TRB {}", accessor);
        let lhs = Accumulator::<T>::default().get(self);
        let rhs = accessor.get(self);
        self.io_cycle();
        accessor.set(self, (!lhs) & rhs);
        self.flags.zero = (lhs & rhs).is_zero();
    }

    fn test_and_set_bits<T: Value, A: AddressMode<T>>(&mut self, parameter: A)
        where A::Output: Write<T>
    {
        let accessor = parameter.resolve(self);
        debug!("TSB {}", accessor);
        let lhs = Accumulator::<T>::default().get(self);
        let rhs = accessor.get(self);
        self.io_cycle();
        accessor.set(self, lhs | rhs);
        self.flags.zero = (lhs & rhs).is_zero();
    }

    fn transfer<T: Value, A: AddressMode<T>, B: AddressMode<T>>(&mut self, src: A, dst: B)
        where B::Output: Write<T>
    {
        let src_accessor = src.resolve(self);
        let dst_accessor = dst.resolve(self);
        debug!("T{}{}", src_accessor, dst_accessor);
        let value = src_accessor.get(self);
        self.io_cycle();
        dst_accessor.set(self, value);
        self.set_zero_and_negative(value);
    }
    
    fn wait_for_interrupt(&mut self) {
        debug!("WAI");
        panic!("Interrupts not yet supported!");
    }

    fn exchange_accumulators(&mut self) {
        debug!("XBA");
        let result = self.regs.accumulator.swap_bytes();
        self.io_cycle();
        self.regs.accumulator = result;
        self.set_zero_and_negative(result);
    }

    fn exchange_carry_and_emulation_bits(&mut self) {
        debug!("XCE");
        mem::swap(&mut self.flags.carry, &mut self.flags.emulation_mode);
        self.flags.memory_size = true;
        self.flags.index_size = true;
        self.io_cycle();
    }
}

impl Display for CpuFlags {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}{}{}{}{}{}{}",
            if self.negative { "N" } else { "-" },
            if self.overflow { "V" } else { "-" },
            if self.emulation_mode {
                if self.unused_flag { "?" } else { "-" }
            } else {
                if self.memory_size { "M" } else { "-" }
            },
            if self.emulation_mode {
                if self.break_flag { "B" } else { "-" }
            } else {
                if self.index_size { "X" } else { "-" }
            },
            if self.overflow { "D" } else { "-" },
            if self.interrupt_disable { "I" } else { "-" },
            if self.zero { "Z" } else { "-" },
            if self.carry { "C" } else { "-" })
    }
}

impl Display for BranchCondition {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            BranchCondition::CarrySet => "CS",
            BranchCondition::CarryClear => "CC",
            BranchCondition::Equal => "EQ",
            BranchCondition::NotEqual => "NE",
            BranchCondition::Minus => "MI",
            BranchCondition::Plus => "PL",
            BranchCondition::OverflowSet => "VS",
            BranchCondition::OverflowClear => "VC",
            BranchCondition::Always => "RA"
        })
    }
}

impl Display for BlockMove {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            BlockMove::Negative => "N",
            BlockMove::Positive => "P"
        })
    }
}
