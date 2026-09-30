//! `na * a + ng * g` in affine coordinates, for hooks with a cheap field division
//! (`Secp256k1Hooks::FE_DIVIDE_IS_CHEAP`: the quotient comes as a hint and is checked with one
//! multiplication).
//!
//! An affine addition or doubling is its slope `λ` (a division), then `x3 = λ² - x1 - x2` and
//! `y3 = λ (x1 - x3) - y1`: one squaring and one multiplication besides the division, where the
//! Jacobian formulas take 7 multiplications for a doubling and 11 for a mixed addition. The
//! scalars are handled as in `ecmult_wnaf` (the endomorphism split of `na`, the split of `ng`
//! in halves, wNAF digits, the static tables of the generator).
//!
//! A doubling followed by an addition, `2 r + p`, is computed as `(r + p) + r` without the `y`
//! of the intermediate point: with `λ1` the slope of `r + p` and `x_t` the `x` of that sum, the
//! slope of the second addition is `-(λ1 + 2 y_r / (x_t - x_r))`. That is two divisions, two
//! squarings and one multiplication.
//!
//! All the formulas are complete here: the inputs of `ecrecover` are chosen by the caller of
//! the precompile, so an accumulator equal to the point to add, or to its negation, is
//! reachable. Every division has its denominator tested, and a zero sends the step to the case
//! it stands for (a doubling, or the point at infinity).

use core::mem::MaybeUninit;

use super::{
    context::{ECMultContext, ECMULT_TABLE_SIZE_A, WINDOW_A, WINDOW_G, WNAF_BITS},
    field::FieldElement,
    hooks::Secp256k1Hooks,
    points::{Affine, AffineStorage},
    recover::{table_index, wnaf, WnafDigit},
    scalars::Scalar,
};

/// Initializes `dst` with a copy of `src`
#[inline(always)]
fn write_copy<'a>(
    dst: &'a mut MaybeUninit<FieldElement>,
    src: &FieldElement,
) -> &'a mut FieldElement {
    #[cfg(feature = "bigint_ops")]
    {
        FieldElement::write_copy(dst, src)
    }
    #[cfg(not(feature = "bigint_ops"))]
    {
        dst.write(*src)
    }
}

#[inline(always)]
fn assign(dst: &mut FieldElement, src: &FieldElement) {
    #[cfg(feature = "bigint_ops")]
    {
        dst.copy_from(src);
    }
    #[cfg(not(feature = "bigint_ops"))]
    {
        *dst = *src;
    }
}

#[inline(always)]
fn triple(x: &mut FieldElement) {
    #[cfg(feature = "bigint_ops")]
    {
        x.triple_in_place();
    }
    #[cfg(not(feature = "bigint_ops"))]
    {
        x.mul_int_in_place(3);
    }
}

/// `x = minuend - x`
#[inline(always)]
fn subtract_from(x: &mut FieldElement, minuend: &FieldElement) {
    #[cfg(feature = "bigint_ops")]
    {
        x.sub_and_negate_in_place(minuend);
    }
    #[cfg(not(feature = "bigint_ops"))]
    {
        let mut difference = *minuend;
        difference.sub_in_place(x);
        *x = difference;
    }
}

/// The representations with magnitudes are kept normalized between the steps; the delegated
/// one needs nothing
#[inline(always)]
fn settle(x: &mut FieldElement) {
    #[cfg(feature = "bigint_ops")]
    {
        let _ = x;
    }
    #[cfg(not(feature = "bigint_ops"))]
    {
        x.normalize_in_place();
    }
}

/// A numerator and a denominator, adjacent as `Secp256k1Hooks::fe_divide` takes them
struct Fraction([MaybeUninit<FieldElement>; 2]);

impl Fraction {
    #[inline(always)]
    fn uninit() -> Self {
        Self([const { MaybeUninit::uninit() }; 2])
    }

