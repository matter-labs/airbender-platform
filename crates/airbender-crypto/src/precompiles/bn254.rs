//! bn254 G1 addition and multiplication (EIP-196) and the pairing check (EIP-197), on the EVM
//! encoding of the points.

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

/// The sum of two points, each big-endian `x || y` (all zeroes for the point at infinity).
/// Fails unless both are on the curve. With an oracle, adds in affine coordinates with a checked
/// hinted slope division.
// The caller maps every failure to its invalid-point error
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
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => serialize_affine(&crate::bn254::g1::add_affine_with_divider(
            a,
            b,
            &mut Bn254FqDivider::new(oracle),
        )),
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        // in place: arkworks' `Projective + Affine` copies every field element
        None => serialize_projective::<O>(crate::bn254::g1::add_affine(a, b), None),
    };

    Ok(result)
}

/// `scalar` (big-endian, any 256-bit integer, not reduced) times the point `(x, y)` (both zero
/// for the point at infinity), in the EVM encoding. Fails unless the point is on the curve. With
/// an oracle, multiplies in affine coordinates with checked hinted divisions.
// The caller maps every failure to its invalid-point error
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
    let scalar = bigint_from_be(scalar);

    let result = match oracle {
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

/// The point with big-endian coordinates `x` and `y`: `Err` unless both are below the modulus
/// and the point is on the curve (G1 has cofactor 1, so this is also the subgroup check)
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
    if !point.infinity {
        let [x, y] = out.as_chunks_mut::<32>().0 else {
            unreachable!("64 bytes are two 32-byte chunks")
        };
        write_bigint_be(&point.x.into_bigint(), x);
        write_bigint_be(&point.y.into_bigint(), y);
    }
    out
}

/// The EVM encoding of `point`. With an oracle, `1 / Z` is a checked hint.
pub(crate) fn serialize_projective<O: HintOracle>(
    point: crate::bn254::G1Projective,
    oracle: Option<&mut O>,
) -> [u8; 64] {
    use crate::ark_ff::Zero;
    if point.is_zero() {
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
    use crate::ark_ff::{BigInteger, PrimeField};
    use crate::hints::test_support::TapeOracle;
    use crate::hints::{record_hints, NoHints};
    use crate::precompiles::HintedCases;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};

    fn be(value: crate::bn254::Fq) -> [u8; 32] {
        value.into_bigint().to_bytes_be().try_into().unwrap()
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

    /// The coordinates `x` and `y` of an encoded point
    fn xy(point: &[u8; 64]) -> (&[u8; 32], &[u8; 32]) {
        let ([x, y], []) = point.as_chunks::<32>() else {
            unreachable!("64 bytes are 2 chunks of 32")
        };
        (x, y)
    }

    /// Scalars around the group order, and 1 and `2^256 - 1`
    fn edge_scalars() -> [[u8; 32]; 6] {
        let order = <crate::bn254::Fr as PrimeField>::MODULUS;
        let (mut below, mut above) = (order, order);
        below.sub_with_borrow(&1u64.into());
        above.add_with_carry(&1u64.into());
        let be = |n: <crate::bn254::Fr as PrimeField>::BigInt| n.to_bytes_be().try_into().unwrap();
        let mut one = [0u8; 32];
        one[31] = 1;
        [[0; 32], be(order), one, be(below), be(above), [0xff; 32]]
    }

    /// Hinted and plain multiplication agree on edge scalars; a scalar that is zero modulo the
    /// order gives the point at infinity without a query
    #[test]
    fn mul_edge_scalars() {
        for point in [multiple(1), multiple(3), negated(&multiple(5))] {
            let (x, y) = xy(&point);
            for (i, scalar) in edge_scalars().iter().enumerate() {
                let plain = bn254_g1_mul(x, y, scalar, None::<&mut NoHints>);
                assert!(plain.is_ok());
                if i < 2 {
                    let mut oracle = TapeOracle::new([]);
                    assert_eq!(bn254_g1_mul(x, y, scalar, Some(&mut oracle)), Ok([0; 64]));
                    assert!(oracle.queries.is_empty());
                    assert_eq!(plain, Ok([0; 64]));
                } else {
                    let (hinted, _) =
                        record_hints(|oracle| bn254_g1_mul(x, y, scalar, Some(oracle)));
                    assert_eq!(hinted, plain);
                }
            }
        }
    }

    #[test]
    fn points_off_the_curve_are_rejected() {
        let mut point = multiple(1);
        point[63] ^= 1;
        let (x, y) = xy(&point);
        let mut oracle = TapeOracle::new([]);
        assert!(bn254_g1_mul(x, y, &[1; 32], Some(&mut oracle)).is_err());
        assert!(bn254_g1_add(&[point, multiple(3)], Some(&mut oracle)).is_err());
        assert!(bn254_g1_add(&[multiple(3), point], None::<&mut NoHints>).is_err());
    }

    mod advice_matches_no_advice {
        use super::*;

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
                    4 => raw.concat().try_into().unwrap(),
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
                    _ => raw.concat().try_into().unwrap(),
                };
                let (x, y) = xy(&point);
                let hinted = hinted_cases.record(|oracle| bn254_g1_mul(x, y, &scalar, Some(oracle)));
                prop_assert_eq!(hinted, bn254_g1_mul(x, y, &scalar, None::<&mut NoHints>));
            });
            hinted_cases.check();
        }
    }

    #[test]
    fn recorded_tape_replays() {
        // the generator plus its negation would take no division
        let (a, b) = (multiple(1), multiple(3));
        let sum = assert_tape_replays!(|oracle| bn254_g1_add(&[a, b], Some(oracle)));
        assert_eq!(sum, bn254_g1_add(&[a, b], None::<&mut NoHints>));
        let (x, y) = xy(&a);
        let product = assert_tape_replays!(|oracle| bn254_g1_mul(x, y, &[0xff; 32], Some(oracle)));
        assert_eq!(
            product,
            bn254_g1_mul(x, y, &[0xff; 32], None::<&mut NoHints>)
        );
    }
}
