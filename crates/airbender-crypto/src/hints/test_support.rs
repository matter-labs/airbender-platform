//! Test doubles of the oracle and of the querier's memory.

use super::oracle::HintError;
use airbender_oracle::MemoryOracle;
use std::collections::VecDeque;
use std::vec::Vec;

/// An oracle that answers from a fixed tape of words, as the guest's oracle does: it records the
/// queries without reading their inputs, and running out of tape is an error.
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

impl MemoryOracle for TapeOracle {
    type Error = HintError;

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

/// One read of a querier
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Read {
    Word,
    U64,
    /// `write_words` of this many words
    Words(usize),
}

/// An oracle that logs how the querier reads the answers of the oracle it wraps
pub(crate) struct ReadLog<O> {
    pub(crate) inner: O,
    pub(crate) reads: Vec<Read>,
}

impl<O> ReadLog<O> {
    pub(crate) fn new(inner: O) -> Self {
        Self {
            inner,
            reads: Vec::new(),
        }
    }
}

impl<O: MemoryOracle> MemoryOracle for ReadLog<O> {
    type Error = O::Error;

    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), O::Error> {
        // SAFETY: guaranteed by the caller
        unsafe { self.inner.send_query(query_id, input_word) }
    }

    fn read_word(&mut self) -> Result<u32, O::Error> {
        self.reads.push(Read::Word);
        self.inner.read_word()
    }

    fn read_u64(&mut self) -> Result<u64, O::Error> {
        self.reads.push(Read::U64);
        self.inner.read_u64()
    }

    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), O::Error> {
        self.reads.push(Read::Words(num_words));
        // SAFETY: guaranteed by the caller
        unsafe { self.inner.write_words(dst, num_words) }
    }

    fn finish_query(&mut self) -> Result<(), O::Error> {
        self.inner.finish_query()
    }
}

/// The advice words of the packed transport for the response words `words` read by `reads`:
/// the packing rule, written out (see `airbender_oracle::PackedWordChannelOracle`)
pub(crate) fn pack(words: &[u32], reads: &[Read]) -> Vec<u64> {
    let mut words = words.iter().copied().map(u64::from);
    let mut next = || words.next().expect("the reads are within the words");
    let mut tape = Vec::new();
    for read in reads {
        match *read {
            Read::Word => tape.push(next()),
            Read::U64 => tape.push(next() | next() << 32),
            Read::Words(len) => {
                for _ in 0..len / 2 {
                    tape.push(next() | next() << 32);
                }
                if len % 2 == 1 {
                    tape.push(next());
                }
            }
        }
    }
    assert!(words.next().is_none(), "the reads take all the words");
    tape
}

/// The advice tape of a packed guest as a word channel: queries are recorded, not answered
#[cfg(target_pointer_width = "64")]
#[derive(Debug, Default)]
pub(crate) struct PackedTape {
    pub(crate) words: VecDeque<u64>,
    pub(crate) written: Vec<usize>,
}

#[cfg(target_pointer_width = "64")]
impl airbender_oracle::WordChannel for PackedTape {
    fn write_word(&mut self, word: usize) {
        self.written.push(word);
    }

    fn read_word(&mut self) -> usize {
        self.words
            .pop_front()
            .expect("the advice tape is exhausted") as usize
    }
}

/// The oracle of a packed guest that reads the advice words `words`
#[cfg(target_pointer_width = "64")]
pub(crate) fn packed_tape_oracle(
    words: impl IntoIterator<Item = u64>,
) -> airbender_oracle::PackedWordChannelOracle<PackedTape, HintError> {
    airbender_oracle::PackedWordChannelOracle::new(PackedTape {
        words: words.into_iter().collect(),
        written: Vec::new(),
    })
}

#[cfg(target_pointer_width = "64")]
pub(crate) use self::in_process::{
    lying_oracle, lying_oracle_last_word, non_identity_claiming_oracle, GuestMemory,
    InProcessMemoryOracle, TestMemorySource,
};

/// The doubles that answer in process, with the host side (64-bit hosts only)
#[cfg(target_pointer_width = "64")]
mod in_process {
    use crate::hints::{
        read_field_hint_request, FieldHintOp, HintError, MemoryOracle, QuerierMemory,
        QuerierMemoryError,
    };
    use crate::oracle_common::{answer_query, not_identity_claim, NativeQuerierMemory};
    use std::collections::{BTreeMap, VecDeque};
    use std::vec::Vec;

    /// An oracle that answers queries in process: `handler` gets the query ID, the input word and
    /// the querier's memory, and returns the response words. Unread or missing words are errors.
    pub(crate) struct InProcessMemoryOracle<H> {
        handler: H,
        response: VecDeque<u32>,
    }

    impl<H> InProcessMemoryOracle<H> {
        pub(crate) fn new(handler: H) -> Self {
            Self {
                handler,
                response: VecDeque::new(),
            }
        }
    }

    impl<H: FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> MemoryOracle
        for InProcessMemoryOracle<H>
    {
        type Error = HintError;

        /// The handler runs during the query, with the memory of this process, which holds a live
        /// request (the contract of `send_query`): it may answer with `answer_query`.
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            // SAFETY: the querier, in this process, exposes the request during this call
            let memory = unsafe { NativeQuerierMemory::new() };
            let response = (self.handler)(query_id, input_word, &memory);
            if !self.response.is_empty() {
                self.response.clear();
                return Err(HintError(
                    "previous oracle response was not consumed in full",
                ));
            }
            self.response = response.into();
            Ok(())
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.response
                .pop_front()
                .ok_or(HintError("oracle response is shorter than expected"))
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            if self.response.len() < num_words {
                return Err(HintError("oracle response is shorter than expected"));
            }
            for (i, word) in self.response.drain(..num_words).enumerate() {
                // SAFETY: `i < num_words`, guaranteed by the caller
                unsafe { dst.add(i).write(word) };
            }
            Ok(())
        }

