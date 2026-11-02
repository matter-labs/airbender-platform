//! The oracle side of the hints on a native host (`oracle_implementations`, 64-bit hosts).

mod field_hints;
mod operand;
mod recorder;

pub use self::field_hints::{not_identity_claim, read_querier_u32_words};
pub use self::operand::{Operand, Responses};
pub use self::recorder::{record_hints, RecordingHintOracle};
pub use airbender_oracle::NativeQuerierMemory;

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
/// (the contract of [`MemoryOracle::send_query`](airbender_oracle::MemoryOracle::send_query)):
/// they are read in place as those types. A guest querier is read through `memory` only.
pub unsafe fn answer_query(
    query_id: u32,
    input_word: usize,
    memory: &dyn airbender_oracle::QuerierMemory,
    native: Option<&mut alloc::vec::Vec<u32>>,
    guest: Option<&mut alloc::vec::Vec<u32>>,
) -> Result<bool, crate::hints::HintError> {
    use crate::hints::{
        FIELD_OPS_ADVISE_QUERY_ID, MODEXP_ADVICE_QUERY_ID, U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
    };
    match query_id {
        FIELD_OPS_ADVISE_QUERY_ID => {
            // SAFETY: guaranteed by the caller
            unsafe {
                field_hints::process_field_hint_query(query_id, input_word, memory, native, guest)
            };
            Ok(true)
        }
        MODEXP_ADVICE_QUERY_ID | U256_WIDE_DIV_REM_ADVICE_QUERY_ID => {
            crate::modexp::oracle_implementations::process_memory_query(
                query_id, input_word, memory, native, guest,
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}
