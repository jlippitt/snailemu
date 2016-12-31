use cpu::accessor::{Read, Write};
use cpu::address_mode::AddressMode;
use cpu::value::Value;
use cpu::Cpu;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

macro_rules! register_static {
    ($struct_name:ident, $field_name:ident, $field_type:ty, $display_name:expr) => {
        #[derive(Default)]
        pub struct $struct_name;

        impl AddressMode<$field_type> for $struct_name {
            type Output = Self;

            fn resolve(self, _cpu: &mut Cpu) -> Self {
                self
            }
        }

        impl Display for $struct_name {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, "{}", $display_name)
            }
        }

        impl Read<$field_type> for $struct_name {
            fn get(&self, cpu: &mut Cpu) -> $field_type {
                cpu.regs().$field_name
            }
        }

        impl Write<$field_type> for $struct_name {
            fn set(&self, cpu: &mut Cpu, value: $field_type) {
                cpu.regs_mut().$field_name = value;
            }
        }
    }
}

macro_rules! register_modal {
    ($struct_name:ident, $field_name:ident, $display_name:expr) => {
        #[derive(Default)]
        pub struct $struct_name<T: Value = u16> {
            _value_type: PhantomData<T>
        }

        impl<T: Value> AddressMode<T> for $struct_name<T> {
            type Output = Self;

            fn resolve(self, _cpu: &mut Cpu) -> Self {
                self
            }
        }

        impl<T: Value> Display for $struct_name<T> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, "{}", $display_name)
            }
        }

        impl<T: Value> Read<T> for $struct_name<T> {
            fn get(&self, cpu: &mut Cpu) -> T {
                T::from_modal(cpu.regs().$field_name)
            }
        }

        impl<T: Value> Write<T> for $struct_name<T> {
            fn set(&self, cpu: &mut Cpu, value: T) {
                value.to_modal(&mut cpu.regs_mut().$field_name);
            }
        }
    }
}

register_modal!(Accumulator, accumulator, "A");
register_modal!(IndexX, index_x, "X");
register_modal!(IndexY, index_y, "Y");
register_static!(DataBank, data_bank, u8, "B");
register_static!(DirectPage, direct_page, u16, "D");
register_static!(ProgramBank, program_bank, u8, "K");
register_modal!(StackPointer, stack_pointer, "S");

#[derive(Default)]
pub struct ProcessorState;

impl AddressMode<u8> for ProcessorState {
    type Output = Self;

    fn resolve(self, _cpu: &mut Cpu) -> Self {
        self
    }
}

impl Display for ProcessorState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "P")
    }
}

impl Read<u8> for ProcessorState {
    fn get(&self, cpu: &mut Cpu) -> u8 {
        let flags = cpu.flags();

        if flags.emulation_mode {
            ((flags.negative as u8) << 7) | ((flags.overflow as u8) << 6) |
                ((flags.unused_flag as u8) << 5) | ((flags.break_flag as u8) << 4) |
                ((flags.decimal_mode as u8) << 3) | ((flags.interrupt_disable as u8) << 2) |
                ((flags.zero as u8) << 1) | (flags.carry as u8)
        } else {
            ((flags.negative as u8) << 7) | ((flags.overflow as u8) << 6) |
                ((flags.memory_size as u8) << 5) | ((flags.index_size as u8) << 4) |
                ((flags.decimal_mode as u8) << 3) | ((flags.interrupt_disable as u8) << 2) |
                ((flags.zero as u8) << 1) | (flags.carry as u8)
        }
    }
}

impl Write<u8> for ProcessorState {
    fn set(&self, cpu: &mut Cpu, value: u8) {
        let truncate_index_regs = {
            let flags = cpu.flags_mut();

            flags.negative = (value & 0x80) != 0;
            flags.overflow = (value & 0x40) != 0;
            flags.decimal_mode = (value & 0x08) != 0;
            flags.interrupt_disable = (value & 0x04) != 0;
            flags.zero = (value & 0x02) != 0;
            flags.carry = (value & 0x01) != 0;

            if flags.emulation_mode {
                flags.unused_flag = (value & 0x20) != 0;
                flags.break_flag = (value & 0x10) != 0;
                false
            } else {
                flags.memory_size = (value & 0x20) != 0;
                flags.index_size = (value & 0x10) != 0;
                flags.index_size
            }
        };

        if truncate_index_regs {
            let regs = cpu.regs_mut();
            regs.index_x &= 0x00FF;
            regs.index_y &= 0x00FF;
        }
    }
}
