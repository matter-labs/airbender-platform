//! BLS12-381 G1/G2 addition, the field-to-curve maps, the MSMs and the pairing check (EIP-2537),
//! on the EIP-2537 encoding of the points.

use crate::ark_ec::{AffineRepr, CurveGroup};
#[cfg(any(feature = "alloc", test))]
use crate::ark_ff::PrimeField;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::ark_ff::Zero;
use crate::bls12_381::*;
use crate::hints::HintOracle;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints::{
    bls12_381_fq2_inverse, bls12_381_fq2_sqrt, bls12_381_fq_inverse, bls12_381_fq_sqrt,
    Bls12381Fq2Divider, Bls12381FqDivider,
};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use core::cell::RefCell;

// the dividers of the subgroup checks of the MSMs and the pairing check
#[cfg(all(
    any(feature = "alloc", test),
    target_arch = "riscv32",
    not(feature = "bigint_ops")
))]
use super::unavailable::{Bls12381Fq2Divider, Bls12381FqDivider};

#[cfg(any(feature = "alloc", test))]
mod msm;
#[cfg(any(feature = "alloc", test))]
mod pairing;

#[cfg(any(feature = "alloc", test))]
pub use self::msm::{bls12_381_g1_msm, bls12_381_g2_msm, G1_MSM_PAIR_LEN, G2_MSM_PAIR_LEN};
#[cfg(any(feature = "alloc", test))]
pub use self::pairing::{bls12_381_pairing_check, BLS12_381_PAIR_LEN};

#[cfg(any(feature = "alloc", test))]
const SCALAR_SERIALIZATION_LEN: usize = 32;
/// The size of the EIP-2537 encoding of a base field element
pub const FIELD_ELEMENT_SERIALIZATION_LEN: usize = 64;
/// The size of the EIP-2537 encoding of an `Fq2` element
pub const FIELD_EXT_ELEMENT_SERIALIZATION_LEN: usize = FIELD_ELEMENT_SERIALIZATION_LEN * 2;
/// The size of the EIP-2537 encoding of a G1 point
pub const G1_SERIALIZATION_LEN: usize = FIELD_ELEMENT_SERIALIZATION_LEN * 2;
/// The size of the EIP-2537 encoding of a G2 point
pub const G2_SERIALIZATION_LEN: usize = FIELD_EXT_ELEMENT_SERIALIZATION_LEN * 2;

/// The errors of the endpoints (the caller checks the input size)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bls12Error {
    /// A G1 point is not a canonical encoding of a point of the curve
    InvalidG1Point,
    /// A G2 point is not a canonical encoding of a point of the curve
    InvalidG2Point,
    /// A point is not in the prime-order subgroup
    NotInSubgroup,
    /// A field element is not canonical
    InvalidFieldElement,
}

// infallible: scalars need not be canonical
#[cfg(any(feature = "alloc", test))]
fn parse_integer(input: &[u8; SCALAR_SERIALIZATION_LEN]) -> <Fr as PrimeField>::BigInt {
    let mut repr = [0u64; 4];
    for (dst, src) in repr.iter_mut().zip(input.as_rchunks::<8>().1.iter().rev()) {
        *dst = u64::from_be_bytes(*src);
    }
    crate::BigInt::new(repr)
}

fn parse_g1(input: &[u8; G1_SERIALIZATION_LEN]) -> Result<G1Affine, Bls12Error> {
    crate::bls12_381::eip2537::parse_g1_bytes(input)
        .map(|(point, _)| point)
        .ok_or(Bls12Error::InvalidG1Point)
}

fn parse_g2(input: &[u8; G2_SERIALIZATION_LEN]) -> Result<G2Affine, Bls12Error> {
    crate::bls12_381::eip2537::parse_g2_bytes(input)
        .map(|(point, _)| point)
        .ok_or(Bls12Error::InvalidG2Point)
}

// the MSMs and the pairing check
#[cfg(any(feature = "alloc", test))]
fn parse_g1_with_subgroup_check(
    input: &[u8; G1_SERIALIZATION_LEN],
) -> Result<G1Affine, Bls12Error> {
    let point = parse_g1(input)?;
    if point.is_zero() || point.is_in_correct_subgroup_assuming_on_curve() {
        Ok(point)
    } else {
        Err(Bls12Error::NotInSubgroup)
    }
}

