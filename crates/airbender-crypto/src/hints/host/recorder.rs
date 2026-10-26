// Ported from zksync-os e855bca2:oracle_provider/src/lib.rs
//! Native recording of the hint answers the RISC-V guest reads: the hint part of the prover input
//! that zksync-os records with `ZkEENonDeterminismSource` in the `NativeRunSavingForRiscV` mode and
//! `ReadWitnessSource`.

use super::{answer_query, NativeQuerierMemory};
use crate::hints::{HintError, HintOracle};
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// The oracle of a native run that records the prover input: it answers the queries of a querier
/// in this process, and records the answers of the RISC-V guest, which the guest reads in place of
/// that querier when it runs the same computation (with the same queries, in the same order), as
/// its non-determinism input.
///
/// Each query is answered for both targets when it is sent, and the whole answer of the guest is
/// then appended to the tape, whatever the querier reads of its own answer. The tape is the words
/// of the answers, in query order, with no framing added between them (the length words that a
/// ModExp reduction answers with are part of its answer). The answer of the querier is served
/// as zksync-os serves a querier in this process: reading past its end is an error (which
/// consumes nothing), and ending the query before it is read in full, or sending the next query
/// before then, is an error that drops the unread rest. The guest would read such a rest as the
/// start of its next answer.
///
/// It reads nothing but the memory of the queries, and keeps its state in itself: recordings on
/// different threads, or with different recorders, are independent, as long as no crate in the
/// build turns `proving` on for a native target. The emulated delegated arithmetic that `proving`
/// brings keeps one scratch space (a `static mut`) for all threads. Without `proving`, the native
/// fields hold no shared state.
///
/// # Trust boundary
///
/// The recorder trusts the input word of each query: it dereferences it as a request of this
/// process, and the operand pointers in it as values of the types the op expects. That is the
/// contract of the `unsafe` [`HintOracle::send_query`], which the hinted operations of this crate
/// (e.g. [`Secp256k1HooksWithOracle`](crate::hints::Secp256k1HooksWithOracle), or
/// `send_field_hint_query` with the operand of the op) hold to; safe code cannot send it a query
/// otherwise. Its answers, on the other hand, are trusted by no one: the hinted operations check
/// them, natively as on the guest.
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
impl HintOracle for RecordingHintOracle {
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        if !self.response.is_empty() {
            self.response.clear();
            return Err(HintError(
                "previous oracle response was not consumed in full",
            ));
        }
        let mut native_run_response = Vec::new();
        let mut guest_run_response = Vec::new();
        // SAFETY: the querier of this process exposes the memory the input word refers to while it sends
        // the query, i.e. during this call
        let memory = unsafe { NativeQuerierMemory::new() };
        // SAFETY: the querier is in this process, whose memory this is, and the input word is the
        // address of a live request whose operands are live values of the types of its op (the
        // contract of `send_query`, guaranteed by the caller)
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
            // SAFETY: `i < num_words`, and the caller guarantees `dst` is valid for `num_words` words
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

/// Runs `f` with a new recorder, and returns its result and the tape: the answers that the RISC-V
/// guest reads when it runs the same computation with its oracle.
///
/// Trust boundary: `f` gets the recorder, which dereferences the requests sent to it (see
/// [`RecordingHintOracle`]); safe code in `f` can only send it the well-formed queries of the
/// hinted operations of this crate, as sending a query is `unsafe`.
pub fn record_hints<R>(f: impl FnOnce(&mut RecordingHintOracle) -> R) -> (R, Vec<u32>) {
    let mut recorder = RecordingHintOracle::new();
    let result = f(&mut recorder);
    (result, recorder.into_tape())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::{send_field_hint_query, FieldHintOp, Secp256k1Element};
    use crate::hints::{HintError, HintOracle, Secp256k1HooksWithOracle};
    use crate::secp256k1::field::FieldElement;
    use crate::secp256k1::hooks::Secp256k1Hooks;

    /// Sends the query for the base field inverse of `x`
    fn send_inverse(recorder: &mut RecordingHintOracle, x: &FieldElement) -> Result<(), HintError> {
        // SAFETY: `x` is a `FieldElement`, the operand type of the base field inverse
        unsafe { send_field_hint_query(recorder, FieldHintOp::Secp256k1BaseFieldInverse, x) }
    }

    /// An element and its inverse
    fn element_and_inverse() -> (FieldElement, FieldElement) {
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        let mut inverse = x;
        inverse.invert_in_place();
        (x, inverse)
    }

    #[test]
    fn recorder_emits_native_and_guest_words() {
        let (x, inverse) = element_and_inverse();
        let ((), tape) = record_hints(|recorder| {
            send_inverse(recorder, &x).unwrap();
            // the whole guest answer is recorded when the query is sent
            assert_eq!(recorder.tape(), inverse.to_montgomery_words());
            let mut native = [0u32; 8];
            // SAFETY: the buffer holds 8 words
            unsafe { recorder.write_words(native.as_mut_ptr(), 8) }.unwrap();
            recorder.finish_query().unwrap();
            assert_eq!(native, inverse.to_canonical_words());
        });
        assert_eq!(tape, inverse.to_montgomery_words());

        // through the hooks, which read the native answer into the element
        let mut y = x;
        let ((), tape) = record_hints(|recorder| {
            Secp256k1HooksWithOracle::new(recorder).fe_invert_and_assign(&mut y)
        });
        assert_eq!(y.to_bytes(), inverse.to_bytes());
        assert_eq!(tape, inverse.to_montgomery_words());
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
        assert!(recorder.tape().is_empty());

        // a short read fails, and reads nothing
        send_inverse(&mut recorder, &x).unwrap();
        let mut words = [0u32; 9];
        // SAFETY: the buffer holds 9 words
        assert!(unsafe { recorder.write_words(words.as_mut_ptr(), 9) }.is_err());
        let mut native = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { recorder.write_words(native.as_mut_ptr(), 8) }.unwrap();
        assert_eq!(native, inverse.to_canonical_words());
        assert_eq!(recorder.finish_query(), Ok(()));
        assert!(recorder.read_word().is_err());
        assert_eq!(recorder.into_tape(), inverse.to_montgomery_words());
    }

    /// An answer left partly unread is an error, as zksync-os's oracle of a native run reports it
    /// (`ZkEENonDeterminismSource`, `ResponseBuffer`): the guest would read the rest of its answer
    /// on the tape as the start of the next one
    #[test]
    fn unread_answers_are_errors() {
        let (x, inverse) = element_and_inverse();
        let mut recorder = RecordingHintOracle::new();

        // the next query, sent before the answer is read in full, fails, is not answered (nor
        // recorded), and drops the rest
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
        let mut native = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { recorder.write_words(native.as_mut_ptr(), 8) }.unwrap();
        assert_eq!(recorder.finish_query(), Ok(()));
        assert_eq!(native, x.to_canonical_words());
        // the guest answers of the queries answered, in order
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

    /// An answer of several values is recorded as the guest reads them: the candidate root, then
    /// the "quadratic non-residue" flag
    #[test]
    fn recorder_records_the_whole_answer() {
        let (x, _) = element_and_inverse();
        let mut candidate = x;
        let is_square = candidate.sqrt_in_place();
        let (exists, tape) = record_hints(|recorder| {
            let mut y = x;
            Secp256k1HooksWithOracle::new(recorder).fe_sqrt_and_assign(&mut y)
        });
        assert_eq!(exists, is_square);
        assert_eq!(
            tape,
            [
                &candidate.to_montgomery_words()[..],
                &[u32::from(!is_square)]
            ]
            .concat()
        );
    }
}