    /// The places of the numerator and of the denominator
    #[inline(always)]
    fn parts(
        &mut self,
    ) -> (
        &mut MaybeUninit<FieldElement>,
        &mut MaybeUninit<FieldElement>,
    ) {
        let [numerator, denominator] = &mut self.0;
        (numerator, denominator)
    }

    /// The quotient into `quotient`. The parts are to be written again before the next
    /// division: the hook may overwrite them.
    ///
    /// # Safety
    /// Both parts must be initialized, and the denominator must not be zero.
    #[inline(always)]
    unsafe fn divide<'a, H: Secp256k1Hooks>(
        &mut self,
        quotient: &'a mut MaybeUninit<FieldElement>,
        hooks: &mut H,
    ) -> &'a mut FieldElement {
        // SAFETY: `MaybeUninit<T>` has the layout of `T`, and the parts are initialized
        let fraction = unsafe { &mut *(&mut self.0 as *mut [MaybeUninit<FieldElement>; 2]).cast() };
        let [numerator, denominator]: &mut [FieldElement; 2] = fraction;
        settle(numerator);
        settle(denominator);
        hooks.fe_divide(fraction, quotient)
    }
}

/// The rest of an addition of a point with the coordinate `x2` (a doubling if `None`) to `r`,
/// given the slope: `x3 = slope² - x1 - x2`, `y3 = slope (x1 - x3) - y1`
#[inline(always)]
fn apply_slope(r: &mut Affine, slope: &FieldElement, x2: Option<&FieldElement>) {
    let mut x3 = MaybeUninit::uninit();
    let x3 = write_copy(&mut x3, slope);
    x3.square_in_place();
    x3.sub_in_place(&r.x);
    x3.sub_in_place(x2.unwrap_or(&r.x));

    r.x.sub_in_place(x3);
    r.x.mul_in_place(slope);
    subtract_from(&mut r.y, &r.x);
    assign(&mut r.x, x3);
    settle(&mut r.x);
    settle(&mut r.y);
}

/// `r = 2 r`
fn double<H: Secp256k1Hooks>(r: &mut Affine, hooks: &mut H) {
    if r.infinity {
        return;
    }
    // the slope is 3 x² / 2 y
    let mut fraction = Fraction::uninit();
    let (numerator, denominator) = fraction.parts();
    let numerator = write_copy(numerator, &r.x);
    numerator.square_in_place();
    triple(numerator);
    let denominator = write_copy(denominator, &r.y);
    denominator.double_in_place();
    if denominator.is_zero() {
        // a point of order 2 (the curve has none, its order is odd)
        r.infinity = true;
        return;
    }
    let mut slope = MaybeUninit::uninit();
    // SAFETY: both parts were written, and the denominator is not zero
    let slope = unsafe { fraction.divide(&mut slope, hooks) };
    apply_slope(r, slope, None);
}

/// The slope of `r + p` for the point `p = (x, y)`, or `(x, -y)` if `negate`, in `fraction`.
/// Returns whether the denominator is not zero, i.e. whether `p` is neither `r` nor `-r`; if it
/// is one of them, it is `r` when the numerator is zero too.
#[inline(always)]
fn slope_of_sum(
    fraction: &mut Fraction,
    r: &Affine,
    x: &FieldElement,
    y: &FieldElement,
    negate: bool,
) -> (bool, bool) {
    let (numerator, denominator) = fraction.parts();
    let (numerator, denominator) = if negate {
        // (-y - y_r) / (x - x_r) = (y + y_r) / (x_r - x)
        let numerator = write_copy(numerator, y);
        numerator.add_in_place(&r.y);
        let denominator = write_copy(denominator, &r.x);
        denominator.sub_in_place(x);
        (numerator, denominator)
    } else {
        let numerator = write_copy(numerator, y);
        numerator.sub_in_place(&r.y);
        let denominator = write_copy(denominator, x);
        denominator.sub_in_place(&r.x);
        (numerator, denominator)
    };
    if denominator.is_zero() {
        (false, numerator.is_zero())
    } else {
        (true, false)
    }
}

