//! The recording of this thread, for the C recording API: a C caller cannot pass a recorder
//! through its calls, so the exports take the recorder of the recording that is active on the
//! calling thread (see [`with_active_recorder`]). Native builds only.

use crate::crypto::hints::RecordingHintOracle;
use alloc::vec::Vec;
use core::cell::RefCell;

std::thread_local! {
    /// The recording of this thread, between `begin_recording` and `end_recording`
    static ACTIVE_RECORDER: RefCell<Option<RecordingHintOracle>> = const { RefCell::new(None) };
}

/// Starts a recording on this thread, for recording across calls that cannot pass a recorder
/// (the C exports): the hinted operations then get the recorder from [`with_active_recorder`]. A
/// recording in progress on this thread starts over.
pub(crate) fn begin_recording() {
    ACTIVE_RECORDER.set(Some(RecordingHintOracle::new()));
}

/// Ends the recording of this thread, and returns its tape (see
/// `airbender_crypto::hints::record_hints`); no words when there is no recording.
pub(crate) fn end_recording() -> Vec<u32> {
    ACTIVE_RECORDER
        .take()
        .map(RecordingHintOracle::into_tape)
        .unwrap_or_default()
}

/// Runs `f` with the recorder of the recording of this thread, `None` when there is none.
///
/// Trust boundary: as for `airbender_crypto::hints::record_hints`, `f` gets the recorder, which
/// dereferences the requests sent to it (see [`RecordingHintOracle`]); safe code in `f` can only
/// send it the well-formed queries of airbender-crypto's hinted operations, as sending a query is
/// `unsafe`.
///
/// # Panics
///
/// If `f` calls this function, [`begin_recording`] or [`end_recording`]: the recorder is borrowed
/// while `f` runs.
pub(crate) fn with_active_recorder<R>(f: impl FnOnce(Option<&mut RecordingHintOracle>) -> R) -> R {
    ACTIVE_RECORDER.with_borrow_mut(|recorder| f(recorder.as_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hints::Secp256k1HooksWithOracle;
    use crate::crypto::hints::{record_hints, HintOracle, Secp256k1Element};
    use crate::crypto::secp256k1::field::FieldElement;
    use crate::crypto::secp256k1::hooks::Secp256k1Hooks;

    /// An element and its inverse
    fn element_and_inverse() -> (FieldElement, FieldElement) {
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        let mut inverse = x;
        inverse.invert_in_place();
        (x, inverse)
    }

    #[test]
    fn the_recording_of_a_thread() {
        let (x, inverse) = element_and_inverse();
        let invert = || {
            with_active_recorder(|recorder| {
                let mut y = x;
                match recorder {
                    Some(recorder) => {
                        Secp256k1HooksWithOracle::new(recorder).fe_invert_and_assign(&mut y)
                    }
                    None => y.invert_in_place(),
                }
                assert_eq!(y.to_bytes(), inverse.to_bytes());
            })
        };

        // no recording: no recorder, and nothing recorded
        assert!(with_active_recorder(|recorder| recorder.is_none()));
        assert!(end_recording().is_empty());
        invert();
        assert!(end_recording().is_empty());

        begin_recording();
        assert!(with_active_recorder(|recorder| recorder.is_some()));
        invert();
        // another `begin_recording` starts over
        begin_recording();
        invert();
        invert();
        assert_eq!(
            end_recording(),
            [inverse.to_montgomery_words(), inverse.to_montgomery_words()].concat()
        );
        assert!(with_active_recorder(|recorder| recorder.is_none()));
        assert!(end_recording().is_empty());
    }

    /// ecrecover inputs (digest, r, s, y parity): a signature of a digest, for each `n`
    fn ecrecover_input(n: u8) -> ([u8; 32], [u8; 32], [u8; 32], bool) {
        use crate::crypto::k256::ecdsa::SigningKey;
        let digest = [n.wrapping_mul(37).wrapping_add(11); 32];
        let key = SigningKey::from_bytes(&[n.wrapping_add(1); 32].into()).unwrap();
        let (signature, recovery_id) = key.sign_prehash_recoverable(&digest).unwrap();
        let (r, s) = signature.split_bytes();
        (digest, r.into(), s.into(), recovery_id.is_y_odd())
    }

    /// The uncompressed public key that the hooks of `oracle` recover from the input `n`
    fn ecrecover<O: HintOracle>(n: u8, oracle: &mut O) -> Vec<u8> {
        let (digest, r, s, y_is_odd) = ecrecover_input(n);
        let mut hooks = Secp256k1HooksWithOracle::new(oracle);
        crate::crypto::secp256k1::recover_from_bytes_with_hooks(
            &digest, &r, &s, y_is_odd, &mut hooks,
        )
        .expect("the signature recovers a key")
        .to_encoded_point(false)
        .as_bytes()
        .to_vec()
    }

    /// The inputs of a thread of `concurrent_recordings_match`
    fn thread_inputs(thread: u8) -> [u8; 4] {
        core::array::from_fn(|i| 4 * thread + i as u8)
    }

    /// 8 threads record at once, through the recording of each thread, and each
    /// records the tape of the same inputs recorded alone
    #[test]
    fn concurrent_recordings_match() {
        use crate::crypto::k256::ecdsa::SigningKey;
        use std::sync::Barrier;

        const THREADS: u8 = 8;
        let expected: Vec<(Vec<Vec<u8>>, Vec<u32>)> = (0..THREADS)
            .map(|thread| {
                record_hints(|recorder| {
                    thread_inputs(thread)
                        .map(|n| ecrecover(n, &mut *recorder))
                        .to_vec()
                })
            })
            .collect();
        // the recovered keys are the signers', and the tapes differ
        for (thread, (keys, tape)) in expected.iter().enumerate() {
            for (n, key) in thread_inputs(thread as u8).into_iter().zip(keys) {
                let signer = SigningKey::from_bytes(&[n.wrapping_add(1); 32].into()).unwrap();
                assert_eq!(
                    key[..],
                    *signer.verifying_key().to_encoded_point(false).as_bytes()
                );
            }
            assert!(!tape.is_empty());
        }
        for (i, (_, a)) in expected.iter().enumerate() {
            for (_, b) in &expected[i + 1..] {
                assert_ne!(a, b);
            }
        }

        let start = Barrier::new(THREADS as usize);
        let recorded: Vec<(Vec<Vec<u8>>, Vec<u32>)> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..THREADS)
                .map(|thread| {
                    let start = &start;
                    scope.spawn(move || {
                        start.wait();
                        begin_recording();
                        let keys = thread_inputs(thread)
                            .map(|n| {
                                with_active_recorder(|recorder| {
                                    ecrecover(n, recorder.expect("recording"))
                                })
                            })
                            .to_vec();
                        (keys, end_recording())
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(|thread| thread.join().unwrap())
                .collect()
        });
        assert_eq!(recorded, expected);
    }
}
