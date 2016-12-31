use cpu::memory_mode::MemoryMode;
use cpu::value::Value;
use cpu::Cpu;
use hardware::HardwareAddress;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;
use std::mem;

pub trait Read<T: Value> : Display {
    fn get(&self, cpu: &mut Cpu) -> T;
}

pub trait Write<T: Value> : Read<T> {
    fn set(&self, cpu: &mut Cpu, value: T);
}

pub trait Address {
    fn bank(&self) -> u8;
    fn offset(&self) -> u16;
}

pub struct MemoryAccessor<T: Value, M: MemoryMode> {
    resolved_address: HardwareAddress,
    immediate_address: HardwareAddress,
    _value_type: PhantomData<T>,
    _memory_mode: PhantomData<M>
}

pub struct ImmediateAccessor<T: Value> {
    value: T
}

impl<T: Value, M: MemoryMode> MemoryAccessor<T, M> {
    pub fn new(resolved: HardwareAddress, immediate: HardwareAddress) -> MemoryAccessor<T, M> {
        MemoryAccessor {
            resolved_address: resolved,
            immediate_address: immediate,
            _value_type: PhantomData,
            _memory_mode: PhantomData
        }
    }
}

impl<T: Value, M: MemoryMode> Display for MemoryAccessor<T, M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        M::format(f, self.immediate_address)
    }
}

impl<T: Value, M: MemoryMode> Read<T> for MemoryAccessor<T, M> {
    fn get(&self, cpu: &mut Cpu) -> T {
        cpu.hardware_mut().read::<T>(self.resolved_address)
    }
}

impl<T: Value, M: MemoryMode> Write<T> for MemoryAccessor<T, M> {
    fn set(&self, cpu: &mut Cpu, value: T) {
        cpu.hardware_mut().write::<T>(self.resolved_address, value)
    }
}

impl<T: Value, M: MemoryMode> Address for MemoryAccessor<T, M> {
    fn bank(&self) -> u8 {
        self.resolved_address.bank()
    }

    fn offset(&self) -> u16 {
        self.resolved_address.offset()
    }
}

impl<T: Value> ImmediateAccessor<T> {
    pub fn new(value: T) -> ImmediateAccessor<T> {
        ImmediateAccessor {
            value: value
        }
    }
}

impl<T: Value> Display for ImmediateAccessor<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match mem::size_of::<T>() {
            1 => write!(f, "#${:02X}", self.value),
            2 => write!(f, "#${:04X}", self.value),
            _ => panic!("Unexpected large immediate value: {:?}", self.value)
        }
    }
}

impl<T: Value> Read<T> for ImmediateAccessor<T> {
    fn get(&self, _cpu: &mut Cpu) -> T {
        self.value
    }
}
