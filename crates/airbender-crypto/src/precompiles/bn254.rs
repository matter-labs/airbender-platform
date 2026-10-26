// Ported from zksync-os e855bca2:basic_system/src/system_functions/bn254_ecadd.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bn254_ecmul.rs
//! bn254 G1 addition and multiplication (EIP-196) and the pairing check (EIP-197, in
//! `pairing_check`), with the EVM encoding of the points.

use crate::ark_ec::CurveGroup;
use crate::ark_ff::PrimeField;
use crate::ark_serialize::Valid;
use crate::hints::HintOracle;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints::{bn254_fq_inverse, Bn254FqDivider};

#[cfg(any(feature = "alloc", test))]
mod pairing_check;

#[cfg(any(feature = "alloc", test))]
pub use self::pairing_check::bn254_pairing_check;

/// The sum of two points, each the EVM encoding of the coordinates `x` and `y`, big-endian (all
/// zeroes for the point at infinity). Fails unless both are points of the curve. With an oracle,
/// the addition works in affine coordinates, with the division of the slope a checked hint.
// zksync-os's signature: the wrapper maps any failure to its invalid point error
#[allow(clippy::result_unit_err)]
pub fn bn254_g1_add<O: HintOracle>(
    coordinates: &[[u8; 64]; 2],
    oracle: Option<&mut O>,
) -> Result<[u8; 64], ()> {
    use crate::bn254::*;

    let mut points = [G1Affine::identity(); 2];
    for (dst, xy) in points.iter_mut().zip(coordinates.iter()) {
        let is_zero = xy.iter().all(|el| *el == 0);
        if is_zero {
            continue;
        }
        let xy = xy.as_chunks::<32>().0;
        *dst = parse_affine(&xy[0], &xy[1])?;
    }

    let [a, b] = &points;
    let result = match oracle {
        // affine coordinates, with the division of the slope a checked hint
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => serialize_affine(&crate::bn254::g1::add_affine_with_divider(
            a,
            b,
            &mut Bn254FqDivider::new(oracle),
        )),
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        // the in-place group law: arkworks' `Projective + Affine` copies every field element
        None => serialize_projective::<O>(crate::bn254::g1::add_affine(a, b), None),
    };

    Ok(result)
}

/// `scalar` (big-endian, any 256-bit integer) times the point with the big-endian coordinates
/// `x` and `y` (both zero for the point at infinity), in the EVM encoding. Fails unless the
/// point is a point of the curve. With an oracle, the multiplication works in affine
/// coordinates, with its field divisions from checked hints.
// zksync-os's signature: the wrapper maps any failure to its invalid point error
#[allow(clippy::result_unit_err)]
pub fn bn254_g1_mul<O: HintOracle>(
    x: &[u8; 32],
    y: &[u8; 32],
    scalar: &[u8; 32],
    oracle: Option<&mut O>,
) -> Result<[u8; 64], ()> {
    use crate::ark_ec::AffineRepr;

    let is_zero = x.iter().all(|el| *el == 0) && y.iter().all(|el| *el == 0);
    if is_zero {
        return Ok([0u8; 64]);
    }
    let affine_point = parse_affine(x, y)?;
    // the scalar is not reduced: any 256-bit integer is a valid multiplier
    let scalar = bigint_from_be(scalar);

    let result = match oracle {
        // affine coordinates, with every division a checked hint
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => serialize_affine(&crate::bn254::g1::mul_affine_with_divider(
            &affine_point,
            &scalar.0,
            &mut Bn254FqDivider::new(oracle),
        )),
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        None => serialize_projective::<O>(affine_point.mul_bigint(&scalar), None),
    };

    Ok(result)
}

/// The integer encoded big-endian in `bytes`
#[cfg(target_arch = "riscv32")]
pub(crate) fn bigint_from_be(bytes: &[u8; 32]) -> <crate::bn254::Fq as PrimeField>::BigInt {
    <crate::bn254::Fq as PrimeField>::BigInt::new(crate::byte_order::be_bytes_to_le_limbs(bytes))
}

