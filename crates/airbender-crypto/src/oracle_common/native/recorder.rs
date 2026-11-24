//! Native recording of the hint answers the RISC-V guest reads (the hint part of the prover input).

use super::{answer_query, NativeQuerierMemory};
use crate::hints::{HintError, HintTarget};
use airbender_oracle::MemoryOracle;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// The oracle of a recording native run: it answers a querier in this process, and records the
/// guest's answers to the same queries, which the guest reads as its non-determinism input when it
/// runs the same computation.
///
/// Each query's whole guest answer is appended to the tape when it is sent, with no framing: in
/// the representation of the guest with the bigint delegation (`HintTarget::Guest`), or, with
/// [`recording_for`](Self::recording_for), the native one of a guest without it. Reading past the
/// native answer is an error that consumes nothing; ending the query or sending the next one
/// before the answer is read in full is an error that drops the rest (the guest would read it as
/// the start of its next answer).
///
/// A 64-bit guest without the delegation reads its answers packed, two words to an advice word
/// wherever it takes two at once (`airbender_oracle::PackedWordChannelOracle`). What an advice
/// word carries then follows from the reads, so the recorder of
/// [`recording_packed_native`](Self::recording_packed_native) builds that tape as the querier
/// reads, not when the query is sent: the computation recorded must read as the guest does, which
/// the same code built for a 64-bit host without `bigint_ops` does.
///
/// Recordings on different threads are independent, with the emulated delegation too: its
/// scratch space is per thread on a host.
///
/// Trust boundary: it dereferences each query's request and operands, per the `unsafe` contract
/// of [`MemoryOracle::send_query`]; its answers are trusted by no one, as the hinted operations
/// check them.
#[derive(Debug)]
pub struct RecordingHintOracle {
    /// Words of the answer to the current query that the querier did not read yet, as it reads
    /// them
    response: VecDeque<u32>,
    /// The answers of the guest, the words it reads, in query order
    tape: Vec<u32>,
    /// The representation the tape is recorded in
    target: HintTarget,
    /// The advice words of a packed guest, as the reads make them, if they are recorded
    packed_tape: Option<Vec<u64>>,
}

impl Default for RecordingHintOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordingHintOracle {
    /// A recorder of the tape of the guest with the bigint delegation
    pub fn new() -> Self {
        Self::recording_for(HintTarget::Guest)
    }

    /// A recorder of the tape of a guest that reads answers in the representation of `target`:
    /// [`HintTarget::Native`] for a guest without `bigint_ops`
    pub fn recording_for(target: HintTarget) -> Self {
        Self {
            response: VecDeque::new(),
            tape: Vec::new(),
            target,
            packed_tape: None,
        }
    }

    /// A recorder of the tape of a 64-bit guest without `bigint_ops` on the packed transport:
    /// the native answers, in the advice words that the reads of the querier take them in (see
    /// [`packed_tape`](Self::packed_tape)). The unpacked tape of [`tape`](Self::tape) is
    /// recorded too, as by `recording_for(HintTarget::Native)`.
    pub fn recording_packed_native() -> Self {
        Self {
            packed_tape: Some(Vec::new()),
            ..Self::recording_for(HintTarget::Native)
        }
    }

    /// The advice words read so far, as a packed guest reads them: one for a `read_word` (the
    /// word in its low half), one for a `read_u64`, and for a `write_words` one for every two
    /// words (the earlier one in the low half) and one for the last of an odd number. Empty
    /// unless the recorder is one of [`recording_packed_native`](Self::recording_packed_native).
    pub fn packed_tape(&self) -> &[u64] {
        self.packed_tape.as_deref().unwrap_or(&[])
    }

    /// The advice words read, as a packed guest reads them (see
    /// [`packed_tape`](Self::packed_tape))
    pub fn into_packed_tape(self) -> Vec<u64> {
        self.packed_tape.unwrap_or_default()
    }

