//! The responder for this crate's hints as an [`OracleQueryProcessor`] of Airbender's
//! `QueryDispatcher` (`riscv_transpiler::oracle`, re-exported by
//! `airbender_oracle::processor`), with the `oracle_processor` feature.

use super::answer_query;
use crate::hints::{
    FIELD_OPS_ADVISE_QUERY_ID, MODEXP_ADVICE_QUERY_ID, U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
};
use airbender_oracle::query_processor::{OracleQueryProcessor, RunMode};
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
/// let mut oracle = QueryDispatcher::new(RunMode::RiscVRun);
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

/// As [`CryptoHints`], for a RISC-V guest that reads the hints in the native representation
/// (`HintTarget::Native`): a `riscv64` guest, which has no bigint delegation, or a `riscv32` one
/// without `bigint_ops`. Such a guest is the twin of the native run, so a recording run
/// ([`RunMode::NativeRunSavingForRiscV`]) saves for it the words its own querier reads.
///
/// It does not serve the guest itself ([`RunMode::RiscVRun`]): the requests of a guest are read
/// in the layout of the `riscv32` guest with `bigint_ops`. Such a guest replays the recording.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeGuestCryptoHints;

impl OracleQueryProcessor for NativeGuestCryptoHints {
    fn supported_memory_query_ids(&self) -> Vec<u32> {
        CryptoHints.supported_memory_query_ids()
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
        assert!(
            mode.produces_native_run_responses(),
            "a guest with the native representation of the hints is not served in the simulator: \
             it replays the hints that a native run records"
        );
        let answered_from = native_run_responses.len();
        CryptoHints.process_memory_query(
            query_id,
            input_word,
            memory,
            RunMode::NativeRunOnly,
            native_run_responses,
            guest_run_responses,
        );
        if mode.produces_guest_run_responses() {
            guest_run_responses.extend_from_slice(&native_run_responses[answered_from..]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::{record_hints, HintError, MemoryOracle, NativeQuerierMemory, NoHints};
    use crate::modexp::modexp;
    use crate::precompiles::secp256k1_ecrecover;
    use alloc::collections::VecDeque;
    use hex_literal::hex;

    /// A native run's oracle over the responder alone, as a dispatcher serves it: each query
    /// goes to the processor, and the querier reads the native response
    #[derive(Default)]
    struct NativeRun {
        response: VecDeque<u32>,
        queries: usize,
    }

    impl MemoryOracle for NativeRun {
        type Error = HintError;

        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            assert!(CryptoHints.supported_memory_query_ids().contains(&query_id));
            assert!(self.response.is_empty(), "the previous answer was read");
            let (mut native, mut guest) = (Vec::new(), Vec::new());
            // SAFETY: the querier exposes the request during this call
            let memory = unsafe { NativeQuerierMemory::new() };
            CryptoHints.process_memory_query(
                query_id,
                input_word,
                &memory,
                RunMode::NativeRunOnly,
                &mut native,
                &mut guest,
            );
            assert!(guest.is_empty(), "a native run only has no guest response");
            self.response = native.into();
            self.queries += 1;
            Ok(())
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.response
                .pop_front()
                .ok_or(HintError("oracle response is shorter than expected"))
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            for i in 0..num_words {
                let word = self.read_word()?;
                // SAFETY: `i < num_words`, guaranteed by the caller
                unsafe { dst.add(i).write(word) };
            }
            Ok(())
        }

        fn finish_query(&mut self) -> Result<(), HintError> {
            assert!(self.response.is_empty(), "the answer was read in full");
            Ok(())
        }
    }

    #[test]
    fn serves_a_native_run_like_the_recorder() {
        let base = hex!("0000000000000000000000000000000000000000000000000000000000c0ffee");
        let exp = hex!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let modulus = hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
        let digest = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e");
        let s = hex!("789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");

        let mut oracle = NativeRun::default();
        let power = modexp(&base, &exp, &modulus, Some(&mut oracle));
        let key = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut oracle)).unwrap();
        assert!(oracle.queries > 0);

        assert_eq!(*power, *modexp(&base, &exp, &modulus, None::<&mut NoHints>));
        let plain = secp256k1_ecrecover(&digest, &digest, &s, false, None::<&mut NoHints>);
        assert_eq!(Ok(key.0), plain.map(|key| key.0));
        // the recorder answers the same queries
        let ((), tape) = record_hints(|recorder| {
            assert_eq!(*modexp(&base, &exp, &modulus, Some(recorder)), *power);
        });
        assert!(!tape.is_empty());
    }

