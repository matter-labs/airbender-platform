//! The KZG proof check of the point evaluation precompile (EIP-4844).

use crate::ark_ec::pairing::Pairing;
use crate::ark_ec::{AffineRepr, CurveGroup};
use crate::ark_ff::{Field, PrimeField};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints as curve_hints;
use airbender_oracle::MemoryOracle;

#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
use super::unavailable as curve_hints;

/// A scalar of the KZG proof check (`z` or `y`), as an integer below the group order
pub type KzgScalar = <crate::bls12_381::Fr as PrimeField>::BigInt;

/// The errors of [`kzg_verify_proof`] (the caller checks the input size and versioned hash)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KzgError {
    /// The commitment or the proof is not the compressed encoding of a point of the subgroup
    InvalidPoint,
    /// `z` or `y` is not a canonical scalar (it is at least the group order)
    InvalidScalar,
}

/// The big-endian integer `input` as a scalar (`z` or `y`); fails unless it is below the group
/// order
// The caller maps the failure to its own error
#[allow(clippy::result_unit_err)]
pub fn parse_scalar(input: &[u8; 32]) -> Result<KzgScalar, ()> {
    let result = crate::parse_u256_be(input);
    if result >= crate::bls12_381::Fr::MODULUS {
        Err(())
    } else {
        Ok(result)
    }
}

/// The point of the compressed ZCash/arkworks encoding `input`, checked to be in the subgroup.
/// A longer slice is decoded from its first 48 bytes; a shorter one is an error.
// The callers map the failure to their own error
#[allow(clippy::result_unit_err)]
pub fn parse_g1_compressed(input: &[u8]) -> Result<crate::bls12_381::G1Affine, ()> {
    use crate::ark_serialize::CanonicalDeserialize;
    crate::bls12_381::G1Affine::deserialize_compressed(input).map_err(|_| ())
}

/// [`parse_g1_compressed`]; with an oracle, the square root is a checked hint and the subgroup
/// check an affine chain with checked hinted divisions
// The caller maps the failure to its own error
#[allow(clippy::result_unit_err)]
pub fn parse_g1_compressed_with_oracle<O: MemoryOracle>(
    input: &[u8],
    oracle: Option<&mut O>,
) -> Result<crate::bls12_381::G1Affine, ()> {
    match oracle {
        Some(oracle) => {
            // the two callbacks query the oracle one after the other
            let oracle = core::cell::RefCell::new(oracle);
            crate::bls12_381::g1_from_compressed_with_hints(
                input,
                |y_squared| curve_hints::bls12_381_fq_sqrt(*oracle.borrow_mut(), y_squared),
                |point| {
                    crate::bls12_381::g1::is_in_subgroup_with_divider(
                        point,
                        &mut curve_hints::Bls12381FqDivider::new(*oracle.borrow_mut()),
                    )
                },
            )
            .map_err(|_| ())
        }
        None => parse_g1_compressed(input),
    }
}