    /// Takes the next `count` words of the answer to the current query, if it has them
    fn take(
        &mut self,
        count: usize,
    ) -> Result<alloc::collections::vec_deque::Drain<'_, u32>, HintError> {
        if self.response.len() < count {
            return Err(HintError("oracle response is shorter than expected"));
        }
        Ok(self.response.drain(..count))
    }

    /// The answers recorded so far, as the guest reads them
    pub fn tape(&self) -> &[u32] {
        &self.tape
    }

    /// The answers recorded, as the guest reads them
    pub fn into_tape(self) -> Vec<u32> {
        self.tape
    }
}

/// The querier runs in this process.
impl MemoryOracle for RecordingHintOracle {
    type Error = HintError;

    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        if !self.response.is_empty() {
            self.response.clear();
            return Err(HintError(
                "previous oracle response was not consumed in full",
            ));
        }
        let mut native_run_response = Vec::new();
        let mut guest_run_response = Vec::new();
        // SAFETY: the querier exposes the request during this call
        let memory = unsafe { NativeQuerierMemory::new() };
        // SAFETY: the contract of `send_query`, guaranteed by the caller
        let answered = unsafe {
            answer_query(
                query_id,
                input_word,
                &memory,
                Some(&mut native_run_response),
                Some(&mut guest_run_response),
            )
        }?;
        if !answered {
            return Err(HintError("invalid query ID"));
        }
        // the responses of both runs, and only those
        if native_run_response.is_empty() != guest_run_response.is_empty() {
            return Err(HintError(
                "the query processor does not respond for the run mode",
            ));
        }
        match self.target {
            HintTarget::Guest => self.tape.extend(guest_run_response),
            HintTarget::Native => self.tape.extend_from_slice(&native_run_response),
        }
        self.response = native_run_response.into();
        Ok(())
    }

    fn read_word(&mut self) -> Result<u32, HintError> {
        let word = self
            .response
            .pop_front()
            .ok_or(HintError("oracle response is shorter than expected"))?;
        if let Some(tape) = &mut self.packed_tape {
            tape.push(u64::from(word));
        }
        Ok(word)
    }

    fn read_u64(&mut self) -> Result<u64, HintError> {
        let mut words = self.take(2)?;
        let low = words.next().expect("two words were taken");
        let high = words.next().expect("two words were taken");
        drop(words);
        let value = u64::from(low) | u64::from(high) << 32;
        if let Some(tape) = &mut self.packed_tape {
            tape.push(value);
        }
        Ok(value)
    }

    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
        let words: Vec<u32> = self.take(num_words)?.collect();
        for (i, word) in words.iter().enumerate() {
            // SAFETY: `i < num_words`, guaranteed by the caller
            unsafe { dst.add(i).write(*word) };
        }
        if let Some(tape) = &mut self.packed_tape {
            tape.extend(words.chunks(2).map(|pair| {
                // the last of an odd number is alone, in the low half
                u64::from(pair[0]) | pair.get(1).map_or(0, |high| u64::from(*high) << 32)
            }));
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

/// Runs `f` with a new recorder, and returns its result and the tape the guest reads when it runs
/// the same computation.
pub fn record_hints<R>(f: impl FnOnce(&mut RecordingHintOracle) -> R) -> (R, Vec<u32>) {
    let mut recorder = RecordingHintOracle::new();
    let result = f(&mut recorder);
    (result, recorder.into_tape())
}

/// As [`record_hints`], for a 64-bit guest without `bigint_ops` on the packed transport: the
/// advice words it reads when it runs the same computation (see
/// [`RecordingHintOracle::recording_packed_native`]).
pub fn record_hints_packed<R>(f: impl FnOnce(&mut RecordingHintOracle) -> R) -> (R, Vec<u64>) {
    let mut recorder = RecordingHintOracle::recording_packed_native();
    let result = f(&mut recorder);
    (result, recorder.into_packed_tape())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::{send_field_hint_query, FieldHintOp, Secp256k1Element};
    use crate::hints::{HintError, MemoryOracle, Secp256k1HooksWithOracle};
    use crate::secp256k1::field::FieldElement;
    use crate::secp256k1::hooks::Secp256k1Hooks;

    /// Sends the query for the base field inverse of `x`
    fn send_inverse(recorder: &mut RecordingHintOracle, x: &FieldElement) -> Result<(), HintError> {
        // SAFETY: `FieldElement` is the operand type of the base field inverse
        unsafe { send_field_hint_query(recorder, FieldHintOp::Secp256k1BaseFieldInverse, x) }
    }

    /// The native answer to the current query
    fn read_element(recorder: &mut RecordingHintOracle) -> Result<[u32; 8], HintError> {
        let mut words = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { recorder.write_words(words.as_mut_ptr(), 8) }.map(|()| words)
    }

    /// An element and its inverse
    fn element_and_inverse() -> (FieldElement, FieldElement) {
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        let mut inverse = x;
        inverse.invert_in_place();
        (x, inverse)
    }

    #[test]
    fn recorder_serves_the_protocol() {
        let (x, inverse) = element_and_inverse();
        let mut recorder = RecordingHintOracle::new();
        // nothing to read before a query, and an unknown query is not recorded
        assert!(recorder.read_word().is_err());
        assert_eq!(
            // SAFETY: nothing is read for a query ID that is not answered
            unsafe { recorder.send_query(0x1234_5678, 0) },
            Err(HintError("invalid query ID"))
        );
        // the whole guest answer is recorded when the query is sent
        send_inverse(&mut recorder, &x).unwrap();
        assert_eq!(recorder.tape(), inverse.to_montgomery_words());
        // a short read fails, and reads nothing
        let mut words = [0u32; 9];
        // SAFETY: the buffer holds 9 words
        assert!(unsafe { recorder.write_words(words.as_mut_ptr(), 9) }.is_err());
        assert_eq!(
            read_element(&mut recorder),
            Ok(inverse.to_canonical_words())
        );
        assert_eq!(recorder.finish_query(), Ok(()));
        assert!(recorder.read_word().is_err());

        // through the hooks, which read the native answer into the element
        let mut y = x;
        let ((), tape) = record_hints(|recorder| {
            Secp256k1HooksWithOracle::new(recorder).fe_invert_and_assign(&mut y)
        });
        assert_eq!(y.to_bytes(), inverse.to_bytes());
        assert_eq!(tape, inverse.to_montgomery_words());

        // a tape for a guest without the delegation: the native words
        let mut native = RecordingHintOracle::recording_for(HintTarget::Native);
        send_inverse(&mut native, &x).unwrap();
        assert_eq!(native.tape(), inverse.to_canonical_words());
        assert_eq!(read_element(&mut native), Ok(inverse.to_canonical_words()));
    }

    /// The packed recorder answers as the unpacked one of the native target does, and its
    /// advice words are those words as the reads take them
    #[test]
    fn the_packed_tape_follows_the_reads() {
        let (x, inverse) = element_and_inverse();
        let words = inverse.to_canonical_words();
        let pair = |i: usize| u64::from(words[i]) | u64::from(words[i + 1]) << 32;

        let mut unpacked = RecordingHintOracle::recording_for(HintTarget::Native);
        send_inverse(&mut unpacked, &x).unwrap();
        assert_eq!(read_element(&mut unpacked), Ok(words));
        // no advice words unless they are recorded
        assert!(unpacked.packed_tape().is_empty());

        // one `write_words`: a word for every two
        let mut recorder = RecordingHintOracle::recording_packed_native();
        send_inverse(&mut recorder, &x).unwrap();
        // nothing is recorded before it is read
        assert!(recorder.packed_tape().is_empty());
        assert_eq!(recorder.tape(), unpacked.tape());
        assert_eq!(read_element(&mut recorder), Ok(words));
        assert_eq!(recorder.finish_query(), Ok(()));
        assert_eq!(recorder.packed_tape(), [pair(0), pair(2), pair(4), pair(6)]);

        // a word, a pair, an odd number of words, a word: the same answer, in other advice
        // words
        send_inverse(&mut recorder, &x).unwrap();
        assert_eq!(recorder.read_word(), Ok(words[0]));
        assert_eq!(recorder.read_u64(), Ok(pair(1)));
        let mut rest = [0u32; 3];
        // SAFETY: the buffer holds 3 words
        unsafe { recorder.write_words(rest.as_mut_ptr(), 3) }.unwrap();
        assert_eq!(rest, words[3..6]);
        // a read past the answer fails, takes nothing and records nothing
        let mut long = [0u32; 3];
        // SAFETY: the buffer holds 3 words
        assert!(unsafe { recorder.write_words(long.as_mut_ptr(), 3) }.is_err());
        assert_eq!(recorder.read_u64(), Ok(pair(6)));
        assert!(recorder.read_u64().is_err());
        assert!(recorder.read_word().is_err());
        assert_eq!(recorder.finish_query(), Ok(()));
        assert_eq!(
            recorder.packed_tape()[4..],
            [
                u64::from(words[0]),
                pair(1),
                pair(3),
                u64::from(words[5]),
                pair(6)
            ]
        );
        assert_eq!(recorder.tape(), [words, words].concat());

        // one word left: a pair is not read, and the query does not end
        send_inverse(&mut recorder, &x).unwrap();
        let mut most = [0u32; 7];
        // SAFETY: the buffer holds 7 words
        unsafe { recorder.write_words(most.as_mut_ptr(), 7) }.unwrap();
        assert!(recorder.read_u64().is_err());
        assert_eq!(
            recorder.finish_query(),
            Err(HintError("oracle response contains excess data"))
        );
        assert_eq!(recorder.packed_tape().len(), 9 + 4);
        assert_eq!(recorder.packed_tape()[12], u64::from(words[6]));

        // through the helper
        let mut y = x;
        let ((), tape) = record_hints_packed(|recorder| {
            Secp256k1HooksWithOracle::new(recorder).fe_invert_and_assign(&mut y)
        });
        assert_eq!(y.to_bytes(), inverse.to_bytes());
        assert_eq!(tape, [pair(0), pair(2), pair(4), pair(6)]);
        assert_eq!(
            RecordingHintOracle::recording_packed_native().into_packed_tape(),
            []
        );
    }

    /// An answer left partly unread is an error: the guest would read its rest on the tape as the
    /// start of the next answer
    #[test]
    fn unread_answers_are_errors() {
        let (x, inverse) = element_and_inverse();
        let mut recorder = RecordingHintOracle::new();

        // the next query, sent before the answer is read in full, fails, is not recorded, and
        // drops the rest
        send_inverse(&mut recorder, &x).unwrap();
        assert_eq!(recorder.read_word(), Ok(inverse.to_canonical_words()[0]));
        assert_eq!(
            send_inverse(&mut recorder, &inverse),
            Err(HintError(
                "previous oracle response was not consumed in full"
            ))
        );
        assert!(recorder.read_word().is_err());
        assert_eq!(recorder.tape(), inverse.to_montgomery_words());

        // ending the query before the answer is read in full fails, and drops the rest
        send_inverse(&mut recorder, &x).unwrap();
        assert_eq!(recorder.read_word(), Ok(inverse.to_canonical_words()[0]));
        assert_eq!(
            recorder.finish_query(),
            Err(HintError("oracle response contains excess data"))
        );
        assert!(recorder.read_word().is_err());

        // and the next query is served
        send_inverse(&mut recorder, &inverse).unwrap();
        assert_eq!(read_element(&mut recorder), Ok(x.to_canonical_words()));
        assert_eq!(recorder.finish_query(), Ok(()));
        assert_eq!(
            recorder.into_tape(),
            [
                inverse.to_montgomery_words(),
                inverse.to_montgomery_words(),
                x.to_montgomery_words(),
            ]
            .concat()
        );
    }
}
