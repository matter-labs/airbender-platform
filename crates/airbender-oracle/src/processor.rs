//! The oracle side of the protocol on the host (the `oracle_processor` feature, `std`), which
//! Airbender defines in `riscv_transpiler::oracle` and this module re-exports:
//! [`QueryDispatcher`] dispatches queries to [`OracleQueryProcessor`]s and serves the RISC-V
//! guest of the simulators as its non-determinism CSR source, [`GuestSource`] adds the plain
//! input words of the guest, and [`ReadWitnessSource`] records what the guest reads.
//!
//! What this crate adds is the querier side on top of them: [`MemoryOracle`] for
//! [`QueryDispatcher`] and [`ReadWitnessSource`], which serve a querier in this process (forward
//! mode, and the native run that records the prover input), with [`QueryError`] as its
//! [`OracleError`]. A querier with an error type of its own converts from it.

use crate::{MemoryOracle, OracleError};
pub use riscv_transpiler::oracle::{
    query_timing, respond_to_every_target, GuestMemory, GuestSource, OracleQueryProcessor,
    QueryDispatcher, QueryError, ReadWitnessSource, RunMode,
};
pub use riscv_transpiler::vm::{NonDeterminismCSRSource, RamPeek};

impl OracleError for QueryError {
    fn new(message: &'static str) -> Self {
        Self(message)
    }
}

/// Implements [`MemoryOracle`] by the transport primitives of the same names that the source
/// has for a querier in this process, whose contracts are those of the trait.
macro_rules! memory_oracle_of_this_process {
    ($source:ty) => {
        impl MemoryOracle for $source {
            type Error = QueryError;

            unsafe fn send_query(
                &mut self,
                query_id: u32,
                input_word: usize,
            ) -> Result<(), QueryError> {
                // SAFETY: guaranteed by the caller
                unsafe { <$source>::send_query(self, query_id, input_word) }
            }

            fn read_word(&mut self) -> Result<u32, QueryError> {
                <$source>::read_word(self)
            }

            unsafe fn write_words(
                &mut self,
                dst: *mut u32,
                num_words: usize,
            ) -> Result<(), QueryError> {
                // SAFETY: guaranteed by the caller
                unsafe { <$source>::write_words(self, dst, num_words) }
            }

            fn finish_query(&mut self) -> Result<(), QueryError> {
                <$source>::finish_query(self)
            }
        }
    };
}

memory_oracle_of_this_process!(QueryDispatcher);
memory_oracle_of_this_process!(ReadWitnessSource);

