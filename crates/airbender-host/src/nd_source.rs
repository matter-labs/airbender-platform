use riscv_transpiler::vm::{NonDeterminismCSRSource, RamPeek};
use std::collections::VecDeque;

/// Non-determinism source backed by a pre-loaded list of u32 words.
pub(crate) struct VecSource(VecDeque<u32>);

impl VecSource {
    pub(crate) fn new(words: Vec<u32>) -> Self {
        Self(words.into())
    }
}

impl NonDeterminismCSRSource for VecSource {
    fn read(&mut self) -> u32 {
        self.0.pop_front().expect("must have an answer")
    }

    fn write_with_memory_access<R: RamPeek>(&mut self, _ram: &R, _value: u32)
    where
        Self: Sized,
    {
    }

    fn write_with_memory_access_dyn(&mut self, _ram: &dyn RamPeek, _value: u32) {}
}
