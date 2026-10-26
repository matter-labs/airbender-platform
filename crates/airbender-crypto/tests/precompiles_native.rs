//! The precompile endpoints with the hints of the recorder, in a build of the crate without
//! `cfg(test)`. Its fields are the arkworks ones unless `proving` is on, so the answers this
//! process reads are not the ones of the RISC-V guest (e.g. a bls12-381 element is 6 limbs here,
//! and secp256k1 answers are canonical integers rather than Montgomery words). The unit tests of
//! the crate run on the delegated fields of its test build, whose representation is the guest's.
#![cfg(all(feature = "alloc", target_pointer_width = "64"))]
// `std::alloc::Global`, the allocator of the endpoints that allocate
#![cfg_attr(
    all(feature = "alloc", target_pointer_width = "64"),
    feature(allocator_api)
)]

use airbender_crypto::ark_ec::{AffineRepr, CurveGroup};
use airbender_crypto::ark_ff::{BigInteger, PrimeField};
use airbender_crypto::bls12_381::eip2537::{
    serialize_fq2_bytes, serialize_fq_bytes, serialize_g1_bytes, serialize_g2_bytes,
};
use airbender_crypto::hints::{record_hints, NoHints};
use airbender_crypto::precompiles::{
    bls12_381_g1_add, bls12_381_g1_msm, bls12_381_g2_add, bls12_381_g2_msm,
    bls12_381_map_fp2_to_g2, bls12_381_map_fp_to_g1, bls12_381_pairing_check, bn254_g1_add,
    bn254_g1_mul, bn254_pairing_check, kzg_verify_proof,
};
use airbender_crypto::{bls12_381, bn254};
use hex_literal::hex;
use std::alloc::Global;

fn bn254_point(k: u64) -> [u8; 64] {
    let point = (bn254::G1Affine::generator() * bn254::Fr::from(k)).into_affine();
    let mut encoded = [0u8; 64];
    encoded[..32].copy_from_slice(&point.x.into_bigint().to_bytes_be());
    encoded[32..].copy_from_slice(&point.y.into_bigint().to_bytes_be());
    encoded
}

fn g1(k: u64) -> [u8; 128] {
    let mut encoded = [0u8; 128];
    let point = (bls12_381::G1Affine::generator() * bls12_381::Fr::from(k)).into_affine();
    serialize_g1_bytes(point, &mut encoded);
    encoded
}

fn g2(k: u64) -> [u8; 256] {
    let mut encoded = [0u8; 256];
    let point = (bls12_381::G2Affine::generator() * bls12_381::Fr::from(k)).into_affine();
    serialize_g2_bytes(point, &mut encoded);
    encoded
}