    /// With Airbender's dispatcher itself: it serves a native run as its `MemoryOracle`, and a
    /// recording one saves the words the guest reads, the tape of the recorder
    #[test]
    fn registers_with_the_dispatcher() {
        use airbender_oracle::processor::{QueryDispatcher, ReadWitnessSource};

        let base = hex!("0000000000000000000000000000000000000000000000000000000000c0ffee");
        let exp = hex!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let modulus = hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
        let digest = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e");
        let s = hex!("789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");
        let plain_power = modexp(&base, &exp, &modulus, None::<&mut NoHints>);
        let plain_key = secp256k1_ecrecover(&digest, &digest, &s, false, None::<&mut NoHints>)
            .unwrap()
            .0;

        let mut oracle = QueryDispatcher::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(CryptoHints);
        assert_eq!(
            *modexp(&base, &exp, &modulus, Some(&mut oracle)),
            *plain_power
        );
        let key = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut oracle)).unwrap();
        assert_eq!(key.0, plain_key);

        let mut oracle = QueryDispatcher::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(CryptoHints);
        let mut source = ReadWitnessSource::new(oracle);
        assert_eq!(
            *modexp(&base, &exp, &modulus, Some(&mut source)),
            *plain_power
        );
        let key = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut source)).unwrap();
        assert_eq!(key.0, plain_key);
        let ((), tape) = record_hints(|recorder| {
            modexp(&base, &exp, &modulus, Some(&mut *recorder));
            secp256k1_ecrecover(&digest, &digest, &s, false, Some(recorder)).unwrap();
        });
        assert!(!tape.is_empty());
        assert_eq!(*source.get_read_items().borrow(), tape);
    }

    /// For a guest with the native representation, a recording run saves the words of the
    /// native answers: the tape of a recorder for that target
    #[test]
    fn a_native_guest_is_recorded_its_native_answers() {
        use crate::hints::{HintTarget, RecordingHintOracle};
        use airbender_oracle::processor::{QueryDispatcher, ReadWitnessSource};

        let digest = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e");
        let s = hex!("789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");
        let base = hex!("0000000000000000000000000000000000000000000000000000000000c0ffee");
        let modulus = hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");

        let mut oracle = QueryDispatcher::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(NativeGuestCryptoHints);
        let mut source = ReadWitnessSource::new(oracle);
        let key = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut source)).unwrap();
        let power = modexp(&base, &digest, &modulus, Some(&mut source));

        let mut recorder = RecordingHintOracle::recording_for(HintTarget::Native);
        let recorded = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut recorder));
        assert_eq!(recorded.map(|key| key.0), Ok(key.0));
        assert_eq!(
            *modexp(&base, &digest, &modulus, Some(&mut recorder)),
            *power
        );
        assert_eq!(*source.get_read_items().borrow(), recorder.tape());
        // which is not the tape of the guest with the bigint delegation
        let ((), delegated) = record_hints(|recorder| {
            secp256k1_ecrecover(&digest, &digest, &s, false, Some(recorder)).unwrap();
        });
        assert_ne!(recorder.tape()[..delegated.len()], delegated[..]);

        // a native run alone is served as by `CryptoHints`
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(NativeGuestCryptoHints);
        let alone = secp256k1_ecrecover(&digest, &digest, &s, false, Some(&mut oracle));
        assert_eq!(alone.map(|key| key.0), Ok(key.0));
    }

    #[test]
    #[should_panic(expected = "is not served in the simulator")]
    fn a_native_guest_is_not_served_in_the_simulator() {
        let (mut native, mut guest) = (Vec::new(), Vec::new());
        // SAFETY: nothing is read before the mode is rejected
        let memory = unsafe { NativeQuerierMemory::new() };
        NativeGuestCryptoHints.process_memory_query(
            crate::hints::FIELD_OPS_ADVISE_QUERY_ID,
            0,
            &memory,
            RunMode::RiscVRun,
            &mut native,
            &mut guest,
        );
    }

    #[test]
    fn declares_the_advice_queries() {
        assert_eq!(
            CryptoHints.supported_memory_query_ids(),
            [0x4005_0011, 0x4005_0010, 0x4005_0031]
        );
    }
}
