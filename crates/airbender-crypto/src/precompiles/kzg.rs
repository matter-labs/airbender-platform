// Ported from zksync-os e855bca2:basic_system/src/system_functions/point_evaluation.rs
//! The KZG proof check of the point evaluation precompile (EIP-4844), from the compressed
//! encodings of the points and the big-endian scalars.

use crate::ark_ec::pairing::Pairing;
use crate::ark_ec::{AffineRepr, CurveGroup};
use crate::ark_ff::{Field, PrimeField};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints as curve_hints;
use crate::hints::HintOracle;

#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
use super::unavailable as curve_hints;

/// A scalar of the KZG proof check (`z` or `y`), as an integer below the group order
pub type KzgScalar = <crate::bls12_381::Fr as PrimeField>::BigInt;

/// The errors of [`kzg_verify_proof`]: zksync-os's `PointEvaluationInterfaceError`, but for
/// the size of the input, the versioned hash and the pairing mismatch, which the wrapper checks
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KzgError {
    /// The commitment or the proof is not the compressed encoding of a point of the subgroup
    InvalidPoint,
    /// `z` or `y` is not a canonical scalar (it is at least the group order)
    InvalidScalar,
}

/// The big-endian integer `input` as a scalar of the KZG proof check (`z` or `y`); fails unless
/// it is below the group order
// We do not need internal representation, just canonical scalar
// zksync-os's signature (its point evaluation wrapper parses with it): the caller maps the
// failure to its error
#[allow(clippy::result_unit_err)]
pub fn parse_scalar(input: &[u8; 32]) -> Result<KzgScalar, ()> {
    // Arkworks has strange format for integer serialization, so we do manually
    let result = crate::parse_u256_be(input);
    if result >= crate::bls12_381::Fr::MODULUS {
        Err(())
    } else {
        Ok(result)
    }
}

/// The point of the compressed encoding `input` (the ZCash/arkworks format), validated by
/// `CanonicalDeserialize`. `input` must be exactly 48 bytes: a longer slice is decoded from its
/// first 48 bytes (the rest is ignored), a shorter one is an error.
// zksync-os's signature (its bootloader imports it): the callers map any failure to their error
#[allow(clippy::result_unit_err)]
pub fn parse_g1_compressed(input: &[u8]) -> Result<crate::bls12_381::G1Affine, ()> {
    // format coincides with one defined in ZCash/Arkworks
    use crate::ark_serialize::CanonicalDeserialize;
    crate::bls12_381::G1Affine::deserialize_compressed(input).map_err(|_| ())
}

