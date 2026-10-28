//! Inverses and square roots in the bn254 and bls12-381 fields from oracle hints, each checked
//! with one multiplication or squaring.
//!
//! A wrong hint fails an assertion: hints come from the prover's own oracle, so a bad one is a
//! broken prover rather than a case to recover from.

pub use self::bls12_381_extension::{
    bls12_381_fq2_inverse, bls12_381_fq2_sqrt, Bls12381Fq2Divider,
};

use super::answer::HintAnswer;
use super::oracle::HintOracle;
use super::request::{query_field_hint, query_field_hint_into, FieldHintOp};
use ark_ff::{Field, PrimeField, Zero};
use core::mem::MaybeUninit;

/// `a^-1` from a hint, `None` for a zero `a`
///
/// # Safety
///
/// `F` must be the operand type of `op` (see `send_field_hint_query`).
unsafe fn inverse_from_hint<O: HintOracle, F: Field + HintAnswer>(
    oracle: &mut O,
    op: FieldHintOp,
    a: &F,
) -> Option<F> {
    if a.is_zero() {
        return None;
    }
    // SAFETY: `a` is a value of the operand type of `op` (the caller's obligation)
    let inverse: F = unsafe { query_field_hint(oracle, op, a) };
    let mut product = *a;
    product *= &inverse;
    assert!(product.is_one(), "the field inverse hint is wrong");
    Some(inverse)
}

/// `a^-1` in the bn254 base field, `None` for a zero `a`
pub fn bn254_fq_inverse<O: HintOracle>(
    oracle: &mut O,
    a: &crate::bn254::Fq,
) -> Option<crate::bn254::Fq> {
    // SAFETY: `a` is a bn254 `Fq`, the operand type of the bn254 base field inverse
    unsafe { inverse_from_hint(oracle, FieldHintOp::Bn254BaseFieldInverse, a) }
}

/// Divisions in the bn254 base field from hints, for the curve arithmetic in affine coordinates
/// (`crate::affine_glv`)
pub struct Bn254FqDivider<'a, O: HintOracle> {
    oracle: &'a mut O,
}

impl<'a, O: HintOracle> Bn254FqDivider<'a, O> {
    pub fn new(oracle: &'a mut O) -> Self {
        Self { oracle }
    }
}

impl<O: HintOracle> crate::affine_glv::Divider<crate::bn254::Fq> for Bn254FqDivider<'_, O> {
    #[inline(always)]
    fn divide<'q>(
        &mut self,
        fraction: &mut [crate::bn254::Fq; 2],
        quotient: &'q mut MaybeUninit<crate::bn254::Fq>,
    ) -> &'q mut crate::bn254::Fq {
        // SAFETY: `[bn254::Fq; 2]` is the operand type of the bn254 base field division
        let quotient = unsafe {
            query_field_hint_into(
                self.oracle,
                FieldHintOp::Bn254BaseFieldDivision,
                &*fraction,
                quotient,
            )
        };
        // quotient * denominator == numerator: the denominator is non-zero (the caller's
        // obligation), so only the quotient passes. The fraction is ours to overwrite.
        let [numerator, denominator] = fraction;
        *denominator *= &*quotient;
        *denominator -= &*numerator;
        assert!(denominator.is_zero(), "the field division hint is wrong");
        quotient
    }
}

/// Divisions in the bls12-381 base field from hints, as [`Bn254FqDivider`]
pub struct Bls12381FqDivider<'a, O: HintOracle> {
    oracle: &'a mut O,
}

impl<'a, O: HintOracle> Bls12381FqDivider<'a, O> {
    pub fn new(oracle: &'a mut O) -> Self {
        Self { oracle }
    }
}

impl<O: HintOracle> crate::affine_glv::Divider<crate::bls12_381::Fq> for Bls12381FqDivider<'_, O> {
    #[inline(always)]
    fn divide<'q>(
        &mut self,
        fraction: &mut [crate::bls12_381::Fq; 2],
        quotient: &'q mut MaybeUninit<crate::bls12_381::Fq>,
    ) -> &'q mut crate::bls12_381::Fq {
        // SAFETY: `[bls12_381::Fq; 2]` is the operand type of the bls12-381 base field division
        let quotient = unsafe {
            query_field_hint_into(
                self.oracle,
                FieldHintOp::Bls12381BaseFieldDivision,
                &*fraction,
                quotient,
            )
        };
        // as for bn254: only the quotient passes
        let [numerator, denominator] = fraction;
        *denominator *= &*quotient;
        *denominator -= &*numerator;
        assert!(denominator.is_zero(), "the field division hint is wrong");
        quotient
    }
}

