//! Native recording of the hint answers the RISC-V guest reads (the hint part of the prover input).

use super::{answer_query, NativeQuerierMemory};
use crate::hints::HintError;
use airbender_oracle::MemoryOracle;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// The oracle of a recording native run: it answers a querier in this process, and records the
/// guest's answers to the same queries, which the guest reads as its non-determinism input when it
/// runs the same computation.
///
/// Each query's whole guest answer is appended to the tape when it is sent, with no framing.
/// Reading past the native answer is an error that consumes nothing; ending the query or sending
/// the next one before the answer is read in full is an error that drops the rest (the guest would
/// read it as the start of its next answer).
///
/// Recordings on different threads are independent unless `proving` is on for a native target:
/// its emulated delegation keeps one `static mut` scratch space for all threads.
///
/// Trust boundary: it dereferences each query's request and operands, per the `unsafe` contract
/// of [`MemoryOracle::send_query`]; its answers are trusted by no one, as the hinted operations
/// check them.
#[derive(Debug, Default)]
pub struct RecordingHintOracle {
    /// Words of the answer to the current query that the querier did not read yet, as it reads
    /// them
    response: VecDeque<u32>,
    /// The answers of the guest, the words it reads, in query order
    tape: Vec<u32>,
}

impl RecordingHintOracle {
    pub fn new() -> Self {
        Self {
            response: VecDeque::new(),
            tape: Vec::new(),
        }
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
        self.response = native_run_response.into();
        self.tape.extend(guest_run_response);
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

/// Runs `f` with a new recorder, and returns its result and the tape the guest reads when it runs
/// the same computation.
pub fn record_hints<R>(f: impl FnOnce(&mut RecordingHintOracle) -> R) -> (R, Vec<u32>) {
    let mut recorder = RecordingHintOracle::new();
    let result = f(&mut recorder);
    (result, recorder.into_tape())
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
