//! The crypto half of the EVM precompiles. Gas, input padding and length checks, error mapping
//! and output encoding stay with the caller.
//!
//! Every endpoint takes `Option<&mut O>` (see [`crate::hints`]): `Some` runs the hinted path,
//! `None::<&mut NoHints>` the plain one, with the same result. On RV32 the hints need
//! `bigint_ops`; without it the endpoints panic when given an oracle. `secp256k1_ecrecover`
//! needs `secp256k1-static-context`, and the allocating endpoints need `alloc`.

/// Records the hints of `$call`, replays the tape through a tape reader as the guest does, and
/// checks the same result and that exactly the whole tape is read; evaluates to the result
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

/// Counts the property-test calls that took hints, so a test cannot pass only on inputs that
/// are rejected before any query
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

    fn check(&self) {
        assert!(self.0.get() >= 8, "{} cases took hints", self.0.get());
    }
}

/// Property-test cases for the hint-heavy endpoints; fewer in debug builds, where each recorded
/// answer takes milliseconds
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

/// Bytes at a word-aligned address, so that big-endian conversions can work by words
#[repr(C, align(4))]
pub struct WordAligned<const N: usize>(pub [u8; N]);

/// The hinted path of an endpoint on RV32 without `bigint_ops`
#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
#[cold]
#[inline(never)]
fn hints_unavailable() -> ! {
    panic!("the hints need `bigint_ops` on RV32")
}