#[test]
fn bn254_endpoints_with_hints_match() {
    for (a, b) in [(1, 2), (1, 1), (5, 7)] {
        let points = [bn254_point(a), bn254_point(b)];
        let (hinted, tape) = record_hints(|oracle| bn254_g1_add(&points, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, bn254_g1_add(&points, None::<&mut NoHints>));
    }
    for (k, scalar) in [(1, [0xff; 32]), (3, [0x5a; 32])] {
        let point = bn254_point(k);
        let (x, y) = (
            point[..32].try_into().unwrap(),
            point[32..].try_into().unwrap(),
        );
        let (hinted, tape) = record_hints(|oracle| bn254_g1_mul(x, y, &scalar, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, bn254_g1_mul(x, y, &scalar, None::<&mut NoHints>));
    }
}

#[test]
fn bls12_381_endpoints_with_hints_match() {
    for (a, b) in [(1, 2), (1, 1)] {
        let (a, b) = (g1(a), g1(b));
        let (hinted, tape) = record_hints(|oracle| bls12_381_g1_add(&a, &b, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, bls12_381_g1_add(&a, &b, None::<&mut NoHints>));
    }
    let (a, b) = (g2(1), g2(2));
    let (hinted, tape) = record_hints(|oracle| bls12_381_g2_add(&a, &b, Some(oracle)));
    assert!(!tape.is_empty());
    assert_eq!(hinted, bls12_381_g2_add(&a, &b, None::<&mut NoHints>));

    for value in [0u64, 7] {
        let mut fp = [0u8; 64];
        serialize_fq_bytes(bls12_381::Fq::from(value), &mut fp);
        let (hinted, tape) = record_hints(|oracle| bls12_381_map_fp_to_g1(&fp, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, bls12_381_map_fp_to_g1(&fp, None::<&mut NoHints>));
    }
    let mut fp2 = [0u8; 128];
    let value = bls12_381::Fq2::new(bls12_381::Fq::from(7u64), bls12_381::Fq::from(11u64));
    serialize_fq2_bytes(value, &mut fp2);
    let (hinted, tape) = record_hints(|oracle| bls12_381_map_fp2_to_g2(&fp2, Some(oracle)));
    assert!(!tape.is_empty());
    assert_eq!(hinted, bls12_381_map_fp2_to_g2(&fp2, None::<&mut NoHints>));
}

#[cfg(feature = "secp256k1-static-context")]
#[test]
fn ecrecover_with_hints_matches() {
    use airbender_crypto::k256::ecdsa::SigningKey;
    use airbender_crypto::precompiles::secp256k1_ecrecover;

    for n in 1..=4u8 {
        let digest = [n.wrapping_mul(37); 32];
        let key = SigningKey::from_bytes(&[n; 32].into()).unwrap();
        let (signature, recovery_id) = key.sign_prehash_recoverable(&digest).unwrap();
        let bytes = signature.to_bytes();
        let r: [u8; 32] = bytes[..32].try_into().unwrap();
        let s: [u8; 32] = bytes[32..].try_into().unwrap();
        let odd = recovery_id.is_y_odd();
        let (hinted, tape) = record_hints(|oracle| {
            secp256k1_ecrecover(&digest, &r, &s, odd, Some(oracle)).map(|key| key.0)
        });
        assert!(!tape.is_empty());
        assert_eq!(
            hinted,
            secp256k1_ecrecover(&digest, &r, &s, odd, None::<&mut NoHints>).map(|key| key.0)
        );
        let public_key = key.verifying_key().to_encoded_point(false);
        assert_eq!(hinted.unwrap()[..], public_key.as_bytes()[1..]);
    }
}

/// The EIP-197 encoding of a pair (the imaginary parts of the G2 coordinates first)
fn bn254_pair(p: bn254::G1Affine, q: bn254::G2Affine) -> Vec<u8> {
    [p.x, p.y, q.x.c1, q.x.c0, q.y.c1, q.y.c0]
        .iter()
        .flat_map(|element| element.into_bigint().to_bytes_be())
        .collect()
}

#[test]
fn bn254_pairing_with_hints_matches() {
    let g1 = |k: u64| (bn254::G1Affine::generator() * bn254::Fr::from(k)).into_affine();
    let g2 = |k: u64| (bn254::G2Affine::generator() * bn254::Fr::from(k)).into_affine();
    // `e(6 G1, G2) e(-2 G1, 3 G2)` is the identity (the residue witness settles it), and
    // `e(6 G1, G2)` is not (the exact final exponentiation does)
    for (pairs, expected) in [
        (vec![(g1(6), g2(1)), (-g1(2), g2(3))], true),
        (vec![(g1(6), g2(1))], false),
    ] {
        let src: Vec<u8> = pairs.iter().flat_map(|&(p, q)| bn254_pair(p, q)).collect();
        let n = pairs.len();
        let (hinted, tape) =
            record_hints(|oracle| bn254_pairing_check(n, &src, Global, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, Ok(expected));
        assert_eq!(
            bn254_pairing_check(n, &src, Global, None::<&mut NoHints>),
            Ok(expected)
        );
    }
}

#[test]
fn bls12_381_msm_and_pairing_with_hints_match() {
    // the affine chains of one pair, and the buckets of four
    for n in [1u64, 4] {
        let input: Vec<u8> = (1..=n)
            .flat_map(|k| [g1(k).as_slice(), &[0xa5; 32]].concat())
            .collect();
        let input = input.as_chunks::<160>().0;
        let (hinted, tape) = record_hints(|oracle| bls12_381_g1_msm(input, Global, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(
            hinted,
            bls12_381_g1_msm(input, Global, None::<&mut NoHints>)
        );
    }
    let input = [g2(3).as_slice(), &[0xa5; 32]].concat();
    let input = input.as_chunks::<288>().0;
    let (hinted, tape) = record_hints(|oracle| bls12_381_g2_msm(input, Global, Some(oracle)));
    assert!(!tape.is_empty());
    assert_eq!(
        hinted,
        bls12_381_g2_msm(input, Global, None::<&mut NoHints>)
    );

    // `e(G1, G2) e(-G1, G2)` is the identity, `e(G1, G2)` is not
    let mut negated = [0u8; 128];
    serialize_g1_bytes(-bls12_381::G1Affine::generator(), &mut negated);
    for (input, expected) in [
        ([g1(1).as_slice(), &g2(1), &negated, &g2(1)].concat(), true),
        ([g1(1).as_slice(), &g2(1)].concat(), false),
    ] {
        let input = input.as_chunks::<384>().0;
        let (hinted, tape) =
            record_hints(|oracle| bls12_381_pairing_check(input, Global, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, Ok(expected));
        assert_eq!(
            bls12_381_pairing_check(input, Global, None::<&mut NoHints>),
            Ok(expected)
        );
    }
}

#[test]
fn kzg_with_hints_matches() {
    // ethereum/c-kzg-4844 a751e2d92abcd63b01d1400789789f5ea289a8b0,
    // tests/verify_kzg_proof/kzg-mainnet/verify_kzg_proof_case_{correct,incorrect}_proof_2_3
    let commitment = hex!("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06");
    let z = hex!("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62");
    let y = hex!("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0");
    for (proof, expected) in [
        (hex!("a1fcd37a924af9ec04143b44853c26f6b0738f6e15a3e0755057e7d5460406c7e148adb0e2d608982140d0ae42fe0b3b"), true),
        (hex!("b3477fc9a5bfab5fdb5523251818ee5a6d52613c59502a3d2df58217f4e366cd9ef37dee55bf2c705a2b08e7808b6fa0"), false),
    ] {
        let (hinted, tape) =
            record_hints(|oracle| kzg_verify_proof(&commitment, &proof, &z, &y, Some(oracle)));
        assert!(!tape.is_empty());
        assert_eq!(hinted, Ok(expected));
        assert_eq!(
            kzg_verify_proof(&commitment, &proof, &z, &y, None::<&mut NoHints>),
            Ok(expected)
        );
    }
}