/// Whether `proof` shows that the polynomial of `commitment` takes the value `y` at `z`. The
/// points must be in the subgroup and the scalars below the group order (or it panics). With an
/// oracle, the curve arithmetic is affine with checked hinted divisions, and the final
/// exponentiation is replaced by a residue witness check or a checked hinted inverse.
pub fn verify_kzg_proof_with_oracle<O: MemoryOracle>(
    commitment: crate::bls12_381::G1Affine,
    proof: crate::bls12_381::G1Affine,
    z: KzgScalar,
    y: KzgScalar,
    mut oracle: Option<&mut O>,
) -> bool {
    use crate::bls12_381::curves::{g1, Bls12_381};
    // e(y G1 - commitment, G2) e(proof, tau G2 - z G2) == 1, with the z multiplication moved to
    // G1: e(y G1 - commitment - z proof, G2) e(proof, tau G2) == 1. `y G1 - z proof` is one joint
    // double-and-add.
    let scalar = |repr: &KzgScalar| {
        crate::bls12_381::Fr::from_bigint(*repr).expect("the scalar is below the group order")
    };
    let generator = crate::bls12_381::G1Affine::generator();
    let left_g1 = match oracle.as_deref_mut() {
        Some(oracle) => {
            let mut divider = curve_hints::Bls12381FqDivider::new(oracle);
            let sum = g1::mul_two_affine_with_divider(
                &generator,
                scalar(&y),
                &proof,
                -scalar(&z),
                &mut divider,
            );
            g1::add_affine_with_divider(&sum, &-commitment, &mut divider)
        }
        None => {
            let mut left_g1 = g1::mul_two(
                &generator.into_group(),
                scalar(&y),
                &proof.into_group(),
                -scalar(&z),
            );
            left_g1 -= &commitment;
            left_g1.into_affine()
        }
    };
    // both G2 points are fixed, so their normalized line coefficients are constants
    let g2 = [
        &crate::bls12_381::consts::PREPARED_G2_GENERATOR,
        &crate::bls12_381::consts::PREPARED_G2_BY_TAU,
    ];
    // the oracle reads the G1 points in place
    let g1 = [left_g1, proof];
    // both pairs in one Miller loop, as the prover computes it for its witness
    let pairs = || g1.iter().zip(g2);
    // the exact final exponentiation, of the loop output conjugated for the negative seed (the
    // prover's inverse is of the conjugate)
    let exact = |inverse: &dyn Fn(&crate::bls12_381::Fq12) -> Option<crate::bls12_381::Fq12>| {
        let mut miller_loop = Bls12_381::multi_miller_loop_shared(None, pairs());
        miller_loop.conjugate_in_place();
        let gt_el = Bls12_381::final_exponentiation_with_inverse(&miller_loop, inverse)
            .expect("the Miller loop output is invertible");
        gt_el == <Bls12_381 as Pairing>::TargetField::ONE
    };
    let Some(oracle) = oracle else {
        return exact(&Field::inverse);
    };
    // Residue witness check in place of the final exponentiation (eprint 2024/640, soundness in
    // `crate::residue_witness`). The prover cannot choose the result: a claimed identity must pass
    // the check or this panics, and a claimed non-identity takes the exact final exponentiation.
    match curve_hints::bls12_381_kzg_residue_witness(oracle, &g1) {
        curve_hints::PairingClaim::Identity { c: _, d, s } => {
            let l = Bls12_381::multi_miller_loop_shared(Some(&d), pairs());
            assert!(
                crate::residue_witness::bls12_381::check(&l, &d, &s),
                "the residue witness of the KZG proof claimed to be valid is wrong"
            );
            true
        }
        curve_hints::PairingClaim::NotIdentity { f_inverse } => {
            exact(&|f| curve_hints::checked_inverse(f, f_inverse))
        }
    }
}