/// The dispatcher and the recording source through [`MemoryOracle`], as a querier that is
/// generic over its oracle uses them. The sources themselves are tested where they are defined.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_ids::DISCONNECT_ORACLE_QUERY_ID;
    use crate::{read_querier_u32_words_into, read_querier_word, QuerierMemory};

    const SWAP_QUERY_ID: u32 = 0x1234_0001;

    /// Reads two 8-word values at the addresses of the array at the input word, and answers
    /// them swapped
    struct SwapProcessor;

    impl OracleQueryProcessor for SwapProcessor {
        fn supported_memory_query_ids(&self) -> Vec<u32> {
            vec![SWAP_QUERY_ID]
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
            assert_eq!(query_id, SWAP_QUERY_ID);
            let word_size = memory.word_size();
            let a = read_querier_word(memory, input_word).unwrap();
            let b = read_querier_word(memory, input_word + word_size).unwrap();
            let mut response = vec![0u32; 16];
            read_querier_u32_words_into(memory, b, &mut response[..8]).unwrap();
            read_querier_u32_words_into(memory, a, &mut response[8..]).unwrap();
            respond_to_every_target(mode, response, native_run_responses, guest_run_responses);
        }
    }

    const REPRESENTATION_QUERY_ID: u32 = 0x1234_0003;

    /// Echoes the 8 words at the input word, in a representation that depends on the target: as
    /// is for a querier in this process, in reverse order for the guest.
    struct RepresentationProcessor {
        follows_mode: bool,
    }

    impl OracleQueryProcessor for RepresentationProcessor {
        fn supported_memory_query_ids(&self) -> Vec<u32> {
            vec![REPRESENTATION_QUERY_ID]
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
            assert_eq!(query_id, REPRESENTATION_QUERY_ID);
            let mut words = [0u32; 8];
            read_querier_u32_words_into(memory, input_word, &mut words).unwrap();
            if !self.follows_mode || mode.produces_native_run_responses() {
                native_run_responses.extend_from_slice(&words);
            }
            if self.follows_mode && mode.produces_guest_run_responses() {
                guest_run_responses.extend(words.iter().rev());
            }
        }
    }

    const REPRESENTATION: RepresentationProcessor = RepresentationProcessor { follows_mode: true };

    fn words(seed: u32) -> [u32; 8] {
        core::array::from_fn(|i| seed.wrapping_add(i as u32))
    }

    fn reversed(value: &[u32; 8]) -> [u32; 8] {
        let mut words = *value;
        words.reverse();
        words
    }

    /// The swap query of a querier in this process
    fn swap<O: MemoryOracle>(
        oracle: &mut O,
        a: &[u32; 8],
        b: &[u32; 8],
    ) -> Result<[u32; 16], O::Error> {
        let addresses = [
            a.as_ptr().expose_provenance(),
            b.as_ptr().expose_provenance(),
        ];
        // SAFETY: the array of addresses and the operands are alive during the query
        unsafe { oracle.send_query(SWAP_QUERY_ID, addresses.as_ptr().expose_provenance())? };
        let mut response = [0u32; 16];
        // SAFETY: the buffer holds 16 words
        unsafe { oracle.write_words(response.as_mut_ptr(), 16)? };
        oracle.finish_query()?;
        Ok(response)
    }

    /// The echo query of a querier in this process, read a word at a time
    fn echo<O: MemoryOracle>(oracle: &mut O, a: &[u32; 8]) -> Result<[u32; 8], O::Error> {
        // SAFETY: the operand is alive during the query
        unsafe { oracle.send_query(REPRESENTATION_QUERY_ID, a.as_ptr().expose_provenance())? };
        let mut response = [0u32; 8];
        for word in response.iter_mut() {
            *word = oracle.read_word()?;
        }
        oracle.finish_query()?;
        Ok(response)
    }

    #[test]
    fn the_protocol_makes_its_errors_as_query_errors() {
        assert_eq!(
            <QueryError as OracleError>::new("message"),
            QueryError("message")
        );
    }

    #[test]
    fn the_dispatcher_serves_a_querier_in_this_process() {
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(REPRESENTATION);
        oracle.add_external_processor(SwapProcessor);
        let (a, b) = (words(1), words(100));
        assert_eq!(echo(&mut oracle, &a), Ok(a));
        assert_eq!(
            swap(&mut oracle, &a, &b),
            Ok([b, a].concat().try_into().unwrap())
        );
        // through a reference too, as the hinted operations take it
        assert_eq!(echo(&mut &mut oracle, &b), Ok(b));
    }

    #[test]
    fn the_errors_of_the_dispatcher_reach_the_querier() {
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(SwapProcessor);
        let a = words(1);
        // SAFETY: nothing is read for an unknown query
        let unknown = unsafe { MemoryOracle::send_query(&mut oracle, 0xabcd, 0) };
        assert_eq!(unknown, Err(QueryError("invalid query ID")));

        // a response that is read past its end, and one that is not read in full
        let addresses = [a.as_ptr().expose_provenance(); 2];
        // SAFETY: the array of addresses and the operand are alive during the query
        unsafe {
            MemoryOracle::send_query(
                &mut oracle,
                SWAP_QUERY_ID,
                addresses.as_ptr().expose_provenance(),
            )
        }
        .unwrap();
        let mut response = [0u32; 17];
        // SAFETY: the buffer holds 17 words
        let too_long = unsafe { MemoryOracle::write_words(&mut oracle, response.as_mut_ptr(), 17) };
        assert_eq!(
            too_long,
            Err(QueryError("oracle response is shorter than expected"))
        );
        assert_eq!(MemoryOracle::read_word(&mut oracle), Ok(a[0]));
        assert_eq!(
            MemoryOracle::finish_query(&mut oracle),
            Err(QueryError("oracle response contains excess data"))
        );
    }

    #[test]
    fn a_querier_in_this_process_is_not_served_in_a_risc_v_run() {
        let mut oracle = QueryDispatcher::new(RunMode::RiscVRun);
        oracle.add_external_processor(REPRESENTATION);
        assert!(echo(&mut oracle, &words(1)).is_err());
    }

    #[test]
    fn a_native_run_is_served_its_responses_and_records_those_of_the_guest_run() {
        let (a, b) = (words(1), words(100));
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(REPRESENTATION);
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        assert_eq!(echo(&mut source, &a), Ok(a));
        // a response that is the same on every target
        assert_eq!(
            swap(&mut source, &a, &b),
            Ok([b, a].concat().try_into().unwrap())
        );
        let expected = [reversed(&a), b, a].concat();
        assert_eq!(*source.get_read_items().borrow(), expected);
    }

    #[test]
    fn recording_needs_the_mode_that_saves_the_responses_of_the_guest_run() {
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        let a = words(1);
        assert!(swap(&mut source, &a, &a).is_err());
        assert!(source.get_read_items().borrow().is_empty());
    }

    #[test]
    fn a_processor_that_does_not_respond_for_the_mode_is_rejected() {
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(RepresentationProcessor {
            follows_mode: false,
        });
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        let a = words(1);
        assert_eq!(
            echo(&mut source, &a),
            Err(QueryError(
                "the query processor does not respond for the run mode"
            ))
        );
        // the failed query is over
        assert_eq!(
            swap(&mut source, &a, &a),
            Ok([a, a].concat().try_into().unwrap())
        );
        assert_eq!(*source.get_read_items().borrow(), [a, a].concat());
    }

    #[test]
    fn a_disconnected_oracle_records_the_zeroes_read() {
        let mut oracle = QueryDispatcher::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(REPRESENTATION);
        let mut source = ReadWitnessSource::new(oracle);
        // SAFETY: the disconnect query has no input
        unsafe { MemoryOracle::send_query(&mut source, DISCONNECT_ORACLE_QUERY_ID, 0) }.unwrap();
        MemoryOracle::finish_query(&mut source).unwrap();
        assert!(!source.inner().is_connected());
        assert_eq!(echo(&mut source, &words(1)), Ok([0; 8]));
        assert_eq!(swap(&mut source, &words(1), &words(2)), Ok([0; 16]));
        assert_eq!(*source.get_read_items().borrow(), vec![0; 24]);
    }
}
