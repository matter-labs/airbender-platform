#![no_std]
#![no_main]

use airbender::crypto::hints::HintError;
use airbender::crypto::modexp::modexp;
use airbender::crypto::precompiles::secp256k1_ecrecover;
use airbender::guest::read;
use airbender_oracle::{CsrChannel, CsrOracle};

/// Runs ModExp and ecrecover on their hinted paths: every expensive step (a multi-digit
/// division, a field inverse, a square root) is a query to the oracle, whose answer the guest
/// checks. The host serves the queries live with a `QueryDispatcher` (see `../host`).
///
/// The inputs are read first: the host serves the input words and the oracle on the same CSR,
/// and every read after a query takes the words of its answer.
#[airbender::main]
fn main() -> u32 {
    let base: [u8; 32] = read().expect("base");
    let exp: [u8; 32] = read().expect("exp");
    let modulus: [u8; 32] = read().expect("modulus");
    let digest: [u8; 32] = read().expect("digest");
    let r: [u8; 32] = read().expect("r");
    let s: [u8; 32] = read().expect("s");
    let y_is_odd: bool = read().expect("y parity");

    // The guest's oracle: the non-determinism CSR as the memory-based query protocol.
    let mut oracle = CsrOracle::<HintError>::new(CsrChannel);

    let power = modexp(&base, &exp, &modulus, Some(&mut oracle));
    let key = secp256k1_ecrecover(&digest, &r, &s, y_is_odd, Some(&mut oracle))
        .expect("the signature must recover");

    fold(&power, &key.0)
}

/// A word of both results: FNV-1a over the power as 32 big-endian bytes, then the key
fn fold(power: &[u8], key: &[u8; 64]) -> u32 {
    let mut padded = [0u8; 32];
    padded[32 - power.len()..].copy_from_slice(power);
    let mut digest = 0x811c_9dc5u32;
    for byte in padded.iter().chain(key) {
        digest ^= u32::from(*byte);
        digest = digest.wrapping_mul(16_777_619);
    }
    digest
}