#[cfg(any(feature = "alloc", test))]
fn parse_g2_with_subgroup_check(
    input: &[u8; G2_SERIALIZATION_LEN],
) -> Result<G2Affine, Bls12Error> {
    let point = parse_g2(input)?;
    if point.is_zero() || point.is_in_correct_subgroup_assuming_on_curve() {
        Ok(point)
    } else {
        Err(Bls12Error::NotInSubgroup)
    }
}

/// Parses and checks subgroup membership; with an oracle, by an affine chain with checked hinted
/// divisions. On RV32 without `bigint_ops` the divider panics (see `unavailable`).
#[cfg(any(feature = "alloc", test))]
fn parse_g1_with_subgroup_check_with_oracle<O: HintOracle>(
    input: &[u8; G1_SERIALIZATION_LEN],
    oracle: Option<&mut O>,
) -> Result<G1Affine, Bls12Error> {
    let Some(oracle) = oracle else {
        return parse_g1_with_subgroup_check(input);
    };
    let point = parse_g1(input)?;
    if point.is_zero()
        || g1::is_in_subgroup_with_divider(&point, &mut Bls12381FqDivider::new(oracle))
    {
        Ok(point)
    } else {
        Err(Bls12Error::NotInSubgroup)
    }
}

#[cfg(any(feature = "alloc", test))]
fn parse_g2_with_subgroup_check_with_oracle<O: HintOracle>(
    input: &[u8; G2_SERIALIZATION_LEN],
    oracle: Option<&mut O>,
) -> Result<G2Affine, Bls12Error> {
    let Some(oracle) = oracle else {
        return parse_g2_with_subgroup_check(input);
    };
    let point = parse_g2(input)?;
    if point.is_zero()
        || g2::is_in_subgroup_with_divider(&point, &mut Bls12381Fq2Divider::new(oracle))
    {
        Ok(point)
    } else {
        Err(Bls12Error::NotInSubgroup)
    }
}

/// The sum of two G1 points (which need not be in the subgroup). With an oracle, adds in affine
/// coordinates with a checked hinted slope division. The caller encodes the result.
pub fn bls12_381_g1_add<O: HintOracle>(
    a: &[u8; G1_SERIALIZATION_LEN],
    b: &[u8; G1_SERIALIZATION_LEN],
    oracle: Option<&mut O>,
) -> Result<G1Affine, Bls12Error> {
    let p0 = parse_g1(a)?;
    let p1 = parse_g1(b)?;

    let result = match oracle {
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => g1::add_affine_with_divider(&p0, &p1, &mut Bls12381FqDivider::new(oracle)),
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        None => (p0 + p1).into_affine(),
    };

    Ok(result)
}

/// The sum of two G2 points (which need not be in the subgroup), as [`bls12_381_g1_add`]
pub fn bls12_381_g2_add<O: HintOracle>(
    a: &[u8; G2_SERIALIZATION_LEN],
    b: &[u8; G2_SERIALIZATION_LEN],
    oracle: Option<&mut O>,
) -> Result<G2Affine, Bls12Error> {
    let p0 = parse_g2(a)?;
    let p1 = parse_g2(b)?;

    let result = match oracle {
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => g2::add_affine_with_divider(&p0, &p1, &mut Bls12381Fq2Divider::new(oracle)),
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        None => (p0 + p1).into_affine(),
    };

    Ok(result)
}

/// The G1 point of a field element: the simplified SWU map, then cofactor clearing. With an
/// oracle, the inversions, square roots and divisions are checked hints. The caller encodes the
/// result.
pub fn bls12_381_map_fp_to_g1<O: HintOracle>(
    fp: &[u8; FIELD_ELEMENT_SERIALIZATION_LEN],
    oracle: Option<&mut O>,
) -> Result<G1Affine, Bls12Error> {
    let field_element =
        crate::bls12_381::eip2537::parse_fq_bytes(fp).ok_or(Bls12Error::InvalidFieldElement)?;
    let invalid_field = || Bls12Error::InvalidFieldElement;
    let result = match oracle {
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => {
            // The callbacks run sequentially; both need access to the same oracle.
            let oracle = RefCell::new(oracle);
            let point = crate::bls12_381::eip2537::map_fp_to_g1_with_hints(
                field_element,
                |value| bls12_381_fq_inverse(*oracle.borrow_mut(), value),
                |value, z| {
                    let mut oracle = oracle.borrow_mut();
                    // Match SWU's Legendre convention: zero takes the non-residue branch.
                    if !value.is_zero() {
                        if let Some(root) = bls12_381_fq_sqrt(*oracle, value) {
                            return (root, true);
                        }
                    }
                    let root = bls12_381_fq_sqrt(*oracle, &(*z * value))
                        .expect("the SWU non-residue times a non-square is square");
                    (root, false)
                },
            );
            crate::bls12_381::curves::g1::clear_cofactor_with_divider(
                &point,
                &mut Bls12381FqDivider::new(oracle.into_inner()),
            )
        }
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        None => crate::bls12_381::eip2537::map_fp_to_g1(field_element)
            .map_err(|_| invalid_field())?
            .clear_cofactor(),
    };

    Ok(result)
}

