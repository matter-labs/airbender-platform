// Ported from zksync-os e855bca2:zk_ee/src/oracle/memory_io/host.rs
// Ported from zksync-os e855bca2:callable_oracles/src/lib.rs
// Ported from zksync-os e855bca2:oracle_provider/src/lib.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/curve_hints.rs
//! Test doubles of the oracle, and of the memory of the querier. `TapeOracle` is new; the others
//! are ported from zksync-os.

use super::oracle::{HintError, HintOracle};
use std::collections::VecDeque;
use std::vec::Vec;

/// An oracle that answers from a fixed tape of words: it records the queries without reading
/// their inputs, and every response word, read with `read_word` or `write_words`, is the next word
/// of the tape. Running out of tape is an error.
#[derive(Debug, Default)]
pub(crate) struct TapeOracle {
    pub(crate) words: VecDeque<u32>,
    pub(crate) queries: Vec<(u32, usize)>,
}

impl TapeOracle {
    pub(crate) fn new(words: impl IntoIterator<Item = u32>) -> Self {
        Self {
            words: words.into_iter().collect(),
            queries: Vec::new(),
        }
    }
}

impl HintOracle for TapeOracle {
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        self.queries.push((query_id, input_word));
        Ok(())
    }

    fn read_word(&mut self) -> Result<u32, HintError> {
        self.words
            .pop_front()
            .ok_or(HintError("the hint tape is exhausted"))
    }

    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
        for i in 0..num_words {
            let word = self.read_word()?;
            // SAFETY: `i < num_words`, guaranteed by the caller
            unsafe { dst.add(i).write(word) };
        }
        Ok(())
    }
}

#[cfg(target_pointer_width = "64")]
pub(crate) use self::in_process::{
    lying_oracle, lying_oracle_last_word, non_identity_claiming_oracle, GuestMemory,
    InProcessMemoryOracle, TestMemorySource,
};

/// The doubles that answer in process, with the host side (`hints::host`, 64-bit hosts only)
#[cfg(target_pointer_width = "64")]
mod in_process {
    use crate::hints::host::{answer_query, not_identity_claim, NativeQuerierMemory};
    use crate::hints::{
        read_field_hint_request, FieldHintOp, HintError, HintOracle, QuerierMemory,
    };
    use std::collections::{BTreeMap, VecDeque};
    use std::vec::Vec;

    /// The response to the current memory-based query, for oracles that serve the protocol in process.
    #[derive(Debug, Default)]
    pub(crate) struct ResponseBuffer {
        words: VecDeque<u32>,
    }

    impl ResponseBuffer {
        /// Starts serving the response to a new query.
        pub(crate) fn set(&mut self, words: Vec<u32>) -> Result<(), HintError> {
            if !self.words.is_empty() {
                self.words.clear();
                return Err(HintError(
                    "previous oracle response was not consumed in full",
                ));
            }
            self.words = words.into();
            Ok(())
        }

        /// [`HintOracle::read_word`]
        pub(crate) fn read_word(&mut self) -> Result<u32, HintError> {
            self.words
                .pop_front()
                .ok_or(HintError("oracle response is shorter than expected"))
        }

        /// [`HintOracle::write_words`]
        ///
        /// # Safety
        ///
        /// `dst` must be aligned for `u32` and valid for writes of `num_words` `u32`s.
        pub(crate) unsafe fn write_words(
            &mut self,
            dst: *mut u32,
            num_words: usize,
        ) -> Result<(), HintError> {
            if self.words.len() < num_words {
                return Err(HintError("oracle response is shorter than expected"));
            }
            for (i, word) in self.words.drain(..num_words).enumerate() {
                // SAFETY: `i < num_words`, and the caller guarantees `dst` is valid for `num_words` words
                unsafe { dst.add(i).write(word) };
            }
            Ok(())
        }

        /// [`HintOracle::finish_query`]
        pub(crate) fn finish(&mut self) -> Result<(), HintError> {
            if self.words.is_empty() {
                Ok(())
            } else {
                self.words.clear();
                Err(HintError("oracle response contains excess data"))
            }
        }
    }

    /// An oracle that answers memory-based queries in process, with a handler that gets the query ID, the
    /// input word and the memory of the querier, and returns the response words. For test doubles and simple
    /// oracles of this process.
    pub(crate) struct InProcessMemoryOracle<H> {
        handler: H,
        response: ResponseBuffer,
    }

    impl<H> InProcessMemoryOracle<H> {
        pub(crate) fn new(handler: H) -> Self {
            Self {
                handler,
                response: ResponseBuffer::default(),
            }
        }
    }

    impl<H: FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> HintOracle for InProcessMemoryOracle<H> {
        /// The handler runs during the query, with the memory of this process, where the input word
        /// is the address of a live request of the querier whose operands are live values of the
        /// types of its op (the contract of `send_query`): a handler may answer it with
        /// `answer_query` and the native responses.
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            // SAFETY: the querier is code of this process, which exposes the memory the input word refers
            // to while it sends the query, i.e. during this call
            let memory = unsafe { NativeQuerierMemory::new() };
            let response = (self.handler)(query_id, input_word, &memory);
            self.response.set(response)
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.response.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            // SAFETY: guaranteed by the caller
            unsafe { self.response.write_words(dst, num_words) }
        }