/// `f^-1` in the bn254 degree-12 extension field, `None` for a zero `f`
pub fn bn254_fq12_inverse<O: HintOracle>(
    oracle: &mut O,
    f: &crate::bn254::Fq12,
) -> Option<crate::bn254::Fq12> {
    // SAFETY: `f` is a bn254 `Fq12`, the operand type of the bn254 `Fq12` inverse
    unsafe { inverse_from_hint(oracle, FieldHintOp::Bn254Fq12Inverse, f) }
}

/// `a^-1` in the bls12-381 base field, `None` for a zero `a`
pub fn bls12_381_fq_inverse<O: HintOracle>(
    oracle: &mut O,
    a: &crate::bls12_381::Fq,
) -> Option<crate::bls12_381::Fq> {
    // SAFETY: `a` is a bls12-381 `Fq`, the operand type of the bls12-381 base field inverse
    unsafe { inverse_from_hint(oracle, FieldHintOp::Bls12381BaseFieldInverse, a) }
}

/// `f^-1` in the bls12-381 degree-12 extension field, `None` for a zero `f`
pub fn bls12_381_fq12_inverse<O: HintOracle>(
    oracle: &mut O,
    f: &crate::bls12_381::Fq12,
) -> Option<crate::bls12_381::Fq12> {
    // SAFETY: `f` is a bls12-381 `Fq12`, the operand type of the bls12-381 `Fq12` inverse
    unsafe { inverse_from_hint(oracle, FieldHintOp::Bls12381Fq12Inverse, f) }
}

// The square root check relies on `-1` being a quadratic non-residue, i.e. on `p = 3 mod 4`
const _: () = assert!(<crate::bls12_381::Fq as PrimeField>::MODULUS.0[0] & 3 == 3);
const _: () = assert!(<crate::bn254::Fq as PrimeField>::MODULUS.0[0] & 3 == 3);

/// A square root of `a` in the bls12-381 base field from a hint, `None` if `a` is not a
/// square. Which of the two roots is returned is up to the oracle.
pub fn bls12_381_fq_sqrt<O: HintOracle>(
    oracle: &mut O,
    a: &crate::bls12_381::Fq,
) -> Option<crate::bls12_381::Fq> {
    use crate::bls12_381::Fq;
    if a.is_zero() {
        return Some(Fq::zero());
    }
    // SAFETY: `a` is a bls12-381 `Fq`, the operand type of the bls12-381 base field square root
    let (candidate, is_non_residue): (Fq, bool) =
        unsafe { query_field_hint(oracle, FieldHintOp::Bls12381BaseFieldSqrt, a) };
    let mut square = candidate;
    square.square_in_place();
    if is_non_residue {
        // `candidate² = -a` proves that `a` is not a square, as `-1` is not one
        square += a;
        assert!(
            square.is_zero(),
            "the non-residue proof of the square root hint is wrong"
        );
        None
    } else {
        assert!(square == *a, "the square root hint is wrong");
        Some(candidate)
    }
}

/// Checked BLS12-381 extension-field operations using the existing base-field hints.
mod bls12_381_extension {
    use super::{bls12_381_fq_inverse, bls12_381_fq_sqrt};
    use crate::bls12_381::{Fq, Fq2};
    use crate::hints::HintOracle;
    use ark_ff::{AdditiveGroup, Field, PrimeField, Zero};
    use core::mem::MaybeUninit;

    /// Inverse in Fq\[i\], where i² = -1. The norm vanishes only for the zero element,
    /// because -1 is not a square in the BLS12-381 base field.
    pub fn bls12_381_fq2_inverse<O: HintOracle>(oracle: &mut O, value: &Fq2) -> Option<Fq2> {
        let norm = value.c0.square() + value.c1.square();
        let inverse = bls12_381_fq_inverse(oracle, &norm)?;
        Some(Fq2::new(value.c0 * inverse, -(value.c1 * inverse)))
    }

