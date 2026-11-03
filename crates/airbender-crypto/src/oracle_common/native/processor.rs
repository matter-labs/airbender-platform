//! The responder for this crate's hints as an [`OracleQueryProcessor`] of
//! `airbender_oracle::processor::QueryDispatcher` (the `oracle_processor` feature).

use super::answer_query;
use crate::hints::{
    FIELD_OPS_ADVISE_QUERY_ID, MODEXP_ADVICE_QUERY_ID, U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
};
use airbender_oracle::processor::{OracleQueryProcessor, RunMode};
use airbender_oracle::QuerierMemory;
use alloc::vec::Vec;

/// The responder for the hints of this crate, for a `QueryDispatcher`: the field operations
/// (inverses, square roots, quotients and pairing witnesses, [`FIELD_OPS_ADVISE_QUERY_ID`]) and
/// the ModExp advice (multi-digit quotients and remainders, [`MODEXP_ADVICE_QUERY_ID`] and
/// [`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`]). It computes each answer from the request the querier
/// points to, with [`answer_query`], in the representation of the querier's target: the delegated
/// fields of the RISC-V guest, or the native fields of a querier in this process.
///
/// ```ignore
/// let mut oracle = QueryDispatcher::<HintError>::new(RunMode::RiscVRun);
/// oracle.add_external_processor(CryptoHints);
/// // `oracle` is the non-determinism CSR source of the guest, or with `RunMode::NativeRunOnly`
/// // the `MemoryOracle` of the hinted operations run in this process
/// ```
///
/// A native run dereferences each request, per the `unsafe` contract of
/// [`MemoryOracle::send_query`](airbender_oracle::MemoryOracle::send_query), which the dispatcher
/// as a `MemoryOracle` passes on to the querier; the RISC-V guest is read through the memory of
/// the simulator only. A malformed request panics, as everywhere on the oracle side.
#[derive(Clone, Copy, Debug, Default)]
pub struct CryptoHints;

impl OracleQueryProcessor for CryptoHints {
    fn supported_memory_query_ids(&self) -> Vec<u32> {
        alloc::vec![
            FIELD_OPS_ADVISE_QUERY_ID,
            MODEXP_ADVICE_QUERY_ID,
            U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
        ]
    }

    fn process_memory_query(
        &mut self,
        query_id: u32,
        input_word: usize,
        memory: &dyn QuerierMemory,
        mode: RunMode,
        native_run_responses: &mut Vec<u32>,
        guest_run_responses: &mut Vec<u32>,
    ) {
        // SAFETY: in a native run the dispatcher is the `MemoryOracle` of a querier in this
        // process, which exposes its request and operands while it sends the query (the contract
        // of `send_query`); the RISC-V guest is read through `memory` only.
        let answered = unsafe {
            answer_query(
                query_id,
                input_word,
                memory,
                mode.produces_native_run_responses()
                    .then_some(native_run_responses),
                mode.produces_guest_run_responses()
                    .then_some(guest_run_responses),
            )
        };
        assert_eq!(
            answered,
            Ok(true),
            "the dispatcher sends this processor the query IDs it declared"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::{record_hints, HintError, NoHints};
    use crate::modexp::modexp;
    use crate::precompiles::secp256k1_ecrecover;
    use airbender_oracle::processor::QueryDispatcher;
    use hex_literal::hex;

    /// The dispatcher with the responder, for a run in `mode`
    fn oracle(mode: RunMode) -> QueryDispatcher<HintError> {
        let mut oracle = QueryDispatcher::new(mode);
        oracle.add_external_processor(CryptoHints);
        oracle
    }

    #[test]
    fn serves_a_native_run_like_the_recorder() {
        let base = hex!("0000000000000000000000000000000000000000000000000000000000c0ffee");
        let exp = hex!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let modulus = hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
        let digest = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e");
        let s = hex!("789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");

        let mut oracle = oracle(RunMode::NativeRunOnly);
        let power = modexp(&base, &exp, &modulus, Some(&mut oracle));
        let key = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut oracle)).unwrap();

        assert_eq!(*power, *modexp(&base, &exp, &modulus, None::<&mut NoHints>));
        let plain = secp256k1_ecrecover(&digest, &digest, &s, false, None::<&mut NoHints>);
        assert_eq!(Ok(key.0), plain.map(|key| key.0));
        // the recorder answers the same queries
        let ((), tape) = record_hints(|recorder| {
            assert_eq!(*modexp(&base, &exp, &modulus, Some(recorder)), *power);
        });
        assert!(!tape.is_empty());
    }

    #[test]
    fn declares_the_advice_queries() {
        assert_eq!(
            CryptoHints.supported_memory_query_ids(),
            [0x4005_0011, 0x4005_0010, 0x4005_0031]
        );
    }
}
