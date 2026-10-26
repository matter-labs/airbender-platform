//! Helpers of the native tests: the C recording API around calls.
// Each test crate uses a subset.
#![allow(dead_code)]

use airbender_crypto_c::*;
use core::fmt::Debug;
use core::ptr;

/// The result of `f`, and the words that it records through the C API
pub fn recorded<T>(f: impl FnOnce() -> T) -> (T, Vec<u32>) {
    airbender_crypto_record_begin();
    let result = f();
    (result, take_recording())
}

/// Ends the recording of this thread and returns its words, as a C host does: a first call that
/// only gets the number of words (`record_end(NULL, 0)`), then one with room for them
pub fn take_recording() -> Vec<u32> {
    unsafe {
        let len = airbender_crypto_record_end(ptr::null_mut(), 0);
        let mut words = vec![0u32; len];
        assert_eq!(airbender_crypto_record_end(words.as_mut_ptr(), len), len);
        words
    }
}

/// The result of `f` without a recording, which recording does not change, and the words of the
/// recording
pub fn with_and_without_recording<T: PartialEq + Debug>(f: impl Fn() -> T) -> (T, Vec<u32>) {
    let plain = f();
    let (hinted, tape) = recorded(&f);
    assert_eq!(hinted, plain, "a recording changed the result");
    (plain, tape)
}