/// The KZG proof check of EIP-4844 on the compressed commitment and proof and the big-endian
/// `z` and `y`. Parses in order, failing at the first invalid input: the commitment and the
/// proof (`InvalidPoint`), then `z` and `y` (`InvalidScalar`). With an oracle, decompression and
/// verification take checked hints (see [`verify_kzg_proof_with_oracle`]); a wrong hint panics.
pub fn kzg_verify_proof<O: MemoryOracle>(
    commitment: &[u8; 48],
    proof: &[u8; 48],
    z: &[u8; 32],
    y: &[u8; 32],
    mut oracle: Option<&mut O>,
) -> Result<bool, KzgError> {
    let Ok(commitment_point) = parse_g1_compressed_with_oracle(commitment, oracle.as_deref_mut())
    else {
        return Err(KzgError::InvalidPoint);
    };
    let Ok(proof) = parse_g1_compressed_with_oracle(proof, oracle.as_deref_mut()) else {
        return Err(KzgError::InvalidPoint);
    };

    let Ok(z) = parse_scalar(z) else {
        return Err(KzgError::InvalidScalar);
    };

    let Ok(y) = parse_scalar(y) else {
        return Err(KzgError::InvalidScalar);
    };

    Ok(verify_kzg_proof_with_oracle(
        commitment_point,
        proof,
        z,
        y,
        oracle,
    ))
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::bls12_381::{Fr, G1Affine};
    use crate::hints::test_support::{lying_oracle, non_identity_claiming_oracle};
    use crate::hints::{record_hints, FieldHintOp, NoHints, RecordingHintOracle};
    use crate::precompiles::{HintedCases, HINT_HEAVY_CASES};
    use hex_literal::hex;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};
    use std::vec;
    use std::vec::Vec;

    // ethereum/c-kzg-4844, tests/verify_kzg_proof/kzg-mainnet/verify_kzg_proof_case_correct_proof_4_4
    const COMMITMENT: [u8; 48] = hex!("8f59a8d2a1a625a17f3fea0fe5eb8c896db3764f3185481bc22f91b4aaffcca25f26936857bc3a7c2539ea8ec3a952b7");
    const Z: [u8; 32] = hex!("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000000");
    const Y: [u8; 32] = hex!("1522a4a7f34e1ea350ae07c29c96c7e79655aa926122e95fe69fcbd932ca49e9");
    const PROOF: [u8; 48] = hex!("a62ad71d14c5719385c0686f1871430475bf3a00f0aa3f7b8dd99a9abc2160744faf0070725e00b60ad9a026a15b1a8c");

    /// The scalar field modulus, which is not a canonical scalar (`Z` is the modulus minus one)
    const MODULUS: [u8; 32] =
        hex!("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001");

    /// A commitment, a proof, `z` and `y`
    type Case = ([u8; 48], [u8; 48], [u8; 32], [u8; 32]);

    /// ethereum/c-kzg-4844 a751e2d92abcd63b01d1400789789f5ea289a8b0,
    /// tests/verify_kzg_proof/kzg-mainnet/verify_kzg_proof_case_{correct_proof_2_3,
    /// correct_proof_3_2, incorrect_proof_2_3}/data.yaml, and whether the proof is valid
    const C_KZG_CASES: [(Case, bool); 3] = [
        ((hex!("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06"), hex!("a1fcd37a924af9ec04143b44853c26f6b0738f6e15a3e0755057e7d5460406c7e148adb0e2d608982140d0ae42fe0b3b"), hex!("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62"), hex!("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0")), true),
        ((hex!("b49d88afcd7f6c61a8ea69eff5f609d2432b47e7e4cd50b02cdddb4e0c1460517e8df02e4e64dc55e3d8ca192d57193a"), hex!("a38758fca85407078c0a7e5fd6d38b34340c809baa0e1fed9deaabb11aa503062acbbe23fcbe620a21b40a83bfa71b89"), hex!("0000000000000000000000000000000000000000000000000000000000000002"), hex!("6a75e4fe63e5e148c853462a680c3e3ccedea34719d28f19bf1b35ae4eea37d6")), true),
        ((hex!("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06"), hex!("b3477fc9a5bfab5fdb5523251818ee5a6d52613c59502a3d2df58217f4e366cd9ef37dee55bf2c705a2b08e7808b6fa0"), hex!("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62"), hex!("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0")), false),
    ];

    #[test]
    fn parse_scalar_bounds() {
        assert!(parse_scalar(&[0; 32]).is_ok());
        assert!(parse_scalar(&Z).is_ok());
        assert!(parse_scalar(&MODULUS).is_err());
    }

    /// Compressed encodings, decoded without an oracle, with `None` and with honest hints
    #[test]
    fn parse_g1_compressed_cases() {
        let generator = hex!("97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb");
        // an x above the field modulus
        let large_x = hex!("9a0d51cc7fa052e0c99d3ea24278105bf01c29943da18ef2505173378a64a261054348443115665b5e964e9b4a3c7c59");
        let mut cases: Vec<(Vec<u8>, bool)> = vec![
            (generator.to_vec(), true),
            (COMMITMENT.to_vec(), true),
            (PROOF.to_vec(), true),
            (vec![0; 47], false),
            (vec![0; 49], false),
            (vec![0xff; 48], false),
            (large_x.to_vec(), false),
        ];
        // the first byte holds the flags: 0x80 compressed, 0x40 infinity, 0x20 the sign of y
        for (flags, last, valid) in [
            (0xc0, 0, true),
            (0xc0, 1, false), // infinity with a nonzero coordinate
            (0x80, 0, false), // x = 0: a point of order 3
            (0x80, 2, false), // x = 2: no point of the curve
            (0x00, 0, false),
            (0x20, 0, false),
            (0x40, 0, false),
            (0x60, 0, false),
            (0xe0, 0, false),
            (0xf0, 0, false),
        ] {
            let mut bytes = vec![0u8; 48];
            (bytes[0], bytes[47]) = (flags, last);
            cases.push((bytes, valid));
        }
        for (input, valid) in cases {
            let result = parse_g1_compressed(&input);
            assert_eq!(result.is_ok(), valid, "{input:02x?}");
            let hinted =
                parse_g1_compressed_with_oracle(&input, Some(&mut RecordingHintOracle::new()));
            assert_eq!(hinted, result, "{input:02x?}");
            assert_eq!(
                parse_g1_compressed_with_oracle::<NoHints>(&input, None),
                result
            );
        }
    }

    /// The endpoint without hints, with honest ones, and, for valid proofs, with an oracle that
    /// claims every product not to be the identity, against the crate's full KZG verifier: for
    /// the c-kzg vectors, other scalars, and proofs swapped or at infinity
    #[test]
    fn kzg_matches_verify_kzg_proof() {
        let mut infinity = [0u8; 48];
        infinity[0] = 0xc0;
        let z_128_bits = hex!("000000000000000000000000000000000123456789abcdef0123456789abcdef");
        let mut cases: Vec<(Case, Option<bool>)> = C_KZG_CASES
            .iter()
            .map(|&(case, valid)| (case, Some(valid)))
            .collect();
        cases.extend([
            ((COMMITMENT, PROOF, Z, Y), Some(true)),
            ((COMMITMENT, PROOF, z_128_bits, Y), None),
            ((COMMITMENT, PROOF, Z, [0; 32]), None),
            ((COMMITMENT, PROOF, [0; 32], Y), None),
            ((COMMITMENT, C_KZG_CASES[0].0 .1, Z, Y), None),
            ((COMMITMENT, infinity, Z, Y), None),
            ((infinity, PROOF, Z, Y), None),
            ((infinity, infinity, Z, [0; 32]), None),
        ]);
        for (i, ((commitment, proof, z, y), valid)) in cases.into_iter().enumerate() {
            let expected = crate::bls12_381::verify_kzg_proof(
                parse_g1_compressed(&commitment).unwrap(),
                parse_g1_compressed(&proof).unwrap(),
                parse_scalar(&z).unwrap(),
                parse_scalar(&y).unwrap(),
            );
            if let Some(valid) = valid {
                assert_eq!(expected, valid, "case {i}");
            }
            let plain = kzg_verify_proof::<NoHints>(&commitment, &proof, &z, &y, None);
            assert_eq!(plain, Ok(expected), "case {i}");
            let (hinted, _) =
                record_hints(|oracle| kzg_verify_proof(&commitment, &proof, &z, &y, Some(oracle)));
            assert_eq!(hinted, Ok(expected), "case {i}");
            if expected {
                let mut claiming = non_identity_claiming_oracle();
                let claimed = kzg_verify_proof(&commitment, &proof, &z, &y, Some(&mut claiming));
                assert_eq!(claimed, Ok(expected), "case {i}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "claimed to be valid is wrong")]
    fn wrong_identity_witness_panics() {
        let mut lying = lying_oracle(&[FieldHintOp::Bls12381KzgResidueWitness]);
        let _ = kzg_verify_proof(&COMMITMENT, &PROOF, &Z, &Y, Some(&mut lying));
    }

    /// The c-kzg proofs take a nontrivial residue witness: neither point of the product is at
    /// infinity, and the last answer is the identity flag, `d` and `s` (1 + 216 words) for a valid
    /// proof, or the flag and `f^-1` (1 + 144 words) for an invalid one
    #[test]
    fn c_kzg_vectors_take_the_nontrivial_branches() {
        use crate::ark_ec::{AffineRepr, CurveGroup};
        for ((commitment, proof, z, y), valid) in C_KZG_CASES {
            let c = parse_g1_compressed(&commitment).unwrap();
            let p = parse_g1_compressed(&proof).unwrap();
            let zs = Fr::from_bigint(parse_scalar(&z).unwrap()).unwrap();
            let ys = Fr::from_bigint(parse_scalar(&y).unwrap()).unwrap();
            let left = (G1Affine::generator() * ys - c - p * zs).into_affine();
            assert!(!left.is_zero() && !p.is_zero());
            let (result, tape) =
                record_hints(|oracle| kzg_verify_proof(&commitment, &proof, &z, &y, Some(oracle)));
            assert_eq!(result, Ok(valid));
            let answer_len = if valid { 1 + 216 } else { 1 + 144 };
            assert_eq!(tape[tape.len() - answer_len], u32::from(valid));
        }
    }

    /// The first invalid input gives the error, after the hints of the points before it
    #[test]
    fn inputs_are_parsed_in_order() {
        let invalid_point = [0u8; 48];
        let parse_tape = |point: &[u8]| {
            record_hints(|oracle| parse_g1_compressed_with_oracle(point, Some(oracle))).1
        };
        let call = |commitment: &[u8; 48], proof: &[u8; 48], z: &[u8; 32], y: &[u8; 32]| {
            let plain = kzg_verify_proof::<NoHints>(commitment, proof, z, y, None);
            let (hinted, tape) =
                record_hints(|oracle| kzg_verify_proof(commitment, proof, z, y, Some(oracle)));
            assert_eq!(hinted, plain);
            (hinted, tape)
        };
        // an invalid commitment comes first, with no hints asked
        let (result, tape) = call(&invalid_point, &invalid_point, &MODULUS, &MODULUS);
        assert_eq!((result, tape.len()), (Err(KzgError::InvalidPoint), 0));
        // then the proof, after the hints of the commitment
        let (result, tape) = call(&COMMITMENT, &invalid_point, &MODULUS, &MODULUS);
        assert_eq!(result, Err(KzgError::InvalidPoint));
        assert_eq!(tape, parse_tape(&COMMITMENT));
        // then `z`, then `y`, after the hints of both points, and before any other
        let points = [parse_tape(&COMMITMENT), parse_tape(&PROOF)].concat();
        assert!(!points.is_empty());
        for (z, y) in [(MODULUS, MODULUS), (Z, MODULUS)] {
            let (result, tape) = call(&COMMITMENT, &PROOF, &z, &y);
            assert_eq!(result, Err(KzgError::InvalidScalar));
            assert_eq!(tape, points);
        }
    }

    /// The compressed encoding of `k` times the generator
    fn compressed(k: u64) -> [u8; 48] {
        use crate::ark_ec::{AffineRepr, CurveGroup};
        use crate::ark_serialize::CanonicalSerialize;
        let point = (G1Affine::generator() * Fr::from(k)).into_affine();
        let mut bytes = [0u8; 48];
        point.serialize_compressed(&mut bytes[..]).unwrap();
        bytes
    }

    /// The c-kzg proofs, random points and scalars, and encodings that are mostly not points or
    /// not canonical scalars
    #[test]
    fn advice_matches_no_advice() {
        let hinted_cases = HintedCases::default();
        proptest!(ProptestConfig::with_cases(HINT_HEAVY_CASES), |(case in 0usize..3, mode in 0u8..8, ks: [u64; 2], scalars: [[u8; 32]; 2], flip: [u8; 3])| {
            let ((mut commitment, mut proof, mut z, mut y), _) = C_KZG_CASES[case];
            match mode {
                0 | 1 => {}
                2 => (commitment, proof) = (compressed(ks[0]), compressed(ks[1])),
                3 => (z, y) = (scalars[0], scalars[1]),
                // mostly not points, or not canonical
                4 => commitment[usize::from(flip[0]) % 48] ^= flip[1] | 1,
                5 => proof[usize::from(flip[0]) % 48] ^= flip[1] | 1,
                6 => z[usize::from(flip[0]) % 32] ^= flip[1] | 1,
                _ => y[0] ^= flip[2] | 0x80,
            }
            let hinted = hinted_cases.record(|oracle| kzg_verify_proof(&commitment, &proof, &z, &y, Some(oracle)));
            prop_assert_eq!(hinted, kzg_verify_proof(&commitment, &proof, &z, &y, None::<&mut NoHints>));
        });
        hinted_cases.check();
    }

    #[test]
    fn recorded_tape_replays() {
        for ((commitment, proof, z, y), valid) in C_KZG_CASES {
            let result = assert_tape_replays!(|oracle| kzg_verify_proof(
                &commitment,
                &proof,
                &z,
                &y,
                Some(oracle)
            ));
            assert_eq!(result, Ok(valid));
        }
        let ((commitment, _, z, y), _) = C_KZG_CASES[0];
        let result = assert_tape_replays!(|oracle| kzg_verify_proof(
            &commitment,
            &[0; 48],
            &z,
            &y,
            Some(oracle)
        ));
        assert_eq!(result, Err(KzgError::InvalidPoint));
    }
}