        fn finish_query(&mut self) -> Result<(), HintError> {
            if self.response.is_empty() {
                Ok(())
            } else {
                self.response.clear();
                Err(HintError("oracle response contains excess data"))
            }
        }
    }

    /// Sparse guest memory: unwritten words are zero.
    #[derive(Default)]
    pub(crate) struct TestMemorySource {
        words: BTreeMap<u32, u32>,
    }

    impl TestMemorySource {
        pub(crate) fn insert_u32(&mut self, address: u32, value: u32) {
            assert!(address.is_multiple_of(4));
            self.words.insert(address, value);
        }
    }

    /// The memory of the RISC-V guest, as the oracle reads it.
    pub(crate) struct GuestMemory<'a>(pub(crate) &'a TestMemorySource);

    impl QuerierMemory for GuestMemory<'_> {
        fn word_size(&self) -> usize {
            size_of::<u32>()
        }

        fn read_u32(&self, address: usize) -> Result<u32, QuerierMemoryError> {
            let address = u32::try_from(address)
                .map_err(|_| QuerierMemoryError("guest address does not fit into u32"))?;
            if !address.is_multiple_of(4) {
                return Err(QuerierMemoryError("unaligned guest address"));
            }
            Ok(self.0.words.get(&address).copied().unwrap_or(0))
        }
    }

    /// The op of the hint request at `input_word`
    fn read_op(memory: &dyn QuerierMemory, input_word: usize) -> FieldHintOp {
        let (op, _, _) = read_field_hint_request(memory, input_word).unwrap();
        FieldHintOp::parse_u32(op).unwrap()
    }

    /// The honest answer of a native run
    ///
    /// # Safety
    ///
    /// As for `answer_query` with a querier in this process.
    unsafe fn native_answer(
        query_id: u32,
        input_word: usize,
        memory: &dyn QuerierMemory,
    ) -> Vec<u32> {
        let mut words = Vec::new();
        // SAFETY: guaranteed by the caller
        let answered =
            unsafe { answer_query(query_id, input_word, memory, Some(&mut words), None) };
        assert_eq!(answered, Ok(true));
        words
    }

    /// An oracle that answers the queries of `ops` with the low bit of one word flipped, and the
    /// others honestly: the last word if `last_word`, otherwise the first element's low word (past
    /// the claim flag of the pairing ops), which stays canonical so the value check is what fails
    fn flipping_oracle(
        ops: &[FieldHintOp],
        last_word: bool,
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        let ops = ops.to_vec();
        InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                let op = read_op(memory, input_word);
                // SAFETY: the handler runs during the query, while its request is live
                let mut words = unsafe { native_answer(query_id, input_word, memory) };
                if ops.contains(&op) {
                    let index = match op {
                        _ if last_word => words.len() - 1,
                        FieldHintOp::Bn254PairingResidueWitness
                        | FieldHintOp::Bls12381KzgResidueWitness
                        | FieldHintOp::Bls12381PairingResidueWitness
                        | FieldHintOp::Secp256k1RecoveryWitness
                        | FieldHintOp::Bn254G1MulWitness => 1,
                        _ => 0,
                    };
                    // an answer that is only its flag (no witness) has the flag flipped
                    let index = index.min(words.len() - 1);
                    words[index] ^= 1;
                }
                words
            },
        )
    }

    /// An oracle that corrupts the answers of `ops` (their first payload word)
    pub(crate) fn lying_oracle(
        ops: &[FieldHintOp],
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        flipping_oracle(ops, false)
    }

    /// An oracle that corrupts the last word of the answers of `ops`
    pub(crate) fn lying_oracle_last_word(
        ops: &[FieldHintOp],
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        flipping_oracle(ops, true)
    }

    /// An oracle that claims every pairing product not to be the identity, with the (correct)
    /// inverse the exact path needs
    pub(crate) fn non_identity_claiming_oracle(
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        InProcessMemoryOracle::new(
            |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                let op = read_op(memory, input_word);
                // SAFETY: the handler runs during the query, while its request is live
                unsafe {
                    match op {
                        FieldHintOp::Bn254PairingResidueWitness
                        | FieldHintOp::Bls12381KzgResidueWitness
                        | FieldHintOp::Bls12381PairingResidueWitness => {
                            not_identity_claim(memory, input_word)
                        }
                        _ => native_answer(query_id, input_word, memory),
                    }
                }
            },
        )
    }
}

/// Runs `f` with a recorder of the tape that a querier with the fields of this build reads when
/// it runs the same computation: the tape of the guest with the bigint delegation in a build
/// with `cfg(bigint_ops)`, the native one otherwise (the fields of a 64-bit guest without the
/// delegation). The two differ wherever the representations do, as for a bls12-381 element.
pub(crate) fn record_build_tape<R>(
    f: impl FnOnce(&mut crate::hints::RecordingHintOracle) -> R,
) -> (R, Vec<u32>) {
    let target = if cfg!(bigint_ops) {
        crate::hints::HintTarget::Guest
    } else {
        crate::hints::HintTarget::Native
    };
    let mut recorder = crate::hints::RecordingHintOracle::recording_for(target);
    let result = f(&mut recorder);
    (result, recorder.into_tape())
}