/// `parse_g1_compressed` with the square root of the decompression from a checked oracle
/// hint, and the subgroup membership test in affine coordinates with hinted divisions, when an
/// oracle is given
// zksync-os's signature (its point evaluation wrapper parses with it): the caller maps the
// failure to its error
#[allow(clippy::result_unit_err)]
pub fn parse_g1_compressed_with_oracle<O: HintOracle>(
    input: &[u8],
    oracle: Option<&mut O>,
) -> Result<crate::bls12_381::G1Affine, ()> {
    match oracle {
        Some(oracle) => {
            // the two steps query the oracle one after the other
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

/// Whether `proof` shows that the polynomial of `commitment` takes the value `y` at `z`: the
/// pairing check of [`kzg_verify_proof`] on the decoded points (which must be in the subgroup)
/// and the scalars (which must be below the group order, or it panics). zksync-os's
/// `verify_kzg_proof`, which its bootloader takes for the blob commitments, is this without an
/// oracle.
///
/// With an oracle, the curve arithmetic works in affine coordinates with its field divisions
/// from checked hints, and the inversion of the final exponentiation comes from one too.
pub fn verify_kzg_proof_with_oracle<O: HintOracle>(
    commitment: crate::bls12_381::G1Affine,
    proof: crate::bls12_381::G1Affine,
    z: KzgScalar,
    y: KzgScalar,
    mut oracle: Option<&mut O>,
) -> bool {
    use crate::bls12_381::curves::{g1, Bls12_381};
    // Original check:
    // e(yG1 - commitment, G2) * e(proof, tauG2 - zG2) == 1.
    //
    // Move the z multiplication from G2 to G1:
    // e(yG1 - commitment - z*proof, G2) * e(proof, tauG2) == 1.
    // y G - z proof as one joint double-and-add (the doublings are shared), then minus the
    // commitment; the scalars were parsed below the group order
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
    // both G2 points are fixed, so their Miller-loop line coefficients are constants
    // (normalized to a first coefficient of one), read in place
    let g2 = [
        &crate::bls12_381::consts::PREPARED_G2_GENERATOR,
        &crate::bls12_381::consts::PREPARED_G2_BY_TAU,
    ];
    // the G1 points of the product, which the oracle reads where they are
    let g1 = [left_g1, proof];
    // the Miller loop over both pairs at once, as the prover computes it for its witness
    let pairs = || g1.iter().zip(g2);
    // the exact final exponentiation of the loop output, conjugated for the negative seed as
    // `multi_miller_loop` conjugates it (the prover's inverse is of the conjugate)
    let exact = |inverse: &dyn Fn(&crate::bls12_381::Fq12) -> Option<crate::bls12_381::Fq12>| {
        let mut miller_loop = Bls12_381::multi_miller_loop_shared(None, pairs());
        miller_loop.conjugate_in_place();
        // the Miller loop of curve points never evaluates to zero
        let gt_el = Bls12_381::final_exponentiation_with_inverse(&miller_loop, inverse)
            .expect("the Miller loop output is invertible");
        gt_el == <Bls12_381 as Pairing>::TargetField::ONE
    };
    let Some(oracle) = oracle else {
        return exact(&Field::inverse);
    };
    // The residue witness check in place of the final exponentiation (Novakovic, Eagen, "On
    // Proving Pairings", https://eprint.iacr.org/2024/640; soundness in the documentation of
    // `crate::residue_witness`): the Miller loop started at `d` gives `d^|u| f`, the rest is
    // one Frobenius map. The prover claims the outcome; it cannot choose it: a claimed
    // identity must come with a witness that passes the check (a failed check is a broken
    // prover and panics), and a claimed non-identity is settled by the exact final
    // exponentiation, which finds an identity all the same.
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

/// The KZG proof check of the point evaluation precompile (EIP-4844), from the commitment and
/// the proof compressed (the ZCash encoding, 48 bytes each) and the big-endian scalars `z` and
/// `y`: whether the proof shows that the polynomial of the commitment takes the value `y` at
/// `z`. Fails with `InvalidPoint` unless the commitment, then the proof, decode to points of
/// the subgroup, then with `InvalidScalar` unless `z`, then `y`, are canonical, in that order.
/// The wrapper keeps the gas, the size of the input, the versioned hash of the commitment, the
/// layout of the input, and the response (or its pairing mismatch error for `Ok(false)`).
/// zksync-os's wrapper does the same steps itself, with [`parse_g1_compressed_with_oracle`],
/// [`parse_scalar`] and [`verify_kzg_proof_with_oracle`]; the C bindings call this.
///
/// With an oracle, the square roots of the decompressions and the divisions of the subgroup
/// checks and of the curve arithmetic (in affine coordinates) are checked hints, and the final
/// exponentiation is replaced by a residue witness check, or, for a proof the prover claims to
/// be invalid, takes a checked hinted inverse. A wrong hint panics.
pub fn kzg_verify_proof<O: HintOracle>(
    commitment: &[u8; 48],
    proof: &[u8; 48],
    z: &[u8; 32],
    y: &[u8; 32],
    mut oracle: Option<&mut O>,
) -> Result<bool, KzgError> {
    // Parse the commitment and proof
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
    use crate::hints::{
        read_field_hint_request, record_hints, FieldHintOp, HintError, NativeQuerierMemory,
        NoHints, RecordingHintOracle,
    };
    use crate::precompiles::{HintedCases, HINT_HEAVY_CASES};
    use hex_literal::hex;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};
    use std::vec::Vec;

    // from zksync-os e855bca2:basic_system/src/system_functions/point_evaluation.rs

    /// zksync-os's `VERSIONED_HASH_VERSION_KZG`
    const VERSIONED_HASH_VERSION_KZG: u8 = 1;

    /// zksync-os's `POINT_EVAL_PRECOMPILE_SUCCESS_RESPONSE`: u256_be(4096) || u256_be(BLS12-381 Fr
    /// characteristic)
    const SUCCESS_RESPONSE: [u8; 64] = hex!(
        "0000000000000000000000000000000000000000000000000000000000001000"
        "73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001"
    );

    /// zksync-os's `PointEvaluationInterfaceError`
    #[derive(Debug, PartialEq, Eq)]
    enum PointEvaluationError {
        InvalidInputSize,
        InvalidVersionedHash,
        InvalidPoint,
        InvalidScalar,
        PairingMismatch,
    }

    fn versioned_hash_for_kzg(data: &[u8]) -> [u8; 32] {
        use crate::sha256::Digest;
        let mut hash: [u8; 32] = crate::sha256::Sha256::digest(data).into();
        hash[0] = VERSIONED_HASH_VERSION_KZG;

        hash
    }

    /// The wrapper's EVM half (zksync-os `point_evaluation.rs:249-303`, without the gas): the
    /// size of the input, its versioned hash, its layout, and the response
    fn point_evaluation<O: HintOracle>(
        input: &[u8],
        oracle: Option<&mut O>,
    ) -> Result<[u8; 64], PointEvaluationError> {
        if input.len() != 192 {
            return Err(PointEvaluationError::InvalidInputSize);
        }
        let versioned_hash = &input[..32];
        let commitment = &input[96..144];
        if versioned_hash_for_kzg(commitment) != versioned_hash {
            return Err(PointEvaluationError::InvalidVersionedHash);
        }
        match kzg_verify_proof(
            commitment.try_into().unwrap(),
            input[144..192].try_into().unwrap(),
            input[32..64].try_into().unwrap(),
            input[64..96].try_into().unwrap(),
            oracle,
        ) {
            Ok(true) => Ok(SUCCESS_RESPONSE),
            Ok(false) => Err(PointEvaluationError::PairingMismatch),
            Err(KzgError::InvalidPoint) => Err(PointEvaluationError::InvalidPoint),
            Err(KzgError::InvalidScalar) => Err(PointEvaluationError::InvalidScalar),
        }
    }

    /// zksync-os's `execute`: runs the precompile without and with the oracle hints, and with an
    /// oracle that claims an invalid proof (with a correct inverse); all must agree
    fn execute(input: &[u8]) -> Result<[u8; 64], PointEvaluationError> {
        let hinted = point_evaluation(input, Some(&mut RecordingHintOracle::new()));
        let result = point_evaluation::<NoHints>(input, None);
        assert_eq!(result, hinted);
        // a prover claiming an invalid proof (with a correct inverse) does not change the answer
        let claimed = point_evaluation(input, Some(&mut non_identity_claiming_oracle()));
        assert_eq!(result, claimed);
        result
    }

    /// `versioned_hash || z || y || commitment || proof`
    fn input(commitment: &[u8], z: &[u8], y: &[u8], proof: &[u8]) -> Vec<u8> {
        let versioned_hash = versioned_hash_for_kzg(commitment);
        [&versioned_hash[..], z, y, commitment, proof].concat()
    }

    // Test data from: https://github.com/ethereum/c-kzg-4844/blob/main/tests/verify_kzg_proof/kzg-mainnet/verify_kzg_proof_case_correct_proof_4_4/data.yaml
    const COMMITMENT: [u8; 48] = hex!("8f59a8d2a1a625a17f3fea0fe5eb8c896db3764f3185481bc22f91b4aaffcca25f26936857bc3a7c2539ea8ec3a952b7");
    const Z: [u8; 32] = hex!("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000000");
    const Y: [u8; 32] = hex!("1522a4a7f34e1ea350ae07c29c96c7e79655aa926122e95fe69fcbd932ca49e9");
    const PROOF: [u8; 48] = hex!("a62ad71d14c5719385c0686f1871430475bf3a00f0aa3f7b8dd99a9abc2160744faf0070725e00b60ad9a026a15b1a8c");

    /// The scalar field modulus, which is not a canonical scalar
    const MODULUS: [u8; 32] = [
        0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8,
        0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00,
        0x00, 0x01,
    ];

    #[test]
    fn basic_test() {
        let input = input(&COMMITMENT, &Z, &Y, &PROOF);

        let expected_output = hex!("000000000000000000000000000000000000000000000000000000000000100073eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001");

        let result = execute(&input);
        assert!(result.is_ok(), "Result: {:?}", result);

        assert_eq!(result.unwrap(), expected_output);
    }

    #[test]
    fn test_rearranged_kzg_verifier_matches_reference() {
        let commitment = parse_g1_compressed(&COMMITMENT).unwrap();
        let proof = parse_g1_compressed(&PROOF).unwrap();
        let z = parse_scalar(&Z).unwrap();
        let y = parse_scalar(&Y).unwrap();
        // zksync-os's `verify_kzg_proof`
        let verify_kzg_proof = |commitment, proof, z, y| {
            verify_kzg_proof_with_oracle::<NoHints>(commitment, proof, z, y, None)
        };

        let local = verify_kzg_proof(commitment, proof, z, y);
        let reference = crate::bls12_381::verify_kzg_proof(commitment, proof, z, y);
        assert!(local, "valid proof should verify");
        assert_eq!(local, reference);

        let unrelated_128_bit_z =
            hex!("000000000000000000000000000000000123456789abcdef0123456789abcdef");
        let unrelated_128_bit_z = parse_scalar(&unrelated_128_bit_z).unwrap();
        assert_eq!(
            verify_kzg_proof(commitment, proof, unrelated_128_bit_z, y),
            crate::bls12_381::verify_kzg_proof(commitment, proof, unrelated_128_bit_z, y)
        );

        let zero_y = parse_scalar(&[0u8; 32]).unwrap();
        assert_eq!(
            verify_kzg_proof(commitment, proof, z, zero_y),
            crate::bls12_381::verify_kzg_proof(commitment, proof, z, zero_y)
        );
    }

    #[test]
    #[should_panic(expected = "claimed to be valid is wrong")]
    fn wrong_identity_witness_panics() {
        let input = input(&COMMITMENT, &Z, &Y, &PROOF);
        let mut lying = lying_oracle(&[FieldHintOp::Bls12381KzgResidueWitness]);
        let _ = point_evaluation(&input, Some(&mut lying));
    }

    #[test]
    fn test_invalid_input() {
        let commitment = hex!("c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let z = hex!("0000000000000000000000000000000000000000000000000000000000000000");
        let y = hex!("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001");
        let proof = hex!("c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

        let input = input(&commitment, &z, &y, &proof);

        let result = execute(&input);
        assert!(result.is_err(), "Result: {:?}", result);
    }

    /// Test invalid scalar - z >= field modulus
    #[test]
    fn test_point_evaluation_invalid_scalar_z() {
        let input = input(&COMMITMENT, &MODULUS, &Y, &PROOF);

        let result = execute(&input);

        assert_eq!(result, Err(PointEvaluationError::InvalidScalar));
    }

    /// Test invalid scalar - y >= field modulus
    #[test]
    fn test_point_evaluation_invalid_scalar_y() {
        let input = input(&COMMITMENT, &Z, &MODULUS, &PROOF);

        let result = execute(&input);

        assert_eq!(result, Err(PointEvaluationError::InvalidScalar));
    }

    /// Test scalar parsing edge cases
    #[test]
    fn test_parse_scalar_edge_cases() {
        // Test maximum valid scalar (modulus - 1)
        let max_valid = [
            0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1,
            0xd8, 0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00,
        ];
        assert!(parse_scalar(&max_valid).is_ok());

        // Test minimum invalid scalar (modulus)
        let min_invalid = [
            0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1,
            0xd8, 0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x01,
        ];
        assert!(parse_scalar(&min_invalid).is_err());

        // Test zero (always valid)
        let zero = [0u8; 32];
        assert!(parse_scalar(&zero).is_ok());
    }

    /// `parse_g1_compressed`, which is the decoding without an oracle; also checks that the
    /// decoding with the hints of an honest oracle agrees
    fn parse(input: &[u8]) -> Result<G1Affine, ()> {
        let result = parse_g1_compressed(input);
        let hinted = parse_g1_compressed_with_oracle(input, Some(&mut RecordingHintOracle::new()));
        assert_eq!(result, hinted, "{input:02x?}");
        assert_eq!(
            result,
            parse_g1_compressed_with_oracle::<NoHints>(input, None)
        );
        result
    }

    /// Test parse_g1_compressed edge cases
    #[test]
    fn test_parse_g1_compressed_edge_cases() {
        // Test valid identity element (point at infinity)
        let identity = [
            0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(parse(&identity).is_ok(), "Identity point should be valid");

        // Test wrong input size - too short
        let too_short = [0u8; 47]; // One byte short
        assert!(parse(&too_short).is_err(), "Input too short should fail");

        // Test wrong input size - too long
        let too_long = [0u8; 49]; // One byte too long
        assert!(parse(&too_long).is_err(), "Input too long should fail");

        // Test all zeros (not a valid compressed point)
        let all_zeros = [0u8; 48];
        assert!(parse(&all_zeros).is_err(), "All zeros should be invalid");

        // Test all ones (invalid field element)
        let all_ones = [0xffu8; 48];
        assert!(parse(&all_ones).is_err(), "All ones should be invalid");

        // Test invalid compression flag (neither compressed nor uncompressed)
        let invalid_flag = [
            0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(
            parse(&invalid_flag).is_err(),
            "Invalid compression flag should fail"
        );

        // Test x-coordinate >= field modulus (invalid field element)
        let invalid_x = [
            0x9a, 0x0d, 0x51, 0xcc, 0x7f, 0xa0, 0x52, 0xe0, 0xc9, 0x9d, 0x3e, 0xa2, 0x42, 0x78,
            0x10, 0x5b, 0xf0, 0x1c, 0x29, 0x94, 0x3d, 0xa1, 0x8e, 0xf2, 0x50, 0x51, 0x73, 0x37,
            0x8a, 0x64, 0xa2, 0x61, 0x05, 0x43, 0x48, 0x44, 0x31, 0x15, 0x66, 0x5b, 0x5e, 0x96,
            0x4e, 0x9b, 0x4a, 0x3c, 0x7c, 0x59,
        ];
        assert!(
            parse(&invalid_x).is_err(),
            "X-coordinate >= field modulus should fail"
        );

        // Test point not on curve (valid x-coordinate but no corresponding y)
        let not_on_curve = [
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ];
        assert!(
            parse(&not_on_curve).is_err(),
            "Point not on curve should fail"
        );

        // Test identity with wrong infinity flag
        let wrong_infinity = [
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(
            parse(&wrong_infinity).is_err(),
            "Wrong infinity flag should fail"
        );
    }

    /// Test parse_g1_compressed with known valid points
    #[test]
    fn test_parse_g1_compressed_known_valid_points() {
        // Test with the actual BLS12-381 generator point (compressed)
        let generator_compressed = hex!("97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb");
        let result = parse(&generator_compressed);
        assert!(
            result.is_ok(),
            "BLS12-381 generator should parse successfully"
        );

        // Test a known valid point from test vectors
        let result = parse(&COMMITMENT);
        assert!(
            result.is_ok(),
            "Known valid point should parse successfully"
        );

        // Test another known valid point with y-bit set
        let result = parse(&PROOF);
        assert!(
            result.is_ok(),
            "Known valid point with y-bit should parse successfully"
        );
    }

    /// Test parse_g1_compressed error conditions comprehensively
    #[test]
    fn test_parse_g1_compressed_comprehensive_errors() {
        // Test various invalid compression flag combinations
        let invalid_flags = [
            0x00, // No compression bit set
            0x20, // Reserved bit set
            0x60, // Multiple reserved bits
            0xe0, // All flag bits except infinity
        ];

        for &flag in &invalid_flags {
            let mut invalid_point = [0u8; 48];
            invalid_point[0] = flag;
            assert!(
                parse(&invalid_point).is_err(),
                "Invalid flag 0x{:02x} should fail",
                flag
            );
        }

        // Test infinity point with non-zero coordinates (should fail)
        let mut invalid_infinity = [0u8; 48];
        invalid_infinity[0] = 0xc0; // Infinity flag
        invalid_infinity[47] = 0x01; // Non-zero coordinate
        assert!(
            parse(&invalid_infinity).is_err(),
            "Infinity point with non-zero coordinates should fail"
        );

        // Test compressed point with both infinity and y-bit flags
        let mut invalid_mixed = [0u8; 48];
        invalid_mixed[0] = 0xf0; // Both infinity and y-bit flags
        assert!(
            parse(&invalid_mixed).is_err(),
            "Point with both infinity and y-bit flags should fail"
        );
    }

    // new tests

    /// A commitment, a proof, `z` and `y`
    type Case = ([u8; 48], [u8; 48], [u8; 32], [u8; 32]);

    /// ethereum/c-kzg-4844 a751e2d92abcd63b01d1400789789f5ea289a8b0,
    /// tests/verify_kzg_proof/kzg-mainnet/{correct_proof_2_3, correct_proof_3_2,
    /// incorrect_proof_2_3}/data.yaml (verify_kzg_proof_case_ prefix): commitment, proof, z, y,
    /// and whether the proof is valid (kept from `affine_glv.rs` before the port)
    const C_KZG_CASES: [(Case, bool); 3] = [
        ((hex!("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06"), hex!("a1fcd37a924af9ec04143b44853c26f6b0738f6e15a3e0755057e7d5460406c7e148adb0e2d608982140d0ae42fe0b3b"), hex!("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62"), hex!("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0")), true),
        ((hex!("b49d88afcd7f6c61a8ea69eff5f609d2432b47e7e4cd50b02cdddb4e0c1460517e8df02e4e64dc55e3d8ca192d57193a"), hex!("a38758fca85407078c0a7e5fd6d38b34340c809baa0e1fed9deaabb11aa503062acbbe23fcbe620a21b40a83bfa71b89"), hex!("0000000000000000000000000000000000000000000000000000000000000002"), hex!("6a75e4fe63e5e148c853462a680c3e3ccedea34719d28f19bf1b35ae4eea37d6")), true),
        ((hex!("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06"), hex!("b3477fc9a5bfab5fdb5523251818ee5a6d52613c59502a3d2df58217f4e366cd9ef37dee55bf2c705a2b08e7808b6fa0"), hex!("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62"), hex!("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0")), false),
    ];

    /// The cases of `kzg_matches_verify_kzg_proof`: the c-kzg vectors and zksync-os's, with the
    /// other scalars of zksync-os's test, a zero `z`, and the proofs swapped or at infinity
    fn cases() -> Vec<Case> {
        let mut infinity = [0u8; 48];
        infinity[0] = 0xc0;
        let unrelated_128_bit_z =
            hex!("000000000000000000000000000000000123456789abcdef0123456789abcdef");
        let mut cases: Vec<_> = C_KZG_CASES.iter().map(|&(case, _)| case).collect();
        cases.extend([
            (COMMITMENT, PROOF, Z, Y),
            (COMMITMENT, PROOF, unrelated_128_bit_z, Y),
            (COMMITMENT, PROOF, Z, [0; 32]),
            (COMMITMENT, PROOF, [0; 32], Y),
            (COMMITMENT, C_KZG_CASES[0].0 .1, Z, Y),
            (COMMITMENT, infinity, Z, Y),
            (infinity, PROOF, Z, Y),
            (infinity, infinity, Z, [0; 32]),
        ]);
        cases
    }

    /// The endpoint (without hints, with the hints of an honest oracle, and, for the valid proofs,
    /// with an oracle that claims every product not to be the identity) gives the result of the
    /// KZG verifier of the crate, `bls12_381::verify_kzg_proof`, which computes the pairings in
    /// full
    #[test]
    fn kzg_matches_verify_kzg_proof() {
        for (i, (commitment, proof, z, y)) in cases().into_iter().enumerate() {
            let expected = crate::bls12_381::verify_kzg_proof(
                parse_g1_compressed(&commitment).unwrap(),
                parse_g1_compressed(&proof).unwrap(),
                parse_scalar(&z).unwrap(),
                parse_scalar(&y).unwrap(),
            );
            if let Some(&(_, valid)) = C_KZG_CASES.get(i) {
                assert_eq!(expected, valid, "c-kzg case {i}");
            }
            let result = kzg_verify_proof::<NoHints>(&commitment, &proof, &z, &y, None);
            assert_eq!(result, Ok(expected), "case {i}");
            let hinted = kzg_verify_proof(
                &commitment,
                &proof,
                &z,
                &y,
                Some(&mut RecordingHintOracle::new()),
            );
            assert_eq!(hinted, Ok(expected), "case {i}");
            if expected {
                // an honest oracle claims the other products not to be the identity anyway
                let claimed = kzg_verify_proof(
                    &commitment,
                    &proof,
                    &z,
                    &y,
                    Some(&mut non_identity_claiming_oracle()),
                );
                assert_eq!(claimed, Ok(expected), "case {i}");
            }
        }
    }

    /// Records the hints of a call, and where the answer of each query starts on the tape, with
    /// the op of the query
    struct QueryLog {
        recorder: RecordingHintOracle,
        queries: Vec<(u32, usize)>,
    }

    impl HintOracle for QueryLog {
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            // SAFETY: the querier is in this process, and its request is live during the query
            // (the contract of `send_query`, guaranteed by the caller)
            let memory = unsafe { NativeQuerierMemory::new() };
            let (op, _, _) = read_field_hint_request(&memory, input_word)?;
            self.queries.push((op, self.recorder.tape().len()));
            // SAFETY: guaranteed by the caller
            unsafe { self.recorder.send_query(query_id, input_word) }
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.recorder.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            // SAFETY: guaranteed by the caller
            unsafe { self.recorder.write_words(dst, num_words) }
        }

        fn finish_query(&mut self) -> Result<(), HintError> {
            self.recorder.finish_query()
        }
    }

    /// The valid c-kzg proofs take the identity claim with a real Miller loop output: the points
    /// of the product, `y G - commitment - z proof` and the proof, are not at infinity (unlike
    /// the degenerate `[O, O]` of the hint's own tests), and the answer of the residue witness is
    /// the identity flag, `d` and `s` (1 + 216 words); the invalid one takes the non-identity
    /// claim, the flag and `f^-1` (1 + 144 words)
    #[test]
    fn c_kzg_vectors_take_the_nontrivial_identity_branch() {
        use crate::ark_ec::{AffineRepr, CurveGroup};
        for ((commitment, proof, z, y), valid) in C_KZG_CASES {
            let c = parse_g1_compressed(&commitment).unwrap();
            let p = parse_g1_compressed(&proof).unwrap();
            let (zs, ys) = (
                Fr::from_bigint(parse_scalar(&z).unwrap()).unwrap(),
                Fr::from_bigint(parse_scalar(&y).unwrap()).unwrap(),
            );
            let left = (G1Affine::generator() * ys - c - p * zs).into_affine();
            assert!(!left.is_zero() && !p.is_zero());

            let mut log = QueryLog {
                recorder: RecordingHintOracle::new(),
                queries: Vec::new(),
            };
            assert_eq!(
                kzg_verify_proof(&commitment, &proof, &z, &y, Some(&mut log)),
                Ok(valid)
            );
            let tape = log.recorder.tape();
            let &(op, start) = log.queries.last().unwrap();
            assert_eq!(op, FieldHintOp::Bls12381KzgResidueWitness as u32);
            // the residue witness is the only query of its kind, and the last
            assert_eq!(
                log.queries
                    .iter()
                    .filter(|(op, _)| *op == FieldHintOp::Bls12381KzgResidueWitness as u32)
                    .count(),
                1
            );
            let answer = &tape[start..];
            if valid {
                assert_eq!((answer[0], answer.len()), (1, 1 + 216));
            } else {
                assert_eq!((answer[0], answer.len()), (0, 1 + 144));
            }
        }
    }

    /// KZG parses the commitment, then the proof, then `z`, then `y` (zksync-os
    /// `point_evaluation.rs:270-293`): the first invalid one gives the error, after the hints of
    /// the points before it
    #[test]
    fn inputs_are_parsed_in_order() {
        let invalid_point = [0u8; 48];
        let parse_tape = |point: &[u8]| {
            record_hints(|oracle| parse_g1_compressed_with_oracle(point, Some(oracle))).1
        };
        let call = |commitment: &[u8; 48], proof: &[u8; 48], z: &[u8; 32], y: &[u8; 32]| {
            record_hints(|oracle| kzg_verify_proof(commitment, proof, z, y, Some(oracle)))
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

    /// With and without the hints of an honest oracle the results are the same: for the valid
    /// and invalid c-kzg proofs, for random points and scalars, and for encodings that are mostly
    /// not points or not canonical scalars
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

    /// The guest, which reads its answers off the recorded tape, gets the same result and takes
    /// the whole tape and no more. The bls12-381 elements of test builds have the representation
    /// of the guest, so the tape is also the answers this process reads.
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