        fn finish_query(&mut self) -> Result<(), HintError> {
            self.response.finish()
        }
    }

    /// BTreeMap-backed memory source for testing oracle query processors.
    #[derive(Default)]
    pub(crate) struct TestMemorySource {
        words: BTreeMap<u32, u32>,
    }

    impl TestMemorySource {
        pub(crate) fn insert_u32(&mut self, address: u32, value: u32) {
            assert!(address.is_multiple_of(4));
            self.words.insert(address, value);
        }

        fn peek_word(&self, address: u32) -> u32 {
            self.words.get(&address).copied().unwrap_or(0)
        }
    }

    /// The memory of the RISC-V guest, for the processors of memory-based queries.
    pub(crate) struct GuestMemory<'a>(pub(crate) &'a TestMemorySource);

    impl QuerierMemory for GuestMemory<'_> {
        fn word_size(&self) -> usize {
            size_of::<u32>()
        }

        fn read_u32(&self, address: usize) -> Result<u32, HintError> {
            let address = u32::try_from(address)
                .map_err(|_| HintError("guest address does not fit into u32"))?;
            if !address.is_multiple_of(4) {
                return Err(HintError("unaligned guest address"));
            }
            Ok(self.0.peek_word(address))
        }
    }

    /// The op of the hint request at `input_word`
    fn read_op(memory: &dyn QuerierMemory, input_word: usize) -> FieldHintOp {
        let (op, _, _) = read_field_hint_request(memory, input_word).unwrap();
        FieldHintOp::parse_u32(op).unwrap()
    }

    /// A processor that answers the field hint queries of the given ops with a corrupted value
    /// (the low bit of the first word flipped), and the others honestly
    struct FlipLowBit {
        ops: Vec<FieldHintOp>,
        /// Corrupt the last word of the answer instead of the first payload word
        last_word: bool,
    }

    impl FlipLowBit {
        fn process_memory_query(
            &mut self,
            query_id: u32,
            input_word: usize,
            memory: &dyn QuerierMemory,
        ) -> Vec<u32> {
            let op = read_op(memory, input_word);
            // the response of the native run (the querier of the tests)
            let mut words = Vec::new();
            // SAFETY: the handler of an `InProcessMemoryOracle` runs during the query, with the
            // memory of this process, where the request and its operand are live, of the types of
            // the op
            assert!(
                unsafe { answer_query(query_id, input_word, memory, Some(&mut words), None) }
                    .expect("must answer the field hint query")
            );
            if self.ops.contains(&op) {
                // the low word of the first element (past the claim flag of the pairing
                // ops): still a canonical element, so the value check is what fails
                let index = match op {
                    _ if self.last_word => words.len() - 1,
                    FieldHintOp::Bn254PairingResidueWitness
                    | FieldHintOp::Bls12381KzgResidueWitness
                    | FieldHintOp::Bls12381PairingResidueWitness => 1,
                    _ => 0,
                };
                words[index] ^= 1;
            }
            words
        }
    }

    /// A processor that claims every pairing product not to be the identity, with the
    /// (correct) inverse the slow path needs
    struct ClaimNotIdentity;

    impl ClaimNotIdentity {
        fn process_memory_query(
            &mut self,
            query_id: u32,
            input_word: usize,
            memory: &dyn QuerierMemory,
        ) -> Vec<u32> {
            match read_op(memory, input_word) {
                FieldHintOp::Bn254PairingResidueWitness
                | FieldHintOp::Bls12381KzgResidueWitness
                | FieldHintOp::Bls12381PairingResidueWitness => {
                    // SAFETY: the handler of an `InProcessMemoryOracle` runs during the query,
                    // with the memory of this process, where the request and its operand are
                    // live, of the types of the op
                    unsafe { not_identity_claim(memory, input_word) }
                }
                _ => {
                    let mut native_run_responses = Vec::new();
                    // SAFETY: as above
                    assert!(unsafe {
                        answer_query(
                            query_id,
                            input_word,
                            memory,
                            Some(&mut native_run_responses),
                            None,
                        )
                    }
                    .expect("must answer the field hint query"));
                    native_run_responses
                }
            }
        }
    }

    /// An oracle that claims every pairing product not to be the identity
    pub(crate) fn non_identity_claiming_oracle(
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        let mut processor = ClaimNotIdentity;
        InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                processor.process_memory_query(query_id, input_word, memory)
            },
        )
    }

    /// An oracle that corrupts the answers of `ops` (their first payload word)
    pub(crate) fn lying_oracle(
        ops: &[FieldHintOp],
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        let mut processor = FlipLowBit {
            ops: ops.to_vec(),
            last_word: false,
        };
        InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                processor.process_memory_query(query_id, input_word, memory)
            },
        )
    }

    /// An oracle that corrupts the last word of the answers of `ops`
    pub(crate) fn lying_oracle_last_word(
        ops: &[FieldHintOp],
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        let mut processor = FlipLowBit {
            ops: ops.to_vec(),
            last_word: true,
        };
        InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                processor.process_memory_query(query_id, input_word, memory)
            },
        )
    }
}