    /// Extension-field division with a checked hint for the inverse of the denominator's norm.
    pub struct Bls12381Fq2Divider<'a, O: HintOracle> {
        oracle: &'a mut O,
    }

    impl<'a, O: HintOracle> Bls12381Fq2Divider<'a, O> {
        pub fn new(oracle: &'a mut O) -> Self {
            Self { oracle }
        }
    }

    impl<O: HintOracle> crate::affine_glv::Divider<Fq2> for Bls12381Fq2Divider<'_, O> {
        fn divide<'q>(
            &mut self,
            fraction: &mut [Fq2; 2],
            quotient: &'q mut MaybeUninit<Fq2>,
        ) -> &'q mut Fq2 {
            // the `Divider` caller guarantees a non-zero denominator
            let inverse = bls12_381_fq2_inverse(self.oracle, &fraction[1])
                .expect("the affine division denominator is nonzero");
            quotient.write(fraction[0] * inverse)
        }
    }

    /// A square root in Fq2, `None` if there is none (both checked by the base-field hints).
    /// The root's sign is unspecified.
    pub fn bls12_381_fq2_sqrt<O: HintOracle>(oracle: &mut O, value: &Fq2) -> Option<Fq2> {
        if value.c1.is_zero() {
            // Every base-field element has a square root in Fq2. If a is a nonsquare
            // in Fq, -a is a square, and (sqrt(-a) * i)² = a.
            return match bls12_381_fq_sqrt(oracle, &value.c0) {
                Some(root) => Some(Fq2::new(root, Fq::ZERO)),
                None => bls12_381_fq_sqrt(oracle, &-value.c0).map(|root| Fq2::new(Fq::ZERO, root)),
            };
        }
        // An element of Fq2 is a square iff its norm is a square in Fq.
        let norm = value.c0.square() + value.c1.square();
        let norm_root = bls12_381_fq_sqrt(oracle, &norm)?;
        let half = -Fq::from_bigint(Fq::MODULUS_MINUS_ONE_DIV_TWO)
            .expect("half the modulus minus one is canonical");
        // The product of these two candidates is -c1²/4, a nonsquare. Exactly
        // one candidate has a square root, and neither is zero when c1 != 0.
        let first = (value.c0 + norm_root) * half;
        let root = match bls12_381_fq_sqrt(oracle, &first) {
            Some(root) => root,
            None => bls12_381_fq_sqrt(oracle, &((value.c0 - norm_root) * half))?,
        };
        let denominator = root.double();
        let inverse = bls12_381_fq_inverse(oracle, &denominator)
            .expect("a non-real square has a nonzero real root component");
        let result = Fq2::new(root, value.c1 * inverse);
        assert_eq!(
            result.square(),
            *value,
            "the extension-field square root is wrong"
        );
        Some(result)
    }

    #[cfg(all(test, target_pointer_width = "64"))]
    mod tests {
        use super::*;
        use crate::affine_glv::Divider;
        use crate::hints::host::RecordingHintOracle;
        use crate::hints::{test_support::lying_oracle, FieldHintOp};

        /// Zero, real, imaginary and mixed elements, squares and non-squares (from `i = 7`)
        fn inputs() -> Vec<Fq2> {
            let mut values = vec![Fq2::ZERO, Fq2::ONE, -Fq2::ONE];
            for i in 1u64..=8 {
                values.push(Fq2::new(Fq::from(i), Fq::ZERO));
                values.push(Fq2::new(Fq::ZERO, Fq::from(i)));
                values.push(Fq2::new(Fq::from(i * 17), Fq::from(i * i + 5)));
            }
            values
        }

        #[test]
        fn inverses_and_divisions_match_field_arithmetic() {
            let mut oracle = RecordingHintOracle::new();
            for value in inputs() {
                assert_eq!(bls12_381_fq2_inverse(&mut oracle, &value), value.inverse());
                if !value.is_zero() {
                    let numerator = Fq2::new(Fq::from(19u64), Fq::from(31u64));
                    let mut slot = MaybeUninit::uninit();
                    let mut fraction = [numerator, value];
                    let actual =
                        Bls12381Fq2Divider::new(&mut oracle).divide(&mut fraction, &mut slot);
                    assert_eq!(*actual * value, numerator);
                }
            }
        }

        #[test]
        fn roots_and_nonresidues_match_field_arithmetic() {
            let mut oracle = RecordingHintOracle::new();
            let mut nonsquares = 0;
            for value in inputs() {
                for value in [value, value.square()] {
                    let root = bls12_381_fq2_sqrt(&mut oracle, &value);
                    assert_eq!(root.is_some(), value.sqrt().is_some(), "{value:?}");
                    match root {
                        Some(root) => assert_eq!(root.square(), value),
                        None => nonsquares += 1,
                    }
                }
            }
            assert!(nonsquares > 0);
        }

        #[test]
        #[should_panic(expected = "the field inverse hint is wrong")]
        fn false_inverse_hint_is_rejected() {
            let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldInverse]);
            let _ = bls12_381_fq2_inverse(&mut oracle, &Fq2::new(Fq::from(2u64), Fq::from(3u64)));
        }

        #[test]
        #[should_panic]
        fn false_sqrt_hint_is_rejected() {
            let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldSqrt]);
            let _ = bls12_381_fq2_sqrt(&mut oracle, &Fq2::new(Fq::from(2u64), Fq::from(3u64)));
        }
    }
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::affine_glv::Divider;
    use crate::hints::host::RecordingHintOracle;
    use crate::hints::test_support::lying_oracle;
    use crate::hints::HintTarget;
    use crate::{bls12_381, bn254};
    use ark_ff::{BigInteger, Field, One, PrimeField, Zero};
    use core::mem::MaybeUninit;

    /// Deterministic field elements spread over the field: powers of a large seed
    fn elements<F: PrimeField>(count: usize) -> Vec<F> {
        let seed = F::from(0x9e37_79b9_7f4a_7c15u64) * F::from(0xd1b5_4a32_d192_ed03u64);
        let mut acc = seed;
        (0..count)
            .map(|_| {
                acc = acc * seed + F::from(3u64);
                acc
            })
            .collect()
    }

    fn extension_element<E: Field>() -> E {
        let base = elements::<E::BasePrimeField>(E::extension_degree() as usize);
        E::from_base_prime_field_elems(base).unwrap()
    }

    fn answer_words<T: HintAnswer>(value: &T, target: HintTarget) -> Vec<u32> {
        let mut response = Vec::new();
        value.write(target, &mut response);
        response
    }

    fn limb_words(limbs: &[u64]) -> Vec<u32> {
        limbs
            .iter()
            .flat_map(|limb| [*limb as u32, (*limb >> 32) as u32])
            .collect()
    }

    #[test]
    fn answers_follow_the_target() {
        // bn254: `R = 2^256` on every target
        let a = elements::<bn254::Fq>(1)[0];
        let expected = limb_words(
            (a * bn254::Fq::from(2u64).pow([256]))
                .into_bigint()
                .as_ref(),
        );
        assert_eq!(answer_words(&a, HintTarget::Native), expected);
        assert_eq!(answer_words(&a, HintTarget::Guest), expected);
        // bls12-381: `R = 2^(64 limbs)` with this build's limbs natively (8 in this test build),
        // `2^512` on the guest; 6 wire limbs on both
        let native_bits = 64 * <<bls12_381::Fq as PrimeField>::BigInt as BigInteger>::NUM_LIMBS;
        let a = elements::<bls12_381::Fq>(1)[0];
        let r = |bits: u64| bls12_381::Fq::from(2u64).pow([bits]);
        let wire_words = |x: bls12_381::Fq| limb_words(&x.into_bigint().as_ref()[..6]);
        let guest = wire_words(a * r(512));
        assert_eq!(
            answer_words(&a, HintTarget::Native),
            wire_words(a * r(native_bits as u64))
        );
        assert_eq!(answer_words(&a, HintTarget::Guest), guest);
        // extension components in order, `c0` first
        let f = bls12_381::Fq2::new(a, -a);
        assert_eq!(
            answer_words(&f, HintTarget::Guest),
            [guest, wire_words((-a) * r(512))].concat()
        );
    }

    #[test]
    fn inverses_match_the_field() {
        let mut oracle = RecordingHintOracle::new();
        for a in elements::<bn254::Fq>(3) {
            assert_eq!(bn254_fq_inverse(&mut oracle, &a), a.inverse());
        }
        for a in elements::<bls12_381::Fq>(3) {
            assert_eq!(bls12_381_fq_inverse(&mut oracle, &a), a.inverse());
        }
        let f = extension_element::<bn254::Fq12>();
        assert_eq!(bn254_fq12_inverse(&mut oracle, &f), f.inverse());
        let f = extension_element::<bls12_381::Fq12>();
        assert_eq!(bls12_381_fq12_inverse(&mut oracle, &f), f.inverse());
        let one = bls12_381::Fq::one();
        assert_eq!(bls12_381_fq_inverse(&mut oracle, &one), Some(one));
        // zero has no inverse, and no hint is asked for
        assert_eq!(bn254_fq_inverse(&mut oracle, &bn254::Fq::zero()), None);
        assert_eq!(bn254_fq12_inverse(&mut oracle, &bn254::Fq12::zero()), None);
        assert_eq!(
            bls12_381_fq_inverse(&mut oracle, &bls12_381::Fq::zero()),
            None
        );
        assert_eq!(
            bls12_381_fq12_inverse(&mut oracle, &bls12_381::Fq12::zero()),
            None
        );
    }

    #[test]
    fn square_roots_match_the_field() {
        let mut oracle = RecordingHintOracle::new();
        let mut squares = 0;
        for a in elements::<bls12_381::Fq>(10) {
            let hinted = bls12_381_fq_sqrt(&mut oracle, &a);
            assert_eq!(hinted.is_some(), a.sqrt().is_some());
            if let Some(root) = hinted {
                squares += 1;
                assert_eq!(root.square(), a);
            }
        }
        assert!(squares > 0 && squares < 10, "{squares} squares of 10");
        let zero = bls12_381::Fq::zero();
        assert_eq!(bls12_381_fq_sqrt(&mut oracle, &zero), Some(zero));
    }

    #[test]
    fn divisions_are_the_quotient() {
        let mut oracle = RecordingHintOracle::new();
        let [n, d] = [bn254::Fq::from(7u64), elements::<bn254::Fq>(1)[0]];
        let mut quotient = MaybeUninit::uninit();
        let q = *Bn254FqDivider::new(&mut oracle).divide(&mut [n, d], &mut quotient);
        assert_eq!(q * d, n);
        let [n, d] = [bls12_381::Fq::from(7u64), elements::<bls12_381::Fq>(1)[0]];
        let mut quotient = MaybeUninit::uninit();
        let q = *Bls12381FqDivider::new(&mut oracle).divide(&mut [n, d], &mut quotient);
        assert_eq!(q * d, n);
    }

    #[test]
    #[should_panic(expected = "inverse hint is wrong")]
    fn wrong_bn254_inverse_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Bn254BaseFieldInverse]);
        let _ = bn254_fq_inverse(&mut oracle, &bn254::Fq::from(7u64));
    }

    #[test]
    #[should_panic(expected = "inverse hint is wrong")]
    fn wrong_bls12_381_fq12_inverse_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381Fq12Inverse]);
        let _ = bls12_381_fq12_inverse(&mut oracle, &extension_element());
    }

    #[test]
    #[should_panic(expected = "division hint is wrong")]
    fn wrong_bls12_381_division_is_rejected() {
        let mut fraction = [bls12_381::Fq::from(7u64), bls12_381::Fq::from(5u64)];
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldDivision]);
        Bls12381FqDivider::new(&mut oracle).divide(&mut fraction, &mut MaybeUninit::uninit());
    }

    #[test]
    #[should_panic(expected = "division hint is wrong")]
    fn wrong_bn254_division_is_rejected() {
        let mut fraction = [bn254::Fq::from(7u64), bn254::Fq::from(5u64)];
        let mut oracle = lying_oracle(&[FieldHintOp::Bn254BaseFieldDivision]);
        Bn254FqDivider::new(&mut oracle).divide(&mut fraction, &mut MaybeUninit::uninit());
    }

    #[test]
    #[should_panic(expected = "square root hint is wrong")]
    fn wrong_square_root_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Bls12381BaseFieldSqrt]);
        let _ = bls12_381_fq_sqrt(&mut oracle, &bls12_381::Fq::from(4u64));
    }
}
