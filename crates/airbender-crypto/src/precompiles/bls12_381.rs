// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/mod.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/addition/mod.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/mappings/mod.rs
//! BLS12-381 G1/G2 addition and the field-to-curve maps (EIP-2537), and the MSMs and the
//! pairing check (in `msm` and `pairing`), with the EIP-2537 encoding of the points.

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

/// The errors of the endpoints: zksync-os's `Bls12PrecompileInterfaceError` without
/// `InvalidInputSize`, as the wrappers check the size of the input
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bls12Error {
    /// A G1 point is not a canonical encoding of a point of the curve (zksync-os `InvalidG1Point`)
    InvalidG1Point,
    /// A G2 point is not a canonical encoding of a point of the curve (zksync-os `InvalidG2Point`)
    InvalidG2Point,
    /// A point is not in the prime-order subgroup (zksync-os `PointNotInSubgroup`)
    NotInSubgroup,
    /// A field element is not canonical (zksync-os `InvalidFieldElement`)
    InvalidFieldElement,
}

// infallible, as scalars are no required to be canonical
#[cfg(any(feature = "alloc", test))]
fn parse_integer(input: &[u8; SCALAR_SERIALIZATION_LEN]) -> <Fr as PrimeField>::BigInt {
    let mut repr = [0u64; 4];
    for (dst, src) in repr.iter_mut().zip(input.as_rchunks::<8>().1.iter().rev()) {
        *dst = u64::from_be_bytes(*src);
    }
    crate::BigInt::new(repr)
}

// Parse functions without Fq/G1/G2 types in signatures
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

/// The existing encoding and on-curve validation, followed by a checked affine subgroup chain.
// On RV32 without `bigint_ops`, the divider is a stand-in that panics (see `unavailable`).
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

/// The sum of two G1 points, from their EIP-2537 encodings (the points need not be in the
/// subgroup). With an oracle, the addition works in affine coordinates, with the division of the
/// slope a checked hint. The encoding of the sum is the wrapper's (zksync-os `write_g1`).
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

/// The sum of two G2 points, from their EIP-2537 encodings (the points need not be in the
/// subgroup). With an oracle, the addition works in affine coordinates, with the division of the
/// slope a checked hint. The encoding of the sum is the wrapper's (zksync-os `write_g2`).
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

/// The G1 point of a field element, from its EIP-2537 encoding: the simplified SWU map, then
/// the cofactor cleared. With an oracle, the inversions and square roots of the map and the
/// divisions of the cofactor clearing are checked hints. The encoding of the point is the
/// wrapper's (zksync-os `write_g1`).
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

