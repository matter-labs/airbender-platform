//! The crypto half of the EVM precompiles. Gas, input padding and length checks, error mapping
//! and output encoding stay with the caller.
//!
//! Every endpoint takes `Option<&mut O>` (see [`crate::hints`]): `Some` runs the hinted path,
//! `None::<&mut NoHints>` the plain one, with the same result. The hinted paths need
//! `hinted_operations`; without it the endpoints panic when given an oracle.
//! `secp256k1_ecrecover` needs `secp256k1-static-context`.
//!
//! Nothing allocates. The pairing checks and the MSMs parse their pairs into stack buffers
//! (`crate::scratch`) whose capacity is the most pairs a transaction can pay for under the gas
//! cap of EIP-7825 ([`MAX_TX_GAS`]); more pairs fail before anything is parsed.

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

/// The execution gas a transaction can spend (EIP-7825, Fusaka): as every precompile prices its
/// pairs, it bounds their number.
pub const MAX_TX_GAS: u64 = 1 << 24;

/// The most pairs of a bn254 pairing check (EIP-1108: 45000 gas, plus 34000 per pair)
pub const MAX_BN254_PAIRS: usize = ((MAX_TX_GAS - 45_000) / 34_000) as usize;

/// The most pairs of a BLS12-381 pairing check (EIP-2537: 37700 gas, plus 32600 per pair)
pub const MAX_BLS12_381_PAIRS: usize = ((MAX_TX_GAS - 37_700) / 32_600) as usize;

/// The most pairs of a BLS12-381 G1 MSM (EIP-2537: 12000 gas per pair at the largest discount,
/// 519/1000, from 128 pairs on)
pub const MAX_BLS12_381_G1_MSM_PAIRS: usize = (MAX_TX_GAS / (12_000 * 519 / 1000)) as usize;

/// The most pairs of a BLS12-381 G2 MSM (EIP-2537: 22500 gas per pair at the largest discount,
/// 524/1000, from 128 pairs on)
pub const MAX_BLS12_381_G2_MSM_PAIRS: usize = (MAX_TX_GAS / (22_500 * 524 / 1000)) as usize;
mod kzg;
#[cfg(not(any(feature = "hinted_operations", test)))]
mod unavailable;

pub use self::bls12_381::{
    bls12_381_g1_add, bls12_381_g2_add, bls12_381_map_fp2_to_g2, bls12_381_map_fp_to_g1,
    Bls12Error, FIELD_ELEMENT_SERIALIZATION_LEN, FIELD_EXT_ELEMENT_SERIALIZATION_LEN,
    G1_SERIALIZATION_LEN, G2_SERIALIZATION_LEN,
};
pub use self::bls12_381::{
    bls12_381_g1_msm, bls12_381_g2_msm, bls12_381_pairing_check, BLS12_381_PAIR_LEN,
    G1_MSM_PAIR_LEN, G2_MSM_PAIR_LEN,
};
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

/// The hinted path of an endpoint without `hinted_operations`
#[cfg(not(any(feature = "hinted_operations", test)))]
#[cold]
#[inline(never)]
fn hints_unavailable() -> ! {
    panic!("the hints need the `hinted_operations` feature")
}

#[cfg(test)]
mod bounds {
    use super::*;

    #[test]
    fn the_pair_bounds_follow_the_gas_cap() {
        assert_eq!(MAX_TX_GAS, 16_777_216);
        assert_eq!(MAX_BN254_PAIRS, 492);
        assert_eq!(MAX_BLS12_381_PAIRS, 513);
        assert_eq!(MAX_BLS12_381_G1_MSM_PAIRS, 2693);
        assert_eq!(MAX_BLS12_381_G2_MSM_PAIRS, 1423);
    }
}