/// The G2 point of an `Fp2` element, as [`bls12_381_map_fp_to_g1`]
pub fn bls12_381_map_fp2_to_g2<O: HintOracle>(
    fp2: &[u8; FIELD_EXT_ELEMENT_SERIALIZATION_LEN],
    oracle: Option<&mut O>,
) -> Result<G2Affine, Bls12Error> {
    let field_element =
        crate::bls12_381::eip2537::parse_fq2_bytes(fp2).ok_or(Bls12Error::InvalidFieldElement)?;

    let invalid_field = || Bls12Error::InvalidFieldElement;
    let result = match oracle {
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => {
            // The callbacks run sequentially; both need access to the same oracle.
            let oracle = RefCell::new(oracle);
            let point = crate::bls12_381::eip2537::map_fp2_to_g2_with_hints(
                field_element,
                |value| bls12_381_fq2_inverse(*oracle.borrow_mut(), value),
                |value, z| {
                    let mut oracle = oracle.borrow_mut();
                    // Match SWU's Legendre convention: zero takes the non-residue branch.
                    if !value.is_zero() {
                        if let Some(root) = bls12_381_fq2_sqrt(*oracle, value) {
                            return (root, true);
                        }
                    }
                    let root = bls12_381_fq2_sqrt(*oracle, &(*z * value))
                        .expect("the SWU non-residue times a non-square is square");
                    (root, false)
                },
            );
            crate::bls12_381::curves::g2::clear_cofactor_with_divider(
                &point,
                &mut Bls12381Fq2Divider::new(oracle.into_inner()),
            )
        }
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        None => crate::bls12_381::eip2537::map_fp2_to_g2(field_element)
            .map_err(|_| invalid_field())?
            .clear_cofactor(),
    };

    Ok(result)
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::ark_ff::{AdditiveGroup, BigInteger, PrimeField};
    use crate::bls12_381::eip2537::{serialize_fq2_bytes, serialize_fq_bytes};
    use crate::hints::test_support::{lying_oracle, TapeOracle};
    use crate::hints::{record_hints, FieldHintOp, NoHints};
    use crate::precompiles::HintedCases;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};

    pub(super) fn write_g1(el: G1Affine) -> [u8; G1_SERIALIZATION_LEN] {
        let mut buffer = [0u8; G1_SERIALIZATION_LEN];
        crate::bls12_381::eip2537::serialize_g1_bytes(el, &mut buffer);
        buffer
    }

    pub(super) fn write_g2(el: G2Affine) -> [u8; G2_SERIALIZATION_LEN] {
        let mut buffer = [0u8; G2_SERIALIZATION_LEN];
        crate::bls12_381::eip2537::serialize_g2_bytes(el, &mut buffer);
        buffer
    }

    /// A point of order 3 on the G1 curve
    pub(super) fn order_three() -> G1Affine {
        G1Affine::new_unchecked(Fq::ZERO, Fq::from(2u64))
    }

    /// A point of G1 off the subgroup: a mapped field element before the cofactor is cleared
    pub(super) fn g1_off_subgroup(bytes: &[u8]) -> G1Affine {
        crate::bls12_381::eip2537::map_fp_to_g1(Fq::from_be_bytes_mod_order(bytes)).unwrap()
    }

    /// A point of G2 off the subgroup: a mapped field element before the cofactor is cleared
    pub(super) fn g2_off_subgroup(bytes: &[u8]) -> G2Affine {
        let (c0, c1) = bytes.split_at(bytes.len() / 2);
        let element = Fq2::new(
            Fq::from_be_bytes_mod_order(c0),
            Fq::from_be_bytes_mod_order(c1),
        );
        crate::bls12_381::eip2537::map_fp2_to_g2(element).unwrap()
    }

    #[test]
    #[should_panic(expected = "wrong")]
    fn wrong_g1_slope_cannot_change_addition() {
        let g = write_g1(G1Affine::generator());
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldDivision]);
        let _ = bls12_381_g1_add(&g, &g, Some(&mut oracle));
    }

    #[test]
    #[should_panic]
    fn forged_mapping_square_root_cannot_change_output() {
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldSqrt]);
        let mut encoded = [0u8; 64];
        serialize_fq_bytes(Fq::from(7u64), &mut encoded);
        let _ = bls12_381_map_fp_to_g1(&encoded, Some(&mut oracle));
    }

    /// Invalid encodings fail before any query
    #[test]
    fn invalid_inputs_are_rejected_before_any_query() {
        let g1 = write_g1(G1Affine::generator());
        let g2 = write_g2(G2Affine::generator());
        let (mut off_curve_g1, mut off_curve_g2) = (g1, g2);
        off_curve_g1[127] ^= 1;
        off_curve_g2[255] ^= 1;
        // a coordinate with a byte above the 48 of an element
        let (mut wide_g1, mut wide_g2) = (g1, g2);
        wide_g1[0] = 1;
        wide_g2[128] = 1;
        // the modulus, which is not canonical
        let mut modulus = [0u8; 64];
        let modulus_bytes = <Fq as PrimeField>::MODULUS.to_bytes_be();
        modulus[16..].copy_from_slice(&modulus_bytes[modulus_bytes.len() - 48..]);
        let mut wide_fp = [0u8; 64];
        wide_fp[15] = 1;

        let mut oracle = TapeOracle::new([]);
        for bad in [off_curve_g1, wide_g1] {
            for (a, b) in [(bad, g1), (g1, bad)] {
                let result = bls12_381_g1_add(&a, &b, Some(&mut oracle));
                assert_eq!(result, Err(Bls12Error::InvalidG1Point));
            }
        }
        for bad in [off_curve_g2, wide_g2] {
            for (a, b) in [(bad, g2), (g2, bad)] {
                let result = bls12_381_g2_add(&a, &b, Some(&mut oracle));
                assert_eq!(result, Err(Bls12Error::InvalidG2Point));
            }
        }
        let invalid = Bls12Error::InvalidFieldElement;
        for bad in [modulus, wide_fp] {
            assert_eq!(
                bls12_381_map_fp_to_g1(&bad, Some(&mut oracle)),
                Err(invalid)
            );
            for fp2 in [[bad, [0; 64]], [[0; 64], bad]] {
                let fp2: [u8; 128] = fp2.concat().try_into().unwrap();
                assert_eq!(
                    bls12_381_map_fp2_to_g2(&fp2, Some(&mut oracle)),
                    Err(invalid)
                );
            }
        }
        assert!(oracle.queries.is_empty());
    }

    /// Zero (the exceptional case of the SWU map) and small values, which take both of its
    /// square-root branches
    #[test]
    fn small_mappings_match() {
        for v in 0..4u8 {
            let mut fp = [0; 64];
            fp[63] = v;
            let (hinted, _) = record_hints(|oracle| bls12_381_map_fp_to_g1(&fp, Some(oracle)));
            assert!(hinted.is_ok());
            assert_eq!(hinted, bls12_381_map_fp_to_g1::<NoHints>(&fp, None));
            let mut fp2 = [0; 128];
            fp2[63] = v;
            fp2[127] = v;
            let (hinted, _) = record_hints(|oracle| bls12_381_map_fp2_to_g2(&fp2, Some(oracle)));
            assert!(hinted.is_ok());
            assert_eq!(hinted, bls12_381_map_fp2_to_g2::<NoHints>(&fp2, None));
        }
    }

    mod advice_matches_no_advice {
        use super::*;

        /// `b` is `k` times the generator, `a` itself or negated, the point at infinity, or a
        /// point off the subgroup (which addition takes), and maybe not on the curve
        #[test]
        fn add() {
            let (g1_cases, g2_cases) = (HintedCases::default(), HintedCases::default());
            proptest!(ProptestConfig::with_cases(64), |(ka: u64, kb: u64, mode in 0u8..6, bytes: [u8; 96], raw: [u8; 32])| {
                let a = (G1Affine::generator() * Fr::from(ka)).into_affine();
                let b = match mode {
                    0 => (G1Affine::generator() * Fr::from(kb)).into_affine(),
                    1 => a,
                    2 => -a,
                    3 => G1Affine::identity(),
                    4 => order_three(),
                    _ => g1_off_subgroup(&bytes[..48]),
                };
                let (a, mut b) = (write_g1(a), write_g1(b));
                if raw[0] < 16 {
                    b[96..].copy_from_slice(&raw);
                }
                for (a, b) in [(a, b), (b, a)] {
                    let hinted = g1_cases.record(|oracle| bls12_381_g1_add(&a, &b, Some(oracle)));
                    prop_assert_eq!(hinted, bls12_381_g1_add(&a, &b, None::<&mut NoHints>));
                }

                let a = (G2Affine::generator() * Fr::from(ka)).into_affine();
                let b = match mode {
                    0 | 4 => (G2Affine::generator() * Fr::from(kb)).into_affine(),
                    1 => a,
                    2 => -a,
                    3 => G2Affine::identity(),
                    _ => g2_off_subgroup(&bytes),
                };
                let (a, mut b) = (write_g2(a), write_g2(b));
                if raw[0] < 16 {
                    b[224..].copy_from_slice(&raw);
                }
                for (a, b) in [(a, b), (b, a)] {
                    let hinted = g2_cases.record(|oracle| bls12_381_g2_add(&a, &b, Some(oracle)));
                    prop_assert_eq!(hinted, bls12_381_g2_add(&a, &b, None::<&mut NoHints>));
                }
            });
            g1_cases.check();
            g2_cases.check();
        }

        /// Elements reduced from random bytes, or with random bytes in their low half (mostly not
        /// canonical), and `Fp2` elements with a zero `c1`
        #[test]
        fn maps() {
            let (g1_cases, g2_cases) = (HintedCases::default(), HintedCases::default());
            proptest!(ProptestConfig::with_cases(64), |(bytes: [[u8; 48]; 2], raw: [u8; 32], zero_c1: bool)| {
                let fp = |bytes: &[u8; 48], noncanonical: bool| {
                    let mut encoded = [0u8; 64];
                    serialize_fq_bytes(Fq::from_be_bytes_mod_order(bytes), &mut encoded);
                    if noncanonical {
                        encoded[16..48].copy_from_slice(&raw);
                    }
                    encoded
                };
                let c0 = fp(&bytes[0], raw[0] < 32);
                let hinted = g1_cases.record(|oracle| bls12_381_map_fp_to_g1(&c0, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_map_fp_to_g1(&c0, None::<&mut NoHints>));

                let c1 = if zero_c1 { [0u8; 64] } else { fp(&bytes[1], raw[0] >= 240) };
                let fp2: [u8; 128] = [c0, c1].concat().try_into().unwrap();
                let hinted = g2_cases.record(|oracle| bls12_381_map_fp2_to_g2(&fp2, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_map_fp2_to_g2(&fp2, None::<&mut NoHints>));
            });
            g1_cases.check();
            g2_cases.check();
        }
    }

    #[test]
    fn recorded_tape_replays() {
        let g1 = G1Affine::generator();
        let (a, b) = (write_g1(g1), write_g1((g1 + g1).into_affine()));
        let sum = assert_tape_replays!(|oracle| bls12_381_g1_add(&a, &b, Some(oracle)));
        assert_eq!(sum, bls12_381_g1_add(&a, &b, None::<&mut NoHints>));

        let g2 = G2Affine::generator();
        let (a, b) = (write_g2(g2), write_g2((g2 + g2).into_affine()));
        let sum = assert_tape_replays!(|oracle| bls12_381_g2_add(&a, &b, Some(oracle)));
        assert_eq!(sum, bls12_381_g2_add(&a, &b, None::<&mut NoHints>));

        let mut fp = [0u8; 64];
        serialize_fq_bytes(Fq::from(7u64), &mut fp);
        let point = assert_tape_replays!(|oracle| bls12_381_map_fp_to_g1(&fp, Some(oracle)));
        assert_eq!(point, bls12_381_map_fp_to_g1(&fp, None::<&mut NoHints>));

        let mut fp2 = [0u8; 128];
        serialize_fq2_bytes(Fq2::new(Fq::from(7u64), Fq::from(11u64)), &mut fp2);
        let point = assert_tape_replays!(|oracle| bls12_381_map_fp2_to_g2(&fp2, Some(oracle)));
        assert_eq!(point, bls12_381_map_fp2_to_g2(&fp2, None::<&mut NoHints>));
    }
}
