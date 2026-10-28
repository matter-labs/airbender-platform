//! The oracle side of the hints on a native host: reads the requests and operands from the
//! querier's memory and answers them, for a querier in this process, for the guest, or both (see
//! [`answer_query`]). [`RecordingHintOracle`] answers a native querier and records the guest's
//! answers. Native requests are read as [`FieldOpsHint64`](super::FieldOpsHint64) (and
//! `ModExpAdviceParams64`), so this module is for 64-bit hosts.

mod arith;
mod field;
mod recorder;

pub use self::field::not_identity_claim;
pub use self::recorder::{record_hints, RecordingHintOracle};

use super::request::offset_address;
use super::{
    HintError, QuerierMemory, FIELD_OPS_ADVISE_QUERY_ID, MODEXP_ADVICE_QUERY_ID,
    U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
};
use alloc::vec::Vec;

/// The memory of a querier in this process, whose addresses are exposed pointers.
#[derive(Debug)]
pub struct NativeQuerierMemory(());

impl NativeQuerierMemory {
    /// # Safety
    ///
    /// Only for reading the inputs of queries sent through [`HintOracle`](super::HintOracle) by
    /// this process, while they are being sent.
    pub const unsafe fn new() -> Self {
        Self(())
    }
}

impl QuerierMemory for NativeQuerierMemory {
    fn word_size(&self) -> usize {
        size_of::<usize>()
    }

    fn read_u32(&self, address: usize) -> Result<u32, HintError> {
        if address == 0 || !address.is_multiple_of(size_of::<u32>()) {
            return Err(HintError("invalid querier address"));
        }
        // SAFETY: guaranteed by the creator of the view
        Ok(unsafe { core::ptr::with_exposed_provenance::<u32>(address).read() })
    }
}

/// Reads `num_words` consecutive `u32` words at `address` of the querier.
pub fn read_querier_u32_words<M: QuerierMemory + ?Sized>(
    memory: &M,
    address: usize,
    num_words: usize,
) -> Result<Vec<u32>, HintError> {
    (0..num_words)
        .map(|i| {
            let offset = i
                .checked_mul(size_of::<u32>())
                .ok_or(HintError("querier address overflows"))?;
            memory.read_u32(offset_address(address, offset)?)
        })
        .collect()
}

/// Answers the query `query_id` at `input_word` of the querier of `memory`, appending the answer
/// words to the responses asked for: `native` as a querier in this process reads them, `guest` as
/// the RISC-V guest does. `native` only is a native run, both a recording native run, `guest` only
/// a guest run.
///
/// For field hints the querier is in this process exactly when `native` is asked for, and
/// `memory`'s word size must match (8, or 4 for the guest), else this panics. The ModExp advice
/// reads its operands through `memory` with either word size and gives every run the same answer.
/// Malformed requests panic. Returns `Ok(false)`, with no response, for an unknown query ID.
///
/// # Safety
///
/// If `native` is `Some`, `memory` must be this process's memory and `input_word` the address of
/// a live request whose operands are live values of its op's types, for the duration of the call
/// (the contract of [`HintOracle::send_query`](super::HintOracle::send_query)): they are read in
/// place as those types. A guest querier is read through `memory` only.
pub unsafe fn answer_query(
    query_id: u32,
    input_word: usize,
    memory: &dyn QuerierMemory,
    native: Option<&mut Vec<u32>>,
    guest: Option<&mut Vec<u32>>,
) -> Result<bool, HintError> {
    match query_id {
        FIELD_OPS_ADVISE_QUERY_ID => {
            // SAFETY: guaranteed by the caller
            unsafe { field::process_field_hint_query(query_id, input_word, memory, native, guest) };
            Ok(true)
        }
        MODEXP_ADVICE_QUERY_ID | U256_WIDE_DIV_REM_ADVICE_QUERY_ID => {
            arith::process_memory_query(query_id, input_word, memory, native, guest);
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::read_querier_word;

    #[test]
    fn native_querier_memory_reads_this_process() {
        let words = [0x0102_0304u32, 0xa0b0_c0d0, 7];
        // SAFETY: reads `words`, which is alive
        let memory = unsafe { NativeQuerierMemory::new() };
        let address = words.as_ptr().expose_provenance();
        assert_eq!(
            read_querier_u32_words(&memory, address, 3),
            Ok(words.to_vec())
        );
        // a querier word is two `u32`s, low first
        assert_eq!(
            read_querier_word(&memory, address),
            Ok(0xa0b0_c0d0_0102_0304)
        );
        for address in [0, address + 2] {
            assert_eq!(
                memory.read_u32(address),
                Err(HintError("invalid querier address"))
            );
        }
    }

    /// A querier memory of zeroes, which it does not dereference
    struct Zeroes;

    impl QuerierMemory for Zeroes {
        fn word_size(&self) -> usize {
            4
        }

        fn read_u32(&self, _address: usize) -> Result<u32, HintError> {
            Ok(0)
        }
    }

    #[test]
    fn querier_words_do_not_wrap_around() {
        assert_eq!(
            read_querier_u32_words(&Zeroes, usize::MAX - 3, 2),
            Err(HintError("querier address overflows"))
        );
    }

    #[test]
    fn unknown_query_ids_are_not_answered() {
        // SAFETY: nothing is read for an unknown query
        let memory = unsafe { NativeQuerierMemory::new() };
        let (mut native, mut guest) = (Vec::new(), Vec::new());
        // SAFETY: as above
        let answered = unsafe { answer_query(0, 0, &memory, Some(&mut native), Some(&mut guest)) };
        assert_eq!(answered, Ok(false));
        assert!(native.is_empty() && guest.is_empty());
    }
}