/// `r = p`, or `-p` if `negate`, for the point `p = (x, y)`
#[inline(always)]
fn set(r: &mut Affine, x: &FieldElement, y: &FieldElement, negate: bool) {
    assign(&mut r.x, x);
    assign(&mut r.y, y);
    if negate {
        r.y.negate_in_place(1);
        settle(&mut r.y);
    }
    r.infinity = false;
}

/// `r = r + p`, or `r - p` if `negate`, for the point `p = (x, y)`
fn add<H: Secp256k1Hooks>(
    r: &mut Affine,
    x: &FieldElement,
    y: &FieldElement,
    negate: bool,
    hooks: &mut H,
) {
    if r.infinity {
        set(r, x, y, negate);
        return;
    }
    let mut fraction = Fraction::uninit();
    let (distinct, same) = slope_of_sum(&mut fraction, r, x, y, negate);
    if !distinct {
        if same {
            double(r, hooks);
        } else {
            r.infinity = true;
        }
        return;
    }
    let mut slope = MaybeUninit::uninit();
    // SAFETY: both parts were written, and the denominator is not zero
    let slope = unsafe { fraction.divide(&mut slope, hooks) };
    apply_slope(r, slope, Some(x));
}

/// `r = 2 r + p`, or `2 r - p` if `negate`, for the point `p = (x, y)`: `(r + p) + r` without the
/// `y` coordinate of `r + p` (see the module documentation). The cases it does not cover (`p`
/// is `r` or `-r`, or `r + p` is `r` or `-r`) are a doubling and an addition.
fn double_and_add<H: Secp256k1Hooks>(
    r: &mut Affine,
    x: &FieldElement,
    y: &FieldElement,
    negate: bool,
    hooks: &mut H,
) {
    if r.infinity {
        set(r, x, y, negate);
        return;
    }
    let mut fraction = Fraction::uninit();
    let (distinct, _) = slope_of_sum(&mut fraction, r, x, y, negate);
    if !distinct {
        double(r, hooks);
        add(r, x, y, negate, hooks);
        return;
    }
    let mut slope = MaybeUninit::uninit();
    // SAFETY: both parts were written, and the denominator is not zero
    let slope = unsafe { fraction.divide(&mut slope, hooks) };

    // x_t = slope² - x_r - x, the coordinate of r + p
    let mut x_t = MaybeUninit::uninit();
    let x_t = write_copy(&mut x_t, slope);
    x_t.square_in_place();
    x_t.sub_in_place(&r.x);
    x_t.sub_in_place(x);

    // the slope of (r + p) + r is -(slope + 2 y_r / (x_t - x_r))
    let (numerator, denominator) = fraction.parts();
    let numerator = write_copy(numerator, &r.y);
    numerator.double_in_place();
    let denominator = write_copy(denominator, x_t);
    denominator.sub_in_place(&r.x);
    if denominator.is_zero() {
        double(r, hooks);
        add(r, x, y, negate, hooks);
        return;
    }
    let mut negated_slope = MaybeUninit::uninit();
    // SAFETY: both parts were written, and the denominator is not zero
    let negated_slope = unsafe { fraction.divide(&mut negated_slope, hooks) };
    negated_slope.add_in_place(slope);
    settle(negated_slope);

    // x3 = slope_2² - x_r - x_t, y3 = slope_2 (x_r - x3) - y_r = -slope_2 (x3 - x_r) - y_r
    let mut x3 = MaybeUninit::uninit();
    let x3 = write_copy(&mut x3, negated_slope);
    x3.square_in_place();
    x3.sub_in_place(&r.x);
    x3.sub_in_place(x_t);

    subtract_from(&mut r.x, x3);
    r.x.mul_in_place(negated_slope);
    subtract_from(&mut r.y, &r.x);
    assign(&mut r.x, x3);
    settle(&mut r.x);
    settle(&mut r.y);
}