/// The G2 point of an `Fp2` element, from its EIP-2537 encoding: the simplified SWU map, then
/// the cofactor cleared. With an oracle, the inversions and square roots of the map and the
/// divisions of the cofactor clearing are checked hints. The encoding of the point is the
/// wrapper's (zksync-os `write_g2`).
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
    use crate::hints::{FieldHintOp, NoHints, RecordingHintOracle};
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};

    pub(super) fn honest_oracle() -> RecordingHintOracle {
        RecordingHintOracle::new()
    }

    /// The EIP-2537 encoding of a G1 point (zksync-os `write_g1`, without the output)
    pub(super) fn write_g1(el: G1Affine) -> [u8; G1_SERIALIZATION_LEN] {
        let mut buffer = [0u8; G1_SERIALIZATION_LEN];
        crate::bls12_381::eip2537::serialize_g1_bytes(el, &mut buffer);
        buffer
    }

    /// The EIP-2537 encoding of a G2 point (zksync-os `write_g2`, without the output)
    pub(super) fn write_g2(el: G2Affine) -> [u8; G2_SERIALIZATION_LEN] {
        let mut buffer = [0u8; G2_SERIALIZATION_LEN];
        crate::bls12_381::eip2537::serialize_g2_bytes(el, &mut buffer);
        buffer
    }

    /// zksync-os's `compare`: the endpoint without the oracle (which cannot query) and with the
    /// hints of an honest oracle give the same result, which must be `Ok`
    pub(super) fn compare<T: PartialEq + core::fmt::Debug>(
        result: Result<T, Bls12Error>,
        hinted: Result<T, Bls12Error>,
    ) -> T {
        assert_eq!(result, hinted);
        result.expect("valid input")
    }

    // from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/addition/tests/mod.rs

    #[test]
    fn additions_preserve_curve_and_infinity_cases() {
        // Addition permits torsion points: it must not apply an MSM subgroup check.
        let torsion = G1Affine::new_unchecked(Fq::ZERO, Fq::from(2u64));
        let g = G1Affine::generator();
        let o = G1Affine::identity();
        for (a, b) in [
            (g, g),
            (g, -g),
            (g, o),
            (o, g),
            (o, o),
            (torsion, torsion),
            (g, torsion),
        ] {
            let (a_bytes, b_bytes) = (write_g1(a), write_g1(b));
            let output = compare(
                bls12_381_g1_add::<NoHints>(&a_bytes, &b_bytes, None),
                bls12_381_g1_add(&a_bytes, &b_bytes, Some(&mut honest_oracle())),
            );
            let expected = (a + b).into_affine();
            assert_eq!(parse_g1(&write_g1(output)).unwrap(), expected);
        }
        let g = G2Affine::generator();
        let o = G2Affine::identity();
        for (a, b) in [(g, g), (g, -g), (g, o), (o, g), (o, o)] {
            let (a_bytes, b_bytes) = (write_g2(a), write_g2(b));
            let output = compare(
                bls12_381_g2_add::<NoHints>(&a_bytes, &b_bytes, None),
                bls12_381_g2_add(&a_bytes, &b_bytes, Some(&mut honest_oracle())),
            );
            assert_eq!(parse_g2(&write_g2(output)).unwrap(), (a + b).into_affine());
        }
    }

    #[test]
    #[should_panic(expected = "wrong")]
    fn wrong_g1_slope_cannot_change_addition() {
        let g = write_g1(G1Affine::generator());
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldDivision]);
        let _ = bls12_381_g1_add(&g, &g, Some(&mut oracle));
    }

    // from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/mappings/tests/mod.rs

    #[test]
    fn both_mapping_outputs_and_charges_match_without_hints() {
        for i in 0u64..=16 {
            let value = Fq::from(i * i + i);
            for value in [value, -value] {
                let mut encoded = [0u8; 64];
                serialize_fq_bytes(value, &mut encoded);
                let _ = compare(
                    bls12_381_map_fp_to_g1(&encoded, Some(&mut honest_oracle())),
                    bls12_381_map_fp_to_g1::<NoHints>(&encoded, None),
                );
                for value in [Fq2::new(value, Fq::ZERO), Fq2::new(value, Fq::from(i * 17))] {
                    let mut encoded = [0u8; 128];
                    serialize_fq2_bytes(value, &mut encoded);
                    let _ = compare(
                        bls12_381_map_fp2_to_g2(&encoded, Some(&mut honest_oracle())),
                        bls12_381_map_fp2_to_g2::<NoHints>(&encoded, None),
                    );
                }
            }
        }
    }

    #[test]
    #[should_panic]
    fn forged_mapping_square_root_cannot_change_output() {
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldSqrt]);
        let mut encoded = [0u8; 64];
        serialize_fq_bytes(Fq::from(7u64), &mut encoded);
        let _ = bls12_381_map_fp_to_g1(&encoded, Some(&mut oracle));
    }

    // new tests

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

    /// Invalid encodings fail with the errors of zksync-os, before any query
    #[test]
    fn invalid_inputs_are_rejected_before_any_query() {
        let g1 = write_g1(G1Affine::generator());
        let g2 = write_g2(G2Affine::generator());
        let mut off_curve_g1 = g1;
        off_curve_g1[127] ^= 1;
        let mut off_curve_g2 = g2;
        off_curve_g2[255] ^= 1;
        // a coordinate with a byte above the 48 of an element
        let mut wide_g1 = g1;
        wide_g1[0] = 1;
        let mut wide_g2 = g2;
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
                assert_eq!(
                    bls12_381_g1_add(&a, &b, Some(&mut oracle)),
                    Err(Bls12Error::InvalidG1Point)
                );
                assert_eq!(
                    bls12_381_g1_add::<NoHints>(&a, &b, None),
                    Err(Bls12Error::InvalidG1Point)
                );
            }
        }
        for bad in [off_curve_g2, wide_g2] {
            for (a, b) in [(bad, g2), (g2, bad)] {
                assert_eq!(
                    bls12_381_g2_add(&a, &b, Some(&mut oracle)),
                    Err(Bls12Error::InvalidG2Point)
                );
                assert_eq!(
                    bls12_381_g2_add::<NoHints>(&a, &b, None),
                    Err(Bls12Error::InvalidG2Point)
                );
            }
        }
        for bad in [modulus, wide_fp] {
            assert_eq!(
                bls12_381_map_fp_to_g1(&bad, Some(&mut oracle)),
                Err(Bls12Error::InvalidFieldElement)
            );
            assert_eq!(
                bls12_381_map_fp_to_g1::<NoHints>(&bad, None),
                Err(Bls12Error::InvalidFieldElement)
            );
            let zero = [0u8; 64];
            for fp2 in [[bad, zero], [zero, bad]] {
                let fp2: [u8; 128] = fp2.concat().try_into().unwrap();
                assert_eq!(
                    bls12_381_map_fp2_to_g2(&fp2, Some(&mut oracle)),
                    Err(Bls12Error::InvalidFieldElement)
                );
                assert_eq!(
                    bls12_381_map_fp2_to_g2::<NoHints>(&fp2, None),
                    Err(Bls12Error::InvalidFieldElement)
                );
            }
        }
        assert!(oracle.queries.is_empty());
    }

    /// The subgroup checks of the MSM and pairing inputs: the hinted chain decides as
    /// the check without hints
    #[test]
    fn subgroup_checks_with_and_without_hints_agree() {
        let torsion = G1Affine::new_unchecked(Fq::ZERO, Fq::from(2u64));
        for (point, in_subgroup) in [
            (G1Affine::generator(), true),
            (G1Affine::identity(), true),
            (torsion, false),
            (g1_off_subgroup(&[7; 48]), false),
        ] {
            let bytes = write_g1(point);
            let expected = if in_subgroup {
                Ok(point)
            } else {
                Err(Bls12Error::NotInSubgroup)
            };
            assert_eq!(parse_g1_with_subgroup_check(&bytes), expected);
            assert_eq!(
                parse_g1_with_subgroup_check_with_oracle::<NoHints>(&bytes, None),
                expected
            );
            assert_eq!(
                parse_g1_with_subgroup_check_with_oracle(&bytes, Some(&mut honest_oracle())),
                expected
            );
        }
        for (point, in_subgroup) in [
            (G2Affine::generator(), true),
            (G2Affine::identity(), true),
            (g2_off_subgroup(&[7; 96]), false),
        ] {
            let bytes = write_g2(point);
            let expected = if in_subgroup {
                Ok(point)
            } else {
                Err(Bls12Error::NotInSubgroup)
            };
            assert_eq!(parse_g2_with_subgroup_check(&bytes), expected);
            assert_eq!(
                parse_g2_with_subgroup_check_with_oracle::<NoHints>(&bytes, None),
                expected
            );
            assert_eq!(
                parse_g2_with_subgroup_check_with_oracle(&bytes, Some(&mut honest_oracle())),
                expected
            );
        }
        let mut off_curve = write_g1(G1Affine::generator());
        off_curve[127] ^= 1;
        assert_eq!(
            parse_g1_with_subgroup_check_with_oracle(&off_curve, Some(&mut TapeOracle::new([]))),
            Err(Bls12Error::InvalidG1Point)
        );
    }

    mod advice_matches_no_advice {
        use super::*;
        use crate::precompiles::HintedCases;

        /// The G1 point of `mode`, from `a` (`k` times the generator) and `bytes`: a multiple of
        /// the generator, `a` itself or negated, the point at infinity, or a point off the
        /// subgroup (which addition takes)
        fn g1_point(a: G1Affine, k: u64, mode: u8, bytes: &[u8; 48]) -> G1Affine {
            match mode {
                0 => (G1Affine::generator() * Fr::from(k)).into_affine(),
                1 => a,
                2 => -a,
                3 => G1Affine::identity(),
                4 => G1Affine::new_unchecked(Fq::ZERO, Fq::from(2u64)),
                _ => g1_off_subgroup(bytes),
            }
        }

        fn g2_point(a: G2Affine, k: u64, mode: u8, bytes: &[u8; 96]) -> G2Affine {
            match mode {
                0 | 4 => (G2Affine::generator() * Fr::from(k)).into_affine(),
                1 => a,
                2 => -a,
                3 => G2Affine::identity(),
                _ => g2_off_subgroup(bytes),
            }
        }

        #[test]
        fn g1_add() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(64), |(ka: u64, kb: u64, mode in 0u8..6, bytes: [u8; 48], raw: [u8; 32])| {
                let a = (G1Affine::generator() * Fr::from(ka)).into_affine();
                let b = g1_point(a, kb, mode, &bytes);
                let mut b = write_g1(b);
                if raw[0] < 16 {
                    // mostly not a point of the curve
                    b[96..].copy_from_slice(&raw);
                }
                let a = write_g1(a);
                for (a, b) in [(a, b), (b, a)] {
                    let hinted = hinted_cases.record(|oracle| bls12_381_g1_add(&a, &b, Some(oracle)));
                    prop_assert_eq!(hinted, bls12_381_g1_add(&a, &b, None::<&mut NoHints>));
                }
            });
            hinted_cases.check();
        }

        #[test]
        fn g2_add() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(64), |(ka: u64, kb: u64, mode in 0u8..6, bytes: [[u8; 48]; 2], raw: [u8; 32])| {
                let a = (G2Affine::generator() * Fr::from(ka)).into_affine();
                let b = g2_point(a, kb, mode, &bytes.concat().try_into().unwrap());
                let mut b = write_g2(b);
                if raw[0] < 16 {
                    // mostly not a point of the curve
                    b[224..].copy_from_slice(&raw);
                }
                let a = write_g2(a);
                for (a, b) in [(a, b), (b, a)] {
                    let hinted = hinted_cases.record(|oracle| bls12_381_g2_add(&a, &b, Some(oracle)));
                    prop_assert_eq!(hinted, bls12_381_g2_add(&a, &b, None::<&mut NoHints>));
                }
            });
            hinted_cases.check();
        }

        /// The encoding of the element of `bytes` (reduced), or with `raw` in place of its low
        /// half (mostly not canonical)
        fn fp(bytes: &[u8; 48], raw: Option<&[u8; 32]>) -> [u8; 64] {
            let mut encoded = [0u8; 64];
            serialize_fq_bytes(Fq::from_be_bytes_mod_order(bytes), &mut encoded);
            if let Some(raw) = raw {
                encoded[16..48].copy_from_slice(raw);
            }
            encoded
        }

        #[test]
        fn map_fp_to_g1() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(64), |(bytes: [u8; 48], raw: [u8; 32])| {
                let fp = fp(&bytes, (raw[0] < 32).then_some(&raw));
                let hinted = hinted_cases.record(|oracle| bls12_381_map_fp_to_g1(&fp, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_map_fp_to_g1(&fp, None::<&mut NoHints>));
            });
            hinted_cases.check();
        }

        #[test]
        fn map_fp2_to_g2() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(64), |(bytes: [[u8; 48]; 2], raw: [u8; 32], zero_c1: bool)| {
                let c0 = fp(&bytes[0], (raw[0] < 16).then_some(&raw));
                let c1 = if zero_c1 { [0u8; 64] } else { fp(&bytes[1], (raw[0] >= 240).then_some(&raw)) };
                let fp2: [u8; 128] = [c0, c1].concat().try_into().unwrap();
                let hinted = hinted_cases.record(|oracle| bls12_381_map_fp2_to_g2(&fp2, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_map_fp2_to_g2(&fp2, None::<&mut NoHints>));
            });
            hinted_cases.check();
        }
    }

    /// The guest, which reads its answers off the recorded tape, gets the same result and takes
    /// the whole tape and no more. The bls12-381 elements of test builds have the representation
    /// of the guest, so the tape is also the answers this process reads.
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
