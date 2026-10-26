// Ported from zksync-os e855bca2:zk_ee/src/oracle/memory_io/word_channel.rs
// Ported from zksync-os e855bca2:proof_running_system/src/io_oracle/mod.rs
// Ported from zksync-os e855bca2:zksync_os/src/main.rs
//! The oracle of the RISC-V guest: the non-determinism CSR as a word channel.

use super::oracle::{HintError, HintOracle};
use core::mem::MaybeUninit;
use riscv_common::{csr_read_word, csr_write_word};

/// [`HintOracle`] over the non-determinism CSR: a query is two CSR writes, the query ID and then
/// the input word, and every response word is one CSR read, stored by the querier itself into the
/// destination.
#[derive(Clone, Copy, Debug, Default)]
pub struct CsrHintOracle;

impl HintOracle for CsrHintOracle {
    /// The prover reads the request and the operand through the input word, in the memory of the
    /// guest, when it receives the query.
    #[inline(always)]
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        csr_write_word(query_id as usize);
        // The input word may be the address of the input, which the oracle reads as soon as it receives
        // the word, while for the compiler the channel write (a `nomem` CSR asm) reads no memory: without
        // a barrier, the stores that fill the input, or the address array of a composite input, are dead
        // and get removed. `black_box(())` is an empty asm that may read any exposed memory, so these
        // stores happen before it, and the channel write, an asm with side effects too, stays after it.
        // Being zero-sized, the argument is not spilled: no instruction is emitted.
        core::hint::black_box(());
        csr_write_word(input_word);
        // zksync-os wraps every CSR write in `black_box` (e855bca2:zksync_os/src/main.rs:28,
        // `core::hint::black_box(write_word(value as u32))`), so the address write is followed by a
        // barrier too: later stores, e.g. into the stack slot of the request that the oracle is still
        // to read, stay after it.
        core::hint::black_box(());
        Ok(())
    }

    #[inline(always)]
    fn read_word(&mut self) -> Result<u32, HintError> {
        Ok(csr_read_word())
    }

    #[inline(always)]
    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
        // SAFETY: guaranteed by the caller
        let dst =
            unsafe { core::slice::from_raw_parts_mut(dst.cast::<MaybeUninit<u32>>(), num_words) };
        // unrolled: for long responses the loop overhead is comparable to the channel read itself
        let (chunks, remainder) = dst.as_chunks_mut::<4>();
        for chunk in chunks.iter_mut() {
            chunk[0].write(csr_read_word());
            chunk[1].write(csr_read_word());
            chunk[2].write(csr_read_word());
            chunk[3].write(csr_read_word());
        }
        for word in remainder.iter_mut() {
            word.write(csr_read_word());
        }
        Ok(())
    }
}
