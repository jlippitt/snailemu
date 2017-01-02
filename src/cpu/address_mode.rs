use cpu::accessor::{ImmediateAccessor, MemoryAccessor, Read};
use cpu::memory_mode::*;
use cpu::value::Value;
use cpu::Cpu;
use std::marker::PhantomData;

pub trait AddressMode<T: Value> {
    type Output : Read<T>;

    fn resolve(self, cpu: &mut Cpu) -> Self::Output;
}

#[derive(Default)]
pub struct Immediate<T: Value> {
    _value_type: PhantomData<T>
}

#[derive(Default)]
pub struct Memory<T: Value, M: MemoryMode> {
    _value_type: PhantomData<T>,
    _memory_mode: PhantomData<M>
}

pub type MemoryAbsolute<T> = Memory<T, Absolute>;
pub type MemoryAbsoluteIndexedX<T> = Memory<T, AbsoluteIndexedX>;
pub type MemoryAbsoluteIndexedXIndirect<T> = Memory<T, AbsoluteIndexedXIndirect>;
pub type MemoryAbsoluteIndexedY<T> = Memory<T, AbsoluteIndexedY>;
pub type MemoryAbsoluteIndirect<T> = Memory<T, AbsoluteIndirect>;
pub type MemoryAbsoluteIndirectLong<T> = Memory<T, AbsoluteIndirectLong>;
pub type MemoryAbsoluteLong<T> = Memory<T, AbsoluteLong>;
pub type MemoryAbsoluteLongIndexedX<T> = Memory<T, AbsoluteLongIndexedX>;
pub type MemoryDirectPage<T> = Memory<T, DirectPage>;
pub type MemoryDirectPageIndexedX<T> = Memory<T, DirectPageIndexedX>;
pub type MemoryDirectPageIndexedXIndirect<T> = Memory<T, DirectPageIndexedXIndirect>;
pub type MemoryDirectPageIndexedY<T> = Memory<T, DirectPageIndexedY>;
pub type MemoryDirectPageIndirect<T> = Memory<T, DirectPageIndirect>;
pub type MemoryDirectPageIndirectIndexedY<T> = Memory<T, DirectPageIndirectIndexedY>;
pub type MemoryDirectPageIndirectLong<T> = Memory<T, DirectPageIndirectLong>;
pub type MemoryDirectPageIndirectLongIndexedY<T> = Memory<T, DirectPageIndirectLongIndexedY>;
pub type MemoryProgramCounterRelative<T> = Memory<T, ProgramCounterRelative>;
pub type MemoryStackRelative<T> = Memory<T, StackRelative>;
pub type MemoryStackRelativeIndirectIndexedY<T> = Memory<T, StackRelativeIndirectIndexedY>;

impl<T: Value> AddressMode<T> for Immediate<T> {
    type Output = ImmediateAccessor<T>;

    fn resolve(self, cpu: &mut Cpu) -> ImmediateAccessor<T> {
        ImmediateAccessor::new(cpu.read_next::<T>())
    }
}

impl<T: Value, M: MemoryMode> AddressMode<T> for Memory<T, M> {
    type Output = MemoryAccessor<T, M>;

    fn resolve(self, cpu: &mut Cpu) -> MemoryAccessor<T, M> {
        let (resolved_address, immediate_address) = M::resolve(cpu);
        MemoryAccessor::new(resolved_address, immediate_address)
    }
}