/// `na * a + ng * g` for the generator `g`
pub(super) fn ecmult_affine<H: Secp256k1Hooks>(
    a: &Affine,
    na: &Scalar,
    ng: &Scalar,
    context: &ECMultContext,
    hooks: &mut H,
) -> Affine {
    // The odd multiples of `a`, and `beta * x` of each (the `x` of the endomorphism's image):
    // written only when `a` contributes, and read only at a non-zero digit of `na`, which
    // implies that
    let mut multiples: [MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A] =
        [const { MaybeUninit::uninit() }; ECMULT_TABLE_SIZE_A];
    let mut images: [MaybeUninit<FieldElement>; ECMULT_TABLE_SIZE_A] =
        [const { MaybeUninit::uninit() }; ECMULT_TABLE_SIZE_A];

    // The digits are zero above the bits of their scalars, so a digit is looked at only for
    // being non-zero
    let mut wnaf_na_1 = [0 as WnafDigit; WNAF_BITS];
    let mut wnaf_na_lam = [0 as WnafDigit; WNAF_BITS];
    let mut wnaf_ng_1 = [0 as WnafDigit; WNAF_BITS];
    let mut wnaf_ng_128 = [0 as WnafDigit; WNAF_BITS];

    let mut bits = 0;

    if !na.is_zero() && !a.is_infinity() {
        let (na_1, na_lam) = na.decompose();
        let bits_na_1 = wnaf(&mut wnaf_na_1, &na_1, WINDOW_A);
        let bits_na_lam = wnaf(&mut wnaf_na_lam, &na_lam, WINDOW_A);
        bits = bits_na_1.max(bits_na_lam);

        let mut doubled = *a;
        doubled.infinity = false;
        double(&mut doubled, hooks);
        let mut multiple = *a;
        multiple.infinity = false;
        for i in 0..ECMULT_TABLE_SIZE_A {
            if i > 0 && !doubled.infinity {
                add(&mut multiple, &doubled.x, &doubled.y, false, hooks);
            }
            let image = write_copy(&mut images[i], &FieldElement::BETA);
            image.mul_in_place(&multiple.x);
            multiples[i].write(multiple);
        }
    }

    if !ng.is_zero() {
        // halves of `ng`, not the endomorphism split: `ng_1 + ng_128 * 2^128 = ng`
        let (ng_1, ng_128) = ng.decompose_128();
        let bits_ng_1 = wnaf(&mut wnaf_ng_1, &ng_1, WINDOW_G);
        let bits_ng_128 = wnaf(&mut wnaf_ng_128, &ng_128, WINDOW_G);
        bits = bits.max(bits_ng_1).max(bits_ng_128);
    }

    let mut r = Affine::INFINITY;

    // the first addition after a doubling goes together with it
    let step = |r: &mut Affine,
                doubled: &mut bool,
                x: &FieldElement,
                y: &FieldElement,
                negate: bool,
                hooks: &mut H| {
        if *doubled {
            add(r, x, y, negate, hooks);
        } else {
            double_and_add(r, x, y, negate, hooks);
            *doubled = true;
        }
    };

    for i in (0..bits as usize).rev() {
        let mut doubled = false;

        let n = wnaf_na_1[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_A);
            // SAFETY: a non-zero digit of `na` implies that the table was written
            let multiple = unsafe { multiples[idx].assume_init_ref() };
            if !multiple.infinity {
                step(
                    &mut r,
                    &mut doubled,
                    &multiple.x,
                    &multiple.y,
                    negate,
                    hooks,
                );
            }
        }

        let n = wnaf_na_lam[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_A);
            // SAFETY: as above
            let (multiple, image) = unsafe {
                (
                    multiples[idx].assume_init_ref(),
                    images[idx].assume_init_ref(),
                )
            };
            if !multiple.infinity {
                step(&mut r, &mut doubled, image, &multiple.y, negate, hooks);
            }
        }

        let n = wnaf_ng_1[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_G);
            with_stored(&context.pre_g[idx], |x, y| {
                step(&mut r, &mut doubled, x, y, negate, hooks)
            });
        }

        let n = wnaf_ng_128[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_G);
            with_stored(&context.pre_g_128[idx], |x, y| {
                step(&mut r, &mut doubled, x, y, negate, hooks)
            });
        }

        if !doubled {
            double(&mut r, hooks);
        }
    }

    r
}