#[cfg(not(target_arch = "riscv32"))]
pub(crate) fn bigint_from_be(bytes: &[u8; 32]) -> <crate::bn254::Fq as PrimeField>::BigInt {
    let mut limbs = [0u64; 4];
    for (limb, chunk) in limbs.iter_mut().zip(bytes.as_chunks::<8>().0.iter().rev()) {
        *limb = u64::from_be_bytes(*chunk);
    }
    <crate::bn254::Fq as PrimeField>::BigInt::new(limbs)
}

/// `value` big-endian in `out`
#[cfg(target_arch = "riscv32")]
fn write_bigint_be(value: &<crate::bn254::Fq as PrimeField>::BigInt, out: &mut [u8; 32]) {
    crate::byte_order::le_limbs_to_be_bytes(&value.0, out);
}

#[cfg(not(target_arch = "riscv32"))]
fn write_bigint_be(value: &<crate::bn254::Fq as PrimeField>::BigInt, out: &mut [u8; 32]) {
    for (chunk, limb) in out
        .as_chunks_mut::<8>()
        .0
        .iter_mut()
        .zip(value.as_ref().iter().rev())
    {
        *chunk = limb.to_be_bytes();
    }
}

/// The curve point with the big-endian coordinates `x` and `y`: `Err` unless both are below
/// the modulus and the point is on the curve (the curve has no other points of small order,
/// so this is also the subgroup check)
pub(crate) fn parse_affine(x: &[u8; 32], y: &[u8; 32]) -> Result<crate::bn254::G1Affine, ()> {
    use crate::bn254::*;
    let x = Fq::from_bigint(bigint_from_be(x)).ok_or(())?;
    let y = Fq::from_bigint(bigint_from_be(y)).ok_or(())?;
    let point = G1Affine::new_unchecked(x, y);
    point.check().map_err(|_| ())?;
    Ok(point)
}

/// The EVM encoding of `point`
// TODO: Investigate the correct approach to avoid warning here
#[allow(dead_code)]
pub(crate) fn serialize_affine(point: &crate::bn254::G1Affine) -> [u8; 64] {
    let mut out = [0u8; 64];
    // zeroes are canonical for the point at infinity
    if !point.infinity {
        let [x, y] = out.as_chunks_mut::<32>().0 else {
            unreachable!("64 bytes are two 32-byte chunks")
        };
        write_bigint_be(&point.x.into_bigint(), x);
        write_bigint_be(&point.y.into_bigint(), y);
    }
    out
}

