//! The C recording API under misuse: every sequence of calls has a defined
//! result, and none panics (a panic in an `extern "C"` function aborts this test process).
//! Each test runs on its own thread, so each starts without a recording.
mod common;

use airbender_crypto_c::*;
use common::{recorded, take_recording};
use core::ptr;

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
fn end_without_begin_returns_no_words() {
    assert_eq!(end(0), 0);
    assert_eq!(end(16), 0);
    // nor after a call made without a recording
    inverse(3);
    assert_eq!(end(16), 0);
}

/// `record_end(NULL, 0)` (`end(0)`) is a size query: it returns the number of words and keeps
/// them, so the next call (another query, then one with room for them) gets the same words
#[test]
fn a_null_end_with_zero_capacity_queries_the_size() {
    let ((), alone) = recorded(|| {
        inverse(3);
    });
    assert!(!alone.is_empty());
    airbender_crypto_record_begin();
    inverse(3);
    assert_eq!(end(0), alone.len());
    assert_eq!(end(0), alone.len());
    let mut words = vec![0u32; alone.len()];
    assert_eq!(
        unsafe { airbender_crypto_record_end(words.as_mut_ptr(), words.len()) },
        alone.len()
    );
    assert_eq!(words, alone);
    // the words were copied, and without a recording there are none
    assert_eq!(end(0), 0);
    // nor for an empty recording, which a size query ends
    airbender_crypto_record_begin();
    assert_eq!(end(0), 0);
    inverse(3);
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
fn begin_drops_the_words_that_did_not_fit() {
    airbender_crypto_record_begin();
    inverse(3);
    let len = end(0);
    assert_ne!(len, 0);
    // kept: the next end reports them again
    assert_eq!(end(0), len);
    airbender_crypto_record_begin();
    assert_eq!(end(0), 0);
}

#[test]
fn calls_without_a_recording_take_the_path_without_hints() {
    let ((), tape) = recorded(|| {
        inverse(7);
    });
    assert!(!tape.is_empty());
    // the same result without a recording, and nothing is recorded for a later end
    let plain = inverse(7);
    assert_eq!(end(16), 0);
    let (hinted, _) = recorded(|| inverse(7));
    assert_eq!(plain, hinted);
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
fn recordings_are_per_thread() {
    let ((), alone) = recorded(|| {
        inverse(3);
    });
    airbender_crypto_record_begin();
    inverse(3);
    // another thread's calls, recorded or not, do not reach this recording
    std::thread::spawn(|| {
        inverse(5);
        let ((), other) = recorded(|| {
            inverse(9);
        });
        assert!(!other.is_empty());
    })
    .join()
    .unwrap();
    assert_eq!(take_recording(), alone);
}

#[test]
fn failing_calls_record_nothing_and_keep_the_recording_usable() {
    let ((), alone) = recorded(|| {
        inverse(3);
    });
    airbender_crypto_record_begin();
    let zero = Bytes { data: [0; 32] };
    let mut output = Bytes { data: [0xa5; 32] };
    assert_eq!(
        unsafe { airbender_secp256k1_fe_inverse(&zero, &mut output) },
        EFAIL
    );
    assert_eq!(output.data, [0xa5; 32]);
    inverse(3);
    assert_eq!(take_recording(), alone);
}
