//! The C recording API under misuse: every call sequence has a defined result and none panics (a
//! panic in an `extern "C"` function aborts the process). Each test starts without a recording.
mod common;

use airbender_crypto_c::*;
use common::{recorded, take_recording};
use core::ptr::{self, NonNull};

fn inverse(value: u8) -> Bytes<32> {
    let mut a = Bytes { data: [0; 32] };
    a.data[31] = value;
    let mut result = Bytes { data: [0; 32] };
    assert_eq!(
        unsafe { airbender_secp256k1_fe_inverse(&a, &mut result) },
        EOK
    );
    result
}

/// `record_end` into a buffer of `capacity` words (NULL for none)
fn end(capacity: usize) -> usize {
    let mut words = vec![0u32; capacity];
    let words = if capacity == 0 {
        ptr::null_mut()
    } else {
        words.as_mut_ptr()
    };
    unsafe { airbender_crypto_record_end(words, capacity) }
}

#[test]
fn end_without_a_recording_returns_no_words() {
    assert_eq!(end(0), 0);
    // nor after a call made without a recording, which records nothing
    inverse(3);
    assert_eq!(end(16), 0);
    // a dangling pointer with a zero capacity is valid too
    let dangling = NonNull::dangling().as_ptr();
    assert_eq!(unsafe { airbender_crypto_record_end(dangling, 0) }, 0);
}

/// `record_end` keeps the words while they do not fit: a size query, then a call with room for
/// them; `begin` drops the words that were never copied
#[test]
fn ends_that_do_not_fit_keep_the_words() {
    let ((), alone) = recorded(|| {
        inverse(3);
    });
    assert!(!alone.is_empty());
    airbender_crypto_record_begin();
    inverse(3);
    assert_eq!(end(0), alone.len());
    let mut words = vec![0u32; alone.len()];
    let short = unsafe { airbender_crypto_record_end(words.as_mut_ptr(), alone.len() - 1) };
    assert_eq!(
        (short, words.iter().all(|word| *word == 0)),
        (alone.len(), true)
    );
    assert_eq!(take_recording(), alone);
    assert_eq!(end(0), 0);

    airbender_crypto_record_begin();
    inverse(3);
    assert_ne!(end(0), 0);
    airbender_crypto_record_begin();
    assert_eq!(end(0), 0);
}

#[test]
fn a_nested_begin_restarts_the_recording() {
    let ((), only_b) = recorded(|| {
        inverse(5);
    });
    airbender_crypto_record_begin();
    inverse(3);
    airbender_crypto_record_begin();
    inverse(5);
    assert_eq!(take_recording(), only_b);
}

#[test]
fn calls_after_end_are_not_recorded() {
    airbender_crypto_record_begin();
    inverse(3);
    let first = take_recording();
    inverse(5);
    assert_eq!(end(16), 0);
    let ((), again) = recorded(|| {
        inverse(3);
    });
    assert_eq!(first, again);
}

#[test]
fn failing_calls_record_nothing_and_keep_the_recording_usable() {
    let ((), alone) = recorded(|| {
        inverse(3);
    });
    airbender_crypto_record_begin();
    let mut output = Bytes { data: [0xa5; 32] };
    let status = unsafe { airbender_secp256k1_fe_inverse(&Bytes { data: [0; 32] }, &mut output) };
    assert_eq!((status, output.data), (EFAIL, [0xa5; 32]));
    inverse(3);
    assert_eq!(take_recording(), alone);
}

/// Threads that record at once each get the tape of their own calls, recorded alone
#[test]
fn concurrent_recordings_are_independent() {
    use airbender_crypto::k256::ecdsa::SigningKey;
    let recover = |n: u8| {
        let key = SigningKey::from_bytes(&[n + 1; 32].into()).unwrap();
        let msg = Bytes { data: [n; 32] };
        let (sig, id) = key.sign_prehash_recoverable(&msg.data).unwrap();
        let mut output = Bytes { data: [0; 64] };
        let status = unsafe {
            let sig = Bytes {
                data: sig.to_bytes().into(),
            };
            zkvm_secp256k1_ecrecover(&msg, &sig, id.to_byte(), &mut output)
        };
        assert_eq!(status, EOK);
        output
    };
    let thread = |t: u8| recorded(|| (0..4).map(|i| recover(4 * t + i)).collect::<Vec<_>>());
    let alone: Vec<_> = (0..8).map(thread).collect();
    for (i, (_, tape)) in alone.iter().enumerate() {
        assert!(alone[i + 1..].iter().all(|(_, other)| other != tape));
    }
    // recorded at once, and with a recording active on this thread, which they must not reach
    airbender_crypto_record_begin();
    let start = std::sync::Barrier::new(8);
    let together: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|t| {
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    thread(t)
                })
            })
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    });
    assert_eq!(together, alone);
    assert!(take_recording().is_empty());
}
