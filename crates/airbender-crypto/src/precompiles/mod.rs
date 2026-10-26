//! The crypto half of the zksync-os precompiles, as their wrappers call it: the EVM half (gas,
//! the padding and length checks of the input, the mapping of the errors, and the encoding of
//! the output) stays in zksync-os.
//!
//! Every endpoint takes an optional oracle (see [`crate::hints`]). With one, it runs the hinted
//! path of zksync-os `e855bca2`, which asks the same hints in the same order; with none
//! (`None::<&mut NoHints>`), its path without advice. The results are the same.
//!
//! On RV32 the hints answer straight into the delegated fields, so they need `bigint_ops`
//! (see `hints::guest_layout`). Without it, the endpoints still run without an oracle, and
//! panic when given one. `secp256k1_ecrecover` needs `secp256k1-static-context` (a default
//! feature), and the endpoints that take an allocator (the pairing checks and the MSMs) need
//! `alloc`.

/// Records the hints of `$call` (an endpoint call that takes `Some($oracle)`), then replays the
/// tape through the reader of a tape, as the guest does, and checks that the replay gets the same
/// result and takes the whole tape and no more; evaluates to the result
#[cfg(all(test, target_pointer_width = "64"))]
macro_rules! assert_tape_replays {
    (|$oracle:ident| $call:expr) => {{
        const NEXT_ANSWER: u32 = 0x5eed_5eed;
        let (recorded, tape) = $crate::hints::record_hints(|$oracle| $call);
        assert!(!tape.is_empty(), "the call takes hints");
        let mut guest =
            $crate::hints::test_support::TapeOracle::new(tape.iter().copied().chain([NEXT_ANSWER]));
        let replayed = {
            let $oracle = &mut guest;
            $call
        };
        assert_eq!(replayed, recorded);
        assert_eq!(guest.words, [NEXT_ANSWER]);
        recorded
    }};
}

/// Records the hints of the calls of a property test, and counts the calls that took any, so
/// that the test does not pass on inputs that the hinted path rejects before asking anything
#[cfg(all(test, target_pointer_width = "64"))]
#[derive(Default)]
struct HintedCases(core::cell::Cell<usize>);

#[cfg(all(test, target_pointer_width = "64"))]
impl HintedCases {
    fn record<R>(&self, call: impl FnOnce(&mut crate::hints::RecordingHintOracle) -> R) -> R {
        let (result, tape) = crate::hints::record_hints(call);
        if !tape.is_empty() {
            self.0.set(self.0.get() + 1);
        }
        result
    }

    /// At least 8 of the calls took hints: an eighth of the 64 calls of a test of 64 cases, a
    /// sixteenth of the 128 calls of an addition test (two per case), and half of the
    /// `HINT_HEAVY_CASES` calls of a test of those in a debug build
    fn check(&self) {
        assert!(self.0.get() >= 8, "{} cases took hints", self.0.get());
    }
}

/// The cases of the property tests of the endpoints that ask hundreds of hints: 64, but 16 in a
/// debug build, where an answer of the recording oracle takes milliseconds (it computes on the
/// emulated delegated fields of the test build)
#[cfg(all(test, target_pointer_width = "64"))]
const HINT_HEAVY_CASES: u32 = if cfg!(debug_assertions) { 16 } else { 64 };

mod bls12_381;
mod bn254;
#[cfg(feature = "secp256k1-static-context")]
mod ecrecover;
mod kzg;
#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
mod unavailable;

pub use self::bls12_381::{
    bls12_381_g1_add, bls12_381_g2_add, bls12_381_map_fp2_to_g2, bls12_381_map_fp_to_g1,
    Bls12Error, FIELD_ELEMENT_SERIALIZATION_LEN, FIELD_EXT_ELEMENT_SERIALIZATION_LEN,
    G1_SERIALIZATION_LEN, G2_SERIALIZATION_LEN,
};
#[cfg(any(feature = "alloc", test))]
pub use self::bls12_381::{
    bls12_381_g1_msm, bls12_381_g2_msm, bls12_381_pairing_check, BLS12_381_PAIR_LEN,
    G1_MSM_PAIR_LEN, G2_MSM_PAIR_LEN,
};
#[cfg(any(feature = "alloc", test))]
pub use self::bn254::bn254_pairing_check;
pub use self::bn254::{bn254_g1_add, bn254_g1_mul};
#[cfg(feature = "secp256k1-static-context")]
pub use self::ecrecover::secp256k1_ecrecover;
pub use self::kzg::{
    kzg_verify_proof, parse_g1_compressed, parse_g1_compressed_with_oracle, parse_scalar,
    verify_kzg_proof_with_oracle, KzgError, KzgScalar,
};

// Ported from zksync-os e855bca2:basic_system/src/system_functions/ecrecover.rs
/// Bytes at an address aligned for words: the conversions between big-endian bytes and integers
/// work by words then (see `crypto::bigint_delegation::u256::from_be_bytes`)
#[repr(C, align(4))]
pub struct WordAligned<const N: usize>(pub [u8; N]);

/// The hinted path of an endpoint on RV32 without `bigint_ops`, where there are no hints
#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
#[cold]
#[inline(never)]
fn hints_unavailable() -> ! {
    panic!("the hints need `bigint_ops` on RV32")
}
