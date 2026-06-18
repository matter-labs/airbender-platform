pub mod consts;
pub mod curves;
pub mod eip2537;
pub mod fields;

pub use self::curves::{g1, g2, G1Affine, G1Projective, G2Affine, G2Projective};
pub use self::fields::{Fq, Fq12, Fq2, Fq6, Fr};

pub(crate) use self::curves::util;

use crate::ark_ec::pairing::Pairing;
use crate::ark_ec::{AffineRepr, CurveGroup};
use crate::ark_ff::{AdditiveGroup, Field, PrimeField};
use consts::{PREPARED_G2_BY_TAU, PREPARED_G2_GENERATOR};

#[inline(always)]
pub fn verify_kzg_proof(
    commitment: G1Affine,
    proof: G1Affine,
    z: <Fr as PrimeField>::BigInt,
    y: <Fr as PrimeField>::BigInt,
) -> bool {
    // Original check:
    //   e(y·G1 - commitment, G2) * e(proof, τ·G2 - z·G2) == 1.
    //
    // Move the z multiplication from G2 to G1 (bilinearity), which avoids a
    // G2 scalar mul (G2 lives in Fq², so it is materially more expensive than
    // G1), a G2 subtraction, and a G2 into_affine:
    //   e(y·G1 - commitment - z·proof, G2) * e(proof, τ·G2) == 1.
    //
    // The two G1 scalar mults (y·G1 and (-z)·proof) are then fused into a
    // single 2-base interleaved double-and-add: one shared 255-step doubling
    // loop instead of two. We avoid arkworks' VariableBaseMSM here because it
    // allocates internally, which breaks the proving binary's allocator setup.
    //
    // τ·G2 is the fixed trusted-setup point; its Miller-loop coefficients are
    // precomputed once into the PREPARED_G2_BY_TAU const rather than recomputed
    // on every call.
    let neg_z = (-Fr::from_bigint(z)
        .expect("z is canonical: the caller validates it via parse_scalar / Fr::into_bigint"))
    .into_bigint();

    let bases = [G1Affine::generator(), proof];
    let scalars = [y, neg_z];

    const NUM_BITS: usize = 256;
    let mut left_g1 = G1Projective::ZERO;
    for bit in (0..NUM_BITS).rev() {
        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        for (base, scalar) in bases.iter().zip(scalars.iter()) {
            if scalar.0[word_idx] & (1u64 << bit_idx) > 0 {
                left_g1 += base;
            }
        }
        if bit > 0 {
            left_g1.double_in_place();
        }
    }
    left_g1 -= &commitment;

    let left_g1 = left_g1.into_affine();

    let gt_el = curves::Bls12_381::multi_pairing(
        [left_g1, proof],
        [PREPARED_G2_GENERATOR.clone(), PREPARED_G2_BY_TAU.clone()],
    );
    gt_el.0 == <curves::Bls12_381 as Pairing>::TargetField::ONE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark_serialize::CanonicalDeserialize;

    fn from_hex<const N: usize>(s: &str) -> [u8; N] {
        let b = s.as_bytes();
        assert_eq!(b.len(), 2 * N);
        let mut out = [0u8; N];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(core::str::from_utf8(&b[2 * i..2 * i + 2]).unwrap(), 16)
                .unwrap();
        }
        out
    }

    fn parse_g1_compressed(input: &[u8; 48]) -> G1Affine {
        G1Affine::deserialize_compressed(&input[..]).unwrap()
    }

    fn parse_scalar(input: &[u8; 32]) -> <Fr as PrimeField>::BigInt {
        let r = crate::parse_u256_be(input);
        assert!(r < Fr::MODULUS, "test scalar must be canonical");
        r
    }

    #[test]
    fn verify_kzg_proof_known_vector() {
        // Known-valid EIP-4844 point-evaluation vector for this trusted setup.
        let commitment = parse_g1_compressed(&from_hex(
            "8f59a8d2a1a625a17f3fea0fe5eb8c896db3764f3185481bc22f91b4aaffcca25f26936857bc3a7c2539ea8ec3a952b7",
        ));
        let proof = parse_g1_compressed(&from_hex(
            "a62ad71d14c5719385c0686f1871430475bf3a00f0aa3f7b8dd99a9abc2160744faf0070725e00b60ad9a026a15b1a8c",
        ));
        let z = parse_scalar(&from_hex(
            "73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000000",
        ));
        let y = parse_scalar(&from_hex(
            "1522a4a7f34e1ea350ae07c29c96c7e79655aa926122e95fe69fcbd932ca49e9",
        ));

        // The valid (commitment, proof, z, y) tuple must verify.
        assert!(verify_kzg_proof(commitment, proof, z, y));

        // An unrelated evaluation point z must be rejected.
        let unrelated_z = parse_scalar(&from_hex(
            "000000000000000000000000000000000123456789abcdef0123456789abcdef",
        ));
        assert!(!verify_kzg_proof(commitment, proof, unrelated_z, y));

        // A zero claimed value y must be rejected.
        let zero_y = parse_scalar(&[0u8; 32]);
        assert!(!verify_kzg_proof(commitment, proof, z, zero_y));
    }

    /// The precomputed PREPARED_G2_BY_TAU const must equal the runtime
    /// G2Prepared derived from G2_BY_TAU_POINT. Catches a stale literal if
    /// G2_BY_TAU_POINT or the Miller-loop precomputation ever changes. Under
    /// `cfg(test)` this exercises the delegation (proving) variant of the const.
    #[test]
    fn prepared_g2_by_tau_const_matches_runtime() {
        let runtime: curves::G2PreparedNoAlloc = consts::G2_BY_TAU_POINT.into();
        assert_eq!(runtime, consts::PREPARED_G2_BY_TAU);
    }
}
