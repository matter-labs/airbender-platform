// Ported from zksync-os e855bca2:zk_ee/src/oracle/memory_io/host.rs
// Ported from zksync-os e855bca2:oracle_provider/src/lib.rs
//! The oracle side of the hints, in a native host: reading the requests and the operands of the
//! queries from the memory of the querier, and answering them, for a querier in this process, for
//! the RISC-V guest, or for both (see [`answer_query`]). [`RecordingHintOracle`] answers a querier
//! in this process, and records the answers of the guest.
//!
//! The requests of a querier in this process are read as [`FieldOpsHint64`](super::FieldOpsHint64)
//! (and `ModExpAdviceParams64` for ModExp), so this module is for 64-bit hosts.

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

/// The memory of a querier that runs in the same process (forward mode), whose addresses are exposed
/// pointers.
#[derive(Debug)]
pub struct NativeQuerierMemory(());

impl NativeQuerierMemory {
    /// # Safety
    ///
    /// Only for reading the inputs of the queries sent through [`HintOracle`](super::HintOracle) by
    /// code of this process, while they are being sent: their addresses refer to live values then.
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

/// Answers the hint query `query_id` with the input word `input_word` of the querier of `memory`:
/// appends the answer words, exactly as the querier reads them, to the responses asked for. To
/// `native`, as a querier in this process reads them; to `guest`, as the RISC-V guest reads them.
///
/// The responses asked for are the run modes of zksync-os (`oracle_provider::RunMode`):
/// - `native` only: a native run, of a querier in this process;
/// - both: a native run that records the prover input, the answers the guest reads in place of the
///   querier of this process (see [`RecordingHintOracle`]);
/// - `guest` only: a run of the RISC-V guest, which the memory and the pointers in the query belong
///   to.
///
/// For the field hints ([`FIELD_OPS_ADVISE_QUERY_ID`]), the querier of `memory` is in this process
/// exactly when `native` is asked for, and its word size must say so (8 bytes, or 4 for the
/// guest), else this panics. The ModExp advice ([`MODEXP_ADVICE_QUERY_ID`] and
/// [`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`]) does not look at the run mode, as in zksync-os: it reads
/// the operands through `memory`, of either word size, and gives every run asked for the same
/// answer. Both panic on a malformed request, as zksync-os's processors do.
///
/// Returns `Ok(false)` for a query ID that it does not answer, without a response.
///
/// # Safety
///
/// If `native` is `Some`, `memory` must be the memory of this process (see
/// [`NativeQuerierMemory`]), and `input_word` the address of a live request of `query_id` whose
/// operand pointers reference live values of the operand types of its op, for the duration of the
/// call: the contract of [`HintOracle::send_query`](super::HintOracle::send_query). The operands
/// are then read as values of those types, in place. A querier on the guest (`native` is `None`)
/// is read through `memory` only.
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
        assert_eq!(memory.word_size(), 8);
        let address = words.as_ptr().expose_provenance();
        assert_eq!(memory.read_u32(address + 4), Ok(0xa0b0_c0d0));
        assert_eq!(
            read_querier_u32_words(&memory, address, 3),
            Ok(words.to_vec())
        );
        assert_eq!(read_querier_u32_words(&memory, address, 0), Ok(Vec::new()));
        // a querier word is two `u32`s, low first
        assert_eq!(
            read_querier_word(&memory, address),
            Ok(0xa0b0_c0d0_0102_0304)
        );
        assert_eq!(
            memory.read_u32(0),
            Err(HintError("invalid querier address"))
        );
        assert_eq!(
            memory.read_u32(address + 2),
            Err(HintError("invalid querier address"))
        );
    }

    /// A querier memory of zeroes at every address, which it does not dereference
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
            read_querier_u32_words(&Zeroes, usize::MAX - 3, 1),
            Ok(vec![0])
        );
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
        for query_id in [0, FIELD_OPS_ADVISE_QUERY_ID + 1, u32::MAX] {
            // SAFETY: nothing is read for a query ID that is not answered
            let answered =
                unsafe { answer_query(query_id, 0, &memory, Some(&mut native), Some(&mut guest)) };
            assert_eq!(answered, Ok(false));
        }
        assert!(native.is_empty() && guest.is_empty());
    }
}
