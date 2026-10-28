//! The oracle of the RISC-V guest: the non-determinism CSR as a word channel.

use super::oracle::{HintError, HintOracle};
use core::mem::MaybeUninit;
use riscv_common::{csr_read_word, csr_write_word};

/// [`HintOracle`] over the non-determinism CSR: a query is two CSR writes (the query ID, then the
/// input word), and every response word is one CSR read.
#[derive(Clone, Copy, Debug, Default)]
pub struct CsrHintOracle;

impl HintOracle for CsrHintOracle {
    /// The prover reads the request and the operand in guest memory when it receives the query.
    #[inline(always)]
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        csr_write_word(query_id as usize);
        // The CSR write is a `nomem` asm, so without a barrier the stores that fill the request
        // look dead and are removed. `black_box(())` may read any exposed memory and emits nothing.
        core::hint::black_box(());
        csr_write_word(input_word);
        // Later stores (e.g. reusing the request's stack slot) must stay after the address write.
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
        // unrolled: the loop overhead is comparable to the CSR read itself
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