/// The coordinates of a stored multiple of the generator: read in place with the delegated
/// field, converted otherwise
#[inline(always)]
fn with_stored(stored: &AffineStorage, f: impl FnOnce(&FieldElement, &FieldElement)) {
    #[cfg(feature = "bigint_ops")]
    {
        let (x, y) = stored.coordinates();
        f(x, y);
    }
    #[cfg(not(feature = "bigint_ops"))]
    {
        let point = stored.to_affine();
        f(&point.x, &point.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secp256k1::context::ECRECOVER_CONTEXT;
    use crate::secp256k1::hooks::DefaultSecp256k1Hooks;
    use crate::secp256k1::points::Jacobian;
    use crate::secp256k1::recover::ecmult;
    use crate::secp256k1::test_vectors::MUL_TEST_VECTORS;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    /// The default operations, with the division declared cheap (it is the inversion and a
    /// multiplication): the affine algorithm on the host
    struct DivisionHooks {
        divisions: usize,
    }

    impl Secp256k1Hooks for DivisionHooks {
        const FE_DIVIDE_IS_CHEAP: bool = true;

        fn fe_sqrt_and_assign(&mut self, fe: &mut FieldElement) -> bool {
            DefaultSecp256k1Hooks.fe_sqrt_and_assign(fe)
        }

        fn fe_invert_and_assign(&mut self, fe: &mut FieldElement) {
            DefaultSecp256k1Hooks.fe_invert_and_assign(fe)
        }

        fn scalar_invert_and_assign(&mut self, scalar: &mut Scalar) {
            DefaultSecp256k1Hooks.scalar_invert_and_assign(scalar)
        }

        fn fe_divide<'a>(
            &mut self,
            fraction: &mut [FieldElement; 2],
            quotient: &'a mut MaybeUninit<FieldElement>,
        ) -> &'a mut FieldElement {
            assert!(!fraction[1].is_zero(), "a division by zero");
            self.divisions += 1;
            let quotient = DefaultSecp256k1Hooks.fe_divide(fraction, quotient);
            // a hook may use the fraction as scratch space
            *fraction = [FieldElement::ZERO; 2];
            quotient
        }
    }

    fn normalized(mut point: Affine) -> Affine {
        if point.infinity {
            return Affine::INFINITY;
        }
        point.normalize_in_place();
        point
    }

    /// The affine result, checked against the Jacobian algorithm
    fn checked(a: &Affine, na: &Scalar, ng: &Scalar) -> Affine {
        let mut hooks = DivisionHooks { divisions: 0 };
        let affine = normalized(ecmult_affine(a, na, ng, &ECRECOVER_CONTEXT, &mut hooks));
        let jacobian = if a.is_infinity() {
            Jacobian::INFINITY
        } else {
            a.to_jacobian()
        };
        let reference = ecmult(&jacobian, na, ng, &ECRECOVER_CONTEXT);
        let reference = if reference.is_infinity() {
            Affine::INFINITY
        } else {
            normalized(reference.to_affine())
        };
        assert_eq!(affine.infinity, reference.infinity);
        if !affine.infinity {
            assert_eq!(affine, reference);
        }
        affine
    }

    fn small(k: u32) -> Scalar {
        let mut bytes = [0u8; 32];
        bytes[28..].copy_from_slice(&k.to_be_bytes());
        Scalar::from_repr(bytes.into())
    }

    fn negated(mut scalar: Scalar) -> Scalar {
        scalar.negate_in_place();
        scalar
    }

    fn negated_point(mut point: Affine) -> Affine {
        point.y.negate_in_place(1);
        point.y.normalize_in_place();
        point
    }

    #[test]
    fn trivial_inputs() {
        let g = Affine::GENERATOR;
        assert!(checked(&Affine::INFINITY, &Scalar::ZERO, &Scalar::ZERO).infinity);
        assert!(checked(&g, &Scalar::ZERO, &Scalar::ZERO).infinity);
        assert!(checked(&Affine::INFINITY, &small(5), &Scalar::ZERO).infinity);
        assert_eq!(checked(&Affine::INFINITY, &Scalar::ZERO, &Scalar::ONE), g);
        assert_eq!(checked(&g, &Scalar::ONE, &Scalar::ZERO), g);
    }

    /// Accumulators that meet the point to add, or its negation: the inputs of the precompile
    /// can be chosen for that
    #[test]
    fn exceptional_cases() {
        let g = Affine::GENERATOR;
        // g + g is a doubling, g - g the point at infinity
        checked(&g, &Scalar::ONE, &Scalar::ONE);
        assert!(checked(&g, &Scalar::ONE, &negated(Scalar::ONE)).infinity);
        assert!(checked(&negated_point(g), &Scalar::ONE, &Scalar::ONE).infinity);
        assert!(checked(&g, &negated(Scalar::ONE), &Scalar::ONE).infinity);
        // multiples of the generator that cancel or coincide, through every table and digit
        for k in 1..70u32 {
            let multiple = checked(&Affine::INFINITY, &Scalar::ZERO, &small(k));
            for l in 1..20u32 {
                checked(&multiple, &small(l), &small(k * l));
                assert!(checked(&multiple, &small(l), &negated(small(k * l))).infinity);
                checked(&multiple, &small(l), &small(k));
                checked(&multiple, &negated(small(l)), &small(k * (l + 1)));
                checked(&multiple, &small(l), &negated(small(k * (l + 1))));
                checked(&negated_point(multiple), &small(l), &small(2 * k * l));
            }
        }
    }

    #[test]
    fn test_vectors() {
        for (k_bytes, x_bytes, y_bytes) in MUL_TEST_VECTORS.iter() {
            let k = Scalar::from_repr((*k_bytes).into());
            let expected = Affine {
                x: FieldElement::from_bytes_unchecked(x_bytes),
                y: FieldElement::from_bytes_unchecked(y_bytes),
                infinity: false,
            };
            assert_eq!(
                checked(&Affine::INFINITY, &Scalar::ZERO, &k),
                normalized(expected)
            );
            assert_eq!(
                checked(&Affine::GENERATOR, &k, &Scalar::ZERO),
                normalized(expected)
            );
        }
    }

    /// Random scalars and random points of the curve (a point off the curve has no group law
    /// with the generator, and the two algorithms add in different orders)
    #[test]
    fn random_inputs() {
        proptest!(|(x: FieldElement, y_is_odd: bool, na: Scalar, ng: Scalar)| {
            let mut a = Affine::DEFAULT;
            if !a.set_xo_for_tests(&x, y_is_odd) {
                return Ok(());
            }
            let mut hooks = DivisionHooks { divisions: 0 };
            let affine = normalized(ecmult_affine(&a, &na, &ng, &ECRECOVER_CONTEXT, &mut hooks));
            let reference = ecmult(&a.to_jacobian(), &na, &ng, &ECRECOVER_CONTEXT);
            prop_assert_eq!(affine.infinity, reference.is_infinity());
            if !affine.infinity {
                prop_assert_eq!(affine, normalized(reference.to_affine()));
            }
            // a doubling or a first half of a double-and-add per bit, a division per addition
            prop_assert!(hooks.divisions <= 2 * 130 + 4 * 70 + 8);
        });
    }
}
