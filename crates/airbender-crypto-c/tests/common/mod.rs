//! Helpers of the native tests.
// Each test crate uses a subset.
#![allow(dead_code)]

use airbender_crypto::ark_ec::short_weierstrass::{Affine, SWCurveConfig};
use airbender_crypto::ark_ec::AffineRepr;
use airbender_crypto::ark_ff::{BigInteger, Field, PrimeField};
use airbender_crypto_c::*;
use core::fmt::Debug;
use core::ptr;

/// The result of `f`, and the words that it records through the C API
pub fn recorded<T>(f: impl FnOnce() -> T) -> (T, Vec<u32>) {
    airbender_crypto_record_begin();
    let result = f();
    (result, take_recording())
}

/// Ends the recording as a C host does: a size query, then a call with room for the words
pub fn take_recording() -> Vec<u32> {
    unsafe {
        let len = airbender_crypto_record_end(ptr::null_mut(), 0);
        let mut words = vec![0u32; len];
        assert_eq!(airbender_crypto_record_end(words.as_mut_ptr(), len), len);
        words
    }
}

/// The result of `f`, which a recording must not change, and the recorded words
pub fn with_and_without_recording<T: PartialEq + Debug>(f: impl Fn() -> T) -> (T, Vec<u32>) {
    let plain = f();
    let (hinted, tape) = recorded(&f);
    assert_eq!(hinted, plain, "a recording changed the result");
    (plain, tape)
}

/// The status and output of `f` on an output set to `initial`, without and with a recording, and
/// the recorded words
pub fn call<T: Copy + PartialEq + Debug>(
    initial: T,
    f: impl Fn(&mut T) -> i32,
) -> ((i32, T), Vec<u32>) {
    with_and_without_recording(|| {
        let mut output = initial;
        (f(&mut output), output)
    })
}

/// The C encoding of `values`: canonical big-endian base-field components, c0 first
pub fn encode<F: Field, const N: usize>(values: &[F]) -> Bytes<N> {
    let components: Vec<_> = values
        .iter()
        .flat_map(|value| value.to_base_prime_field_elements())
        .collect();
    let mut output = Bytes { data: [0; N] };
    let width = N / components.len();
    for (dst, component) in output.data.chunks_exact_mut(width).zip(components) {
        // the delegated bls12-381 big integers are wider: the value is in the low bytes
        let bytes = component.into_bigint().to_bytes_be();
        dst.copy_from_slice(&bytes[bytes.len() - width..]);
    }
    output
}

/// The C encoding of an affine point: x || y, all zero for infinity
pub fn point<C: SWCurveConfig, const N: usize>(p: &Affine<C>) -> Bytes<N> {
    match p.xy() {
        Some((x, y)) => encode(&[x, y]),
        None => Bytes { data: [0; N] },
    }
}