/// The EVM encoding of `point`. With an oracle, the inversion of its `Z` coordinate comes from
/// a checked hint instead of an exponentiation.
pub(crate) fn serialize_projective<O: HintOracle>(
    point: crate::bn254::G1Projective,
    oracle: Option<&mut O>,
) -> [u8; 64] {
    use crate::ark_ff::Zero;
    if point.is_zero() {
        // canonical for zero point
        [0u8; 64]
    } else {
        let result = match oracle {
            #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
            Some(oracle) => {
                crate::hinted_ops::to_affine_with_inverse(&point, |z| bn254_fq_inverse(oracle, z))
            }
            #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
            Some(_) => super::hints_unavailable(),
            None => point.into_affine(),
        };
        let mut out = [0u8; 64];
        let [x, y] = out.as_chunks_mut::<32>().0 else {
            unreachable!("64 bytes are two 32-byte chunks")
        };
        write_bigint_be(&result.x.into_bigint(), x);
        write_bigint_be(&result.y.into_bigint(), y);
        out
    }
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::ark_ec::{AffineRepr, CurveGroup};
    use crate::ark_ff::{BigInteger, Field, PrimeField};
    use crate::hints::test_support::TapeOracle;
    use crate::hints::{NoHints, RecordingHintOracle};
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};
    use std::vec;
    use std::vec::Vec;

    fn oracle() -> RecordingHintOracle {
        RecordingHintOracle::new()
    }

    fn be(value: crate::bn254::Fq) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        for (chunk, limb) in bytes
            .as_chunks_mut::<8>()
            .0
            .iter_mut()
            .zip(value.into_bigint().as_ref().iter().rev())
        {
            *chunk = limb.to_be_bytes();
        }
        bytes
    }

    /// Encoded points: the point at infinity, multiples of the generator and their negations
    // `p += p + generator` doubles `p` and adds the generator, as in zksync-os
    #[allow(clippy::misrefactored_assign_op)]
    fn points() -> Vec<[u8; 64]> {
        let generator = crate::bn254::G1Affine::generator();
        let mut points = vec![[0u8; 64]];
        let mut p = generator.into_group();
        for _ in 0..4 {
            for point in [p.into_affine(), (-p).into_affine()] {
                let mut encoded = [0u8; 64];
                encoded[..32].copy_from_slice(&be(point.x));
                encoded[32..].copy_from_slice(&be(point.y));
                points.push(encoded);
            }
            p += p + generator;
        }
        points
    }

    fn scalars() -> Vec<[u8; 32]> {
        let order: [u8; 32] = <crate::bn254::Fr as PrimeField>::MODULUS
            .to_bytes_be()
            .try_into()
            .unwrap();
        let mut scalars = vec![[0u8; 32], [0xff; 32], order];
        for delta in [1u8, 2] {
            let mut below = order;
            // zksync-os runs its tests in release, where `below[31] -= delta` wraps: the order
            // ends in byte 1, so delta 2 makes `order + 254`, kept here (and `order - 2` below)
            below[31] = below[31].wrapping_sub(delta);
            scalars.push(below);
            let mut above = order;
            above[31] += delta;
            scalars.push(above);
            let mut small = [0u8; 32];
            small[31] = delta;
            scalars.push(small);
        }
        let mut below = <crate::bn254::Fr as PrimeField>::MODULUS;
        below.sub_with_borrow(&2u64.into());
        scalars.push(below.to_bytes_be().try_into().unwrap());
        let mut k = crate::bn254::Fr::from(0x9e37_79b9_7f4a_7c15u64);
        for _ in 0..12 {
            k = k.square() + crate::bn254::Fr::from(3u64);
            scalars.push(k.into_bigint().to_bytes_be().try_into().unwrap());
        }
        scalars
    }

    /// With the advice of the oracle (affine arithmetic with hinted divisions) the results are
    /// the ones without it
    #[test]
    fn multiplication_with_advice_matches() {
        for point in points() {
            let (x, y) = (
                point[..32].try_into().unwrap(),
                point[32..].try_into().unwrap(),
            );
            for scalar in scalars() {
                let expected = bn254_g1_mul::<NoHints>(x, y, &scalar, None);
                let hinted = bn254_g1_mul(x, y, &scalar, Some(&mut oracle()));
                assert!(expected.is_ok());
                assert_eq!(hinted, expected);
            }
        }
    }

    #[test]
    fn addition_with_advice_matches() {
        for a in points() {
            for b in points() {
                let expected = bn254_g1_add::<NoHints>(&[a, b], None);
                let hinted = bn254_g1_add(&[a, b], Some(&mut oracle()));
                assert!(expected.is_ok());
                assert_eq!(hinted, expected);
            }
        }
    }

    #[test]
    fn points_off_the_curve_are_rejected() {
        let mut point = points()[1];
        point[63] ^= 1;
        let (x, y) = (
            point[..32].try_into().unwrap(),
            point[32..].try_into().unwrap(),
        );
        assert!(bn254_g1_mul(x, y, &[1; 32], Some(&mut oracle())).is_err());
        assert!(bn254_g1_add(&[point, points()[3]], Some(&mut oracle())).is_err());
    }

    /// The coordinates `x` and `y` of an encoded point
    fn xy(point: &[u8; 64]) -> (&[u8; 32], &[u8; 32]) {
        let ([x, y], []) = point.as_chunks::<32>() else {
            unreachable!("64 bytes are 2 chunks of 32")
        };
        (x, y)
    }

    /// The encoding of `k` times the generator
    fn multiple(k: u64) -> [u8; 64] {
        let point = (crate::bn254::G1Affine::generator() * crate::bn254::Fr::from(k)).into_affine();
        let mut encoded = [0u8; 64];
        if !point.infinity {
            encoded[..32].copy_from_slice(&be(point.x));
            encoded[32..].copy_from_slice(&be(point.y));
        }
        encoded
    }

    /// The encoding of a point of the curve negated (the point at infinity stays)
    fn negated(point: &[u8; 64]) -> [u8; 64] {
        let y = crate::bn254::Fq::from_be_bytes_mod_order(&point[32..]);
        let mut negated = *point;
        negated[32..].copy_from_slice(&be(-y));
        negated
    }

    mod advice_matches_no_advice {
        use super::*;
        use crate::precompiles::HintedCases;

        #[test]
        fn add() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(64), |(ka: u64, kb: u64, mode in 0u8..6, raw: [[u8; 32]; 2])| {
                let a = multiple(ka);
                let b = match mode {
                    0 => multiple(kb),
                    // doubling, and the sum of opposite points
                    1 => a,
                    2 => negated(&a),
                    3 => [0; 64],
                    // mostly not a point of the curve, or coordinates above the modulus
                    4 => {
                        let mut b = [0u8; 64];
                        b[..32].copy_from_slice(&raw[0]);
                        b[32..].copy_from_slice(&raw[1]);
                        b
                    }
                    _ => {
                        let mut b = a;
                        b[63] ^= 1;
                        b
                    }
                };
                for points in [[a, b], [b, a]] {
                    let hinted = hinted_cases.record(|oracle| bn254_g1_add(&points, Some(oracle)));
                    prop_assert_eq!(hinted, bn254_g1_add(&points, None::<&mut NoHints>));
                }
            });
            hinted_cases.check();
        }

        #[test]
        fn mul() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(64), |(k: u64, scalar: [u8; 32], mode in 0u8..4, raw: [[u8; 32]; 2])| {
                let point = match mode {
                    0 | 1 => multiple(k),
                    2 => [0; 64],
                    // mostly not a point of the curve, or coordinates above the modulus
                    _ => {
                        let mut point = [0u8; 64];
                        point[..32].copy_from_slice(&raw[0]);
                        point[32..].copy_from_slice(&raw[1]);
                        point
                    }
                };
                let (x, y) = xy(&point);
                let hinted = hinted_cases.record(|oracle| bn254_g1_mul(x, y, &scalar, Some(oracle)));
                prop_assert_eq!(hinted, bn254_g1_mul(x, y, &scalar, None::<&mut NoHints>));
            });
            hinted_cases.check();
        }
    }

    /// A zero scalar (or one the group order reduces to zero) gives the point at infinity without
    /// asking for any hint
    #[test]
    fn zero_scalar_makes_no_query() {
        let order: [u8; 32] = <crate::bn254::Fr as PrimeField>::MODULUS
            .to_bytes_be()
            .try_into()
            .unwrap();
        for point in points() {
            let (x, y) = xy(&point);
            for scalar in [[0u8; 32], order] {
                let mut oracle = TapeOracle::new([]);
                assert_eq!(
                    bn254_g1_mul(x, y, &scalar, Some(&mut oracle)),
                    Ok([0u8; 64])
                );
                assert!(oracle.queries.is_empty());
            }
        }
    }

    /// The guest, which reads its answers off the recorded tape, gets the same result and takes
    /// the whole tape and no more. The bn254 elements of test builds have the representation of
    /// the guest, so the tape is also the answers this process reads.
    #[test]
    fn recorded_tape_replays() {
        // the generator and 3 times it (the generator plus its negation takes no division)
        let [_, a, _, b, ..] = points()[..] else {
            unreachable!("there are 9 points")
        };
        let sum = assert_tape_replays!(|oracle| bn254_g1_add(&[a, b], Some(oracle)));
        assert_eq!(sum, bn254_g1_add(&[a, b], None::<&mut NoHints>));
        let scalar = scalars()[1];
        let (x, y) = xy(&a);
        let product = assert_tape_replays!(|oracle| bn254_g1_mul(x, y, &scalar, Some(oracle)));
        assert_eq!(product, bn254_g1_mul(x, y, &scalar, None::<&mut NoHints>));
    }
}
