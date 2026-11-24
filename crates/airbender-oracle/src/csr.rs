//! The oracle of the RISC-V guest: the non-determinism CSR as a word channel.

use super::word_channel::{WordChannel, WordChannelOracle};
use riscv_common::{csr_read_word, csr_write_word};

/// The non-determinism CSR as a [`WordChannel`]
#[derive(Clone, Copy, Debug, Default)]
pub struct CsrChannel;

impl WordChannel for CsrChannel {
    #[inline(always)]
    fn write_word(&mut self, word: usize) {
        csr_write_word(word)
    }

    #[inline(always)]
    fn read_word(&mut self) -> usize {
        csr_read_word() as usize
    }
}

/// The oracle of the guest: [`WordChannelOracle`] over the non-determinism CSR, with the
/// querier's error type `E`
pub type CsrOracle<E> = WordChannelOracle<CsrChannel, E>;
