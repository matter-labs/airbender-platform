//! GLV scalar multiplication in affine coordinates, for short Weierstrass curves with `a = 0`
//! (bn254 and bls12-381 G1) and a cheap field division: a [`Divider`] that takes the quotient
//! as a hint and checks it with one multiplication.
//!
//! An affine addition or doubling is its slope `λ` (a division), then `x3 = λ² - x1 - x2` and
//! `y3 = λ (x1 - x3) - y1`: one squaring and one multiplication besides the division, where the
//! Jacobian formulas take 7 multiplications for a doubling and 11 to 16 for an addition. The
//! scalar is split by the endomorphism in two halves of 128 bits (as in
//! `glv_decomposition`), each in a windowed non-adjacent form over the odd multiples of `P`,
//! whose images under the endomorphism cost a multiplication each.
//!
//! A doubling followed by an addition, `2 r + p`, is computed as `(r + p) + r` without the `y`
//! of the intermediate point: with `λ1` the slope of `r + p` and `x_t` the `x` of that sum, the
//! slope of the second addition is `-(λ1 + 2 y_r / (x_t - x_r))`. That is two divisions, two
//! squarings and one multiplication.
//!
//! All the formulas are complete here: the inputs of the precompiles are chosen by their
//! callers. Every division has its denominator tested, and a zero sends the step to the case it
//! stands for (a doubling, or the point at infinity).

use core::mem::MaybeUninit;

use ark_ec::{
    scalar_mul::glv::GLVConfig,
    short_weierstrass::{Affine, SWCurveConfig},
};
use ark_ff::{Field, PrimeField, Zero};

use crate::extension_tower::{fp_init, fp_tmp, CopyAssign};
use crate::glv_decomposition::{glv_halves, GLVConfigNoAllocator};

/// A field division
pub trait Divider<F> {
    /// `fraction[0] / fraction[1]` into `quotient`, for a non-zero denominator. The numerator
    /// and the denominator are adjacent in memory, so that an oracle reads them as one operand.
    /// The fraction is scratch space of the divider (e.g. for the check of a hinted quotient):
    /// its value is unspecified afterwards.
    fn divide<'a>(&mut self, fraction: &mut [F; 2], quotient: &'a mut MaybeUninit<F>) -> &'a mut F;
}

/// Divides by the inverse that the field computes
pub struct InvertingDivider;

impl<F: Field> Divider<F> for InvertingDivider {
    fn divide<'a>(&mut self, fraction: &mut [F; 2], quotient: &'a mut MaybeUninit<F>) -> &'a mut F {
        let inverse = fraction[1].inverse().expect("the denominator is not zero");
        quotient.write(fraction[0] * inverse)
    }
}

/// A numerator and a denominator, adjacent as [`Divider::divide`] takes them
struct Fraction<F>([MaybeUninit<F>; 2]);

impl<F: CopyAssign> Fraction<F> {
    #[inline(always)]
    fn uninit() -> Self {
        Self([const { MaybeUninit::uninit() }; 2])
    }

    /// Initializes the fraction with copies of a numerator and a denominator, to be changed
    /// in place
    #[inline(always)]
    fn set(&mut self, numerator: &F, denominator: &F) -> (&mut F, &mut F) {
        let [n, d] = &mut self.0;
        (fp_init(n, numerator), fp_init(d, denominator))
    }

    /// The quotient into `quotient`. The parts are to be set again before the next division:
    /// the divider may overwrite them.
    ///
    /// # Safety
    /// Both parts must be initialized, and the denominator must not be zero.
    #[inline(always)]
    unsafe fn divide<'a, D: Divider<F>>(
        &mut self,
        quotient: &'a mut MaybeUninit<F>,
        divider: &mut D,
    ) -> &'a mut F {
        // SAFETY: `MaybeUninit<T>` has the layout of `T`, and the parts are initialized
        let fraction = unsafe { &mut *(&mut self.0 as *mut [MaybeUninit<F>; 2]).cast() };
        divider.divide(fraction, quotient)
    }
}

/// An affine point that operations update in place
struct Point<F> {
    x: F,
    y: F,
    infinity: bool,
}

/// A point to add: its coordinates, and whether it is to be negated first
#[derive(Clone, Copy)]
struct Addend<'a, F> {
    x: &'a F,
    y: &'a F,
    negate: bool,
}

impl<F: Field + CopyAssign> Point<F> {
    #[inline(always)]
    fn init_infinity(slot: &mut MaybeUninit<Self>) -> &mut Self {
        // SAFETY: all fields are written before the value is used
        unsafe {
            let p = slot.as_mut_ptr();
            F::init_copy(&raw mut (*p).x, &F::ZERO);
            F::init_copy(&raw mut (*p).y, &F::ZERO);
            (&raw mut (*p).infinity).write(true);
            slot.assume_init_mut()
        }
    }

    #[inline(always)]
    fn set(&mut self, p: Addend<F>) {
        self.x.copy_assign(p.x);
        self.y.copy_assign(p.y);
        if p.negate {
            self.y.neg_in_place();
        }
        self.infinity = false;
    }

    /// The rest of an addition of a point with the coordinate `x2` (a doubling if `None`),
    /// given the slope: `x3 = slope² - x1 - x2`, `y3 = slope (x1 - x3) - y1`
    #[inline(always)]
    fn apply_slope(&mut self, slope: &F, x2: Option<&F>) {
        fp_tmp!(x3 = slope);
        *x3 *= slope;
        *x3 -= &self.x;
        *x3 -= x2.unwrap_or(&self.x);

        self.x -= &*x3;
        self.x *= slope;
        self.x -= &self.y;
        self.y.copy_assign(&self.x);
        self.x.copy_assign(x3);
    }

    /// `self = 2 self`
    fn double<D: Divider<F>>(&mut self, divider: &mut D) {
        if self.infinity {
            return;
        }
        // the slope is 3 x² / 2 y
        let mut fraction = Fraction::uninit();
        let (numerator, denominator) = fraction.set(&self.x, &self.y);
        *numerator *= &self.x;
        fp_tmp!(square = &*numerator);
        numerator.double_in_place();
        *numerator += &*square;
        denominator.double_in_place();
        if denominator.is_zero() {
            // a point of order 2
            self.infinity = true;
            return;
        }
        let mut slope = MaybeUninit::uninit();
        // SAFETY: both parts were set, and the denominator is not zero
        let slope = unsafe { fraction.divide(&mut slope, divider) };
        self.apply_slope(slope, None);
    }

    /// The slope of `self + p` in `fraction`. Returns whether the denominator is not zero,
    /// i.e. whether `p` is neither `self` nor `-self`, and otherwise whether it is `self`.
    #[inline(always)]
    fn slope_of_sum(&self, fraction: &mut Fraction<F>, p: Addend<F>) -> (bool, bool) {
        let (numerator, denominator) = if p.negate {
            // (-y - y_r) / (x - x_r) = (y + y_r) / (x_r - x)
            let (numerator, denominator) = fraction.set(p.y, &self.x);
            *numerator += &self.y;
            *denominator -= p.x;
            (numerator, denominator)
        } else {
            let (numerator, denominator) = fraction.set(p.y, p.x);
            *numerator -= &self.y;
            *denominator -= &self.x;
            (numerator, denominator)
        };
        if denominator.is_zero() {
            (false, numerator.is_zero())
        } else {
            (true, false)
        }
    }

    /// `self = self + p`
    fn add<D: Divider<F>>(&mut self, p: Addend<F>, divider: &mut D) {
        if self.infinity {
            self.set(p);
            return;
        }
        let mut fraction = Fraction::uninit();
        let (distinct, same) = self.slope_of_sum(&mut fraction, p);
        if !distinct {
            if same {
                self.double(divider);
            } else {
                self.infinity = true;
            }
            return;
        }
        let mut slope = MaybeUninit::uninit();
        // SAFETY: both parts were set, and the denominator is not zero
        let slope = unsafe { fraction.divide(&mut slope, divider) };
        self.apply_slope(slope, Some(p.x));
    }

    /// `self = 2 self + p`: `(self + p) + self` without the `y` coordinate of `self + p` (see
    /// the module documentation). The cases it does not cover (`p` is `self` or `-self`, or
    /// `self + p` is `self` or `-self`) are a doubling and an addition.
    fn double_and_add<D: Divider<F>>(&mut self, p: Addend<F>, divider: &mut D) {
        if self.infinity {
            self.set(p);
            return;
        }
        let mut fraction = Fraction::uninit();
        let (distinct, _) = self.slope_of_sum(&mut fraction, p);
        if !distinct {
            self.double(divider);
            self.add(p, divider);
            return;
        }
        let mut slope = MaybeUninit::uninit();
        // SAFETY: both parts were set, and the denominator is not zero
        let slope = unsafe { fraction.divide(&mut slope, divider) };

        // x_t = slope² - x_r - x, the coordinate of self + p
        fp_tmp!(x_t = &*slope);
        *x_t *= &*slope;
        *x_t -= &self.x;
        *x_t -= p.x;

        // the slope of (self + p) + self is -(slope + 2 y_r / (x_t - x_r))
        let (numerator, denominator) = fraction.set(&self.y, x_t);
        numerator.double_in_place();
        *denominator -= &self.x;
        if denominator.is_zero() {
            self.double(divider);
            self.add(p, divider);
            return;
        }
        let mut negated_slope = MaybeUninit::uninit();
        // SAFETY: both parts were set, and the denominator is not zero
        let negated_slope = unsafe { fraction.divide(&mut negated_slope, divider) };
        *negated_slope += &*slope;

        // x3 = slope_2² - x_r - x_t, y3 = slope_2 (x_r - x3) - y_r = -slope_2 (x3 - x_r) - y_r
        fp_tmp!(x3 = &*negated_slope);
        *x3 *= &*negated_slope;
        *x3 -= &self.x;
        *x3 -= &*x_t;

        x_t.copy_assign(x3);
        *x_t -= &self.x;
        *x_t *= &*negated_slope;
        *x_t -= &self.y;
        self.y.copy_assign(x_t);
        self.x.copy_assign(x3);
    }
}

/// Width of the windows of the non-adjacent forms of the halves of the scalar: with the 128
/// bits of a half, `2 * 128 / (w + 1)` additions in the loop and `2^(w - 2)` group operations
/// for the table are the fewest at `w = 5`
const WINDOW: usize = 5;
/// Odd multiples in the table: `1, 3, ..., 2^(WINDOW - 1) - 1`
const TABLE_SIZE: usize = 1 << (WINDOW - 2);
/// Digits of the non-adjacent form of a half: it is below `2^128`, and a carry can take one
/// more
const DIGITS: usize = 129;

/// The non-adjacent form of `k` with the window `WINDOW`: digits from the lowest, zero or odd
/// with a magnitude below `2^(WINDOW - 1)`, with at least `WINDOW - 1` zeros after a non-zero
/// one, into `digits`, which must be zeros. Returns the position after the highest non-zero
/// one.
fn wnaf(digits: &mut [i8; DIGITS], k: u128) -> usize {
    // as 32-bit words, with zeros above: a bit test is one shift on a 32-bit target, and a
    // window is read from a pair of words
    let mut words = [0u32; 6];
    for (i, word) in words.iter_mut().take(4).enumerate() {
        *word = (k >> (32 * i)) as u32;
    }
    let mut len = 0;
    let mut carry = 0u32;
    let mut bit = 0;
    while bit < DIGITS {
        if (words[bit >> 5] >> (bit & 31)) & 1 == carry {
            bit += 1;
            continue;
        }
        let pair = words[bit >> 5] as u64 | ((words[(bit >> 5) + 1] as u64) << 32);
        let window = ((pair >> (bit & 31)) as u32 & ((1 << WINDOW) - 1)) + carry;
        carry = window >> (WINDOW - 1);
        digits[bit] = (window as i32 - ((carry as i32) << WINDOW)) as i8;
        len = bit + 1;
        bit += WINDOW;
    }
    debug_assert_eq!(carry, 0);
    len
}

/// The odd multiples `1, 3, ...` of a point `P`, and the `x` coordinates of their images under
/// the endomorphism (`φ(x, y) = (β x, y)`)
struct Table<F> {
    multiples: [Point<F>; TABLE_SIZE],
    images: [F; TABLE_SIZE],
}

impl<F: Field + CopyAssign> Table<F> {
    /// Initializes `slot` with the table of `(x, y)`, which is not the point at infinity
    fn init<'a, D: Divider<F>>(
        slot: &'a mut MaybeUninit<Self>,
        x: &F,
        y: &F,
        beta: &F,
        divider: &mut D,
    ) -> &'a Self {
        let this = slot.as_mut_ptr();
        let p = Addend {
            x,
            y,
            negate: false,
        };
        let mut doubled = MaybeUninit::uninit();
        let doubled = Point::init_infinity(&mut doubled);
        doubled.set(p);
        doubled.double(divider);
        // SAFETY: the places of the multiples and of the images are inside the table, written
        // one after another before they are read, and all of them before the table is
        let mut previous: &Point<F> = unsafe {
            let first = Point::init_infinity(&mut *(&raw mut (*this).multiples[0]).cast());
            first.set(p);
            first
        };
        for i in 0..TABLE_SIZE {
            if i > 0 {
                // SAFETY: as above
                let multiple =
                    unsafe { Point::init_infinity(&mut *(&raw mut (*this).multiples[i]).cast()) };
                if !previous.infinity {
                    multiple.set(Addend {
                        x: &previous.x,
                        y: &previous.y,
                        negate: false,
                    });
                }
                if !doubled.infinity {
                    multiple.add(
                        Addend {
                            x: &doubled.x,
                            y: &doubled.y,
                            negate: false,
                        },
                        divider,
                    );
                }
                previous = multiple;
            }
            // SAFETY: as above
            let image = unsafe { fp_init(&mut *(&raw mut (*this).images[i]).cast(), beta) };
            *image *= &previous.x;
        }
        // SAFETY: all the multiples and the images were written
        unsafe { slot.assume_init_ref() }
    }

    /// The point of a non-zero digit, or its image under the endomorphism, negated if `negate`
    /// (the sign of the half of the scalar); `None` for the point at infinity
    #[inline(always)]
    fn addend(&self, digit: i8, image: bool, negate: bool) -> Option<Addend<'_, F>> {
        debug_assert!(digit & 1 == 1);
        let index = (digit.unsigned_abs() as usize - 1) / 2;
        let multiple = &self.multiples[index];
        (!multiple.infinity).then_some(Addend {
            x: if image {
                &self.images[index]
            } else {
                &multiple.x
            },
            y: &multiple.y,
            negate: negate ^ (digit < 0),
        })
    }
}

/// `k p` with the divisions of `divider`
pub(crate) fn glv_mul_affine<C, D>(p: &Affine<C>, k: C::ScalarField, divider: &mut D) -> Affine<C>
where
    C: SWCurveConfig + GLVConfig + GLVConfigNoAllocator,
    C::BaseField: CopyAssign,
    D: Divider<C::BaseField>,
{
    debug_assert!(C::COEFF_A.is_zero());
    if p.infinity {
        return *p;
    }
    let Some(((positive_1, k1), (positive_2, k2))) = glv_halves::<C>(k) else {
        // the decomposition keeps both halves below 2^128 for the supported curves; the
        // Jacobian double-and-add is the fallback should one not fit
        use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
        return p.into_group().mul_bigint(k.into_bigint()).into_affine();
    };
    let mut digits_1 = [0i8; DIGITS];
    let mut digits_2 = [0i8; DIGITS];
    let len = wnaf(&mut digits_1, k1).max(wnaf(&mut digits_2, k2));
    if len == 0 {
        return Affine::identity();
    }

    let mut table = MaybeUninit::uninit();
    let table = Table::init(&mut table, &p.x, &p.y, &C::ENDO_COEFFS[0], divider);

    let mut slot = MaybeUninit::uninit();
    let r = Point::init_infinity(&mut slot);
    for i in (0..len).rev() {
        // zero digits are the most of them
        let first = match digits_1[i] {
            0 => None,
            digit => table.addend(digit, false, !positive_1),
        };
        let second = match digits_2[i] {
            0 => None,
            digit => table.addend(digit, true, !positive_2),
        };
        // the first addition after a doubling goes together with it
        match (first, second) {
            (None, None) => r.double(divider),
            (Some(addend), None) | (None, Some(addend)) => r.double_and_add(addend, divider),
            (Some(first), Some(second)) => {
                r.double_and_add(first, divider);
                r.add(second, divider);
            }
        }
    }

    if r.infinity {
        Affine::identity()
    } else {
        Affine::new_unchecked(r.x, r.y)
    }
}

/// `k p + l q` with the divisions of `divider`: the doublings are shared between the two
/// multiplications
pub(crate) fn glv_mul_two_affine<C, D>(
    p: &Affine<C>,
    k: C::ScalarField,
    q: &Affine<C>,
    l: C::ScalarField,
    divider: &mut D,
) -> Affine<C>
where
    C: SWCurveConfig + GLVConfig + GLVConfigNoAllocator,
    C::BaseField: CopyAssign,
    D: Divider<C::BaseField>,
{
    debug_assert!(C::COEFF_A.is_zero());
    if p.infinity {
        return glv_mul_affine(q, l, divider);
    }
    if q.infinity {
        return glv_mul_affine(p, k, divider);
    }
    let (Some(halves_p), Some(halves_q)) = (glv_halves::<C>(k), glv_halves::<C>(l)) else {
        // see `glv_mul_affine`
        use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
        let sum =
            p.into_group().mul_bigint(k.into_bigint()) + q.into_group().mul_bigint(l.into_bigint());
        return sum.into_affine();
    };
    let mut digits = [[0i8; DIGITS]; 4];
    let [digits_p1, digits_p2, digits_q1, digits_q2] = &mut digits;
    let len = wnaf(digits_p1, halves_p.0 .1)
        .max(wnaf(digits_p2, halves_p.1 .1))
        .max(wnaf(digits_q1, halves_q.0 .1))
        .max(wnaf(digits_q2, halves_q.1 .1));
    if len == 0 {
        return Affine::identity();
    }
    let beta = &C::ENDO_COEFFS[0];
    let mut table_p = MaybeUninit::uninit();
    let table_p = Table::init(&mut table_p, &p.x, &p.y, beta, divider);
    let mut table_q = MaybeUninit::uninit();
    let table_q = Table::init(&mut table_q, &q.x, &q.y, beta, divider);

    let mut slot = MaybeUninit::uninit();
    let r = Point::init_infinity(&mut slot);
    for i in (0..len).rev() {
        // the first addition after a doubling goes together with it
        let mut doubled = false;
        let streams = [
            (table_p, digits_p1[i], false, !halves_p.0 .0),
            (table_p, digits_p2[i], true, !halves_p.1 .0),
            (table_q, digits_q1[i], false, !halves_q.0 .0),
            (table_q, digits_q2[i], true, !halves_q.1 .0),
        ];
        for (table, digit, image, negate) in streams {
            if digit == 0 {
                continue;
            }
            let Some(addend) = table.addend(digit, image, negate) else {
                continue;
            };
            if doubled {
                r.add(addend, divider);
            } else {
                r.double_and_add(addend, divider);
                doubled = true;
            }
        }
        if !doubled {
            r.double(divider);
        }
    }

    if r.infinity {
        Affine::identity()
    } else {
        Affine::new_unchecked(r.x, r.y)
    }
}

/// `k p` for a small `k`, by double-and-add, with the divisions of `divider`
pub(crate) fn mul_u64_affine<C, D>(p: &Affine<C>, k: u64, divider: &mut D) -> Affine<C>
where
    C: SWCurveConfig,
    C::BaseField: CopyAssign,
    D: Divider<C::BaseField>,
{
    debug_assert!(C::COEFF_A.is_zero());
    if p.infinity || k == 0 {
        return Affine::identity();
    }
    let addend = Addend {
        x: &p.x,
        y: &p.y,
        negate: false,
    };
    let mut slot = MaybeUninit::uninit();
    let r = Point::init_infinity(&mut slot);
    for i in (0..64 - k.leading_zeros()).rev() {
        if (k >> i) & 1 == 1 {
            r.double_and_add(addend, divider);
        } else {
            r.double(divider);
        }
    }
    if r.infinity {
        Affine::identity()
    } else {
        Affine::new_unchecked(r.x, r.y)
    }
}

/// `a + b` with the division of `divider`
pub(crate) fn add_affine<C, D>(a: &Affine<C>, b: &Affine<C>, divider: &mut D) -> Affine<C>
where
    C: SWCurveConfig,
    C::BaseField: CopyAssign,
    D: Divider<C::BaseField>,
{
    debug_assert!(C::COEFF_A.is_zero());
    if a.infinity {
        return *b;
    }
    if b.infinity {
        return *a;
    }
    let mut slot = MaybeUninit::uninit();
    let r = Point::init_infinity(&mut slot);
    r.set(Addend {
        x: &a.x,
        y: &a.y,
        negate: false,
    });
    r.add(
        Addend {
            x: &b.x,
            y: &b.y,
            negate: false,
        },
        divider,
    );
    if r.infinity {
        Affine::identity()
    } else {
        Affine::new_unchecked(r.x, r.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
    use ark_ff::{AdditiveGroup, UniformRand};

    /// Counts the divisions, rejects a zero denominator and overwrites the fraction
    struct Counting(usize);

    impl<F: Field> Divider<F> for Counting {
        fn divide<'a>(
            &mut self,
            fraction: &mut [F; 2],
            quotient: &'a mut MaybeUninit<F>,
        ) -> &'a mut F {
            assert!(!fraction[1].is_zero(), "a division by zero");
            self.0 += 1;
            let quotient = InvertingDivider.divide(fraction, quotient);
            *fraction = [F::ZERO; 2];
            quotient
        }
    }

    fn multiples<C: SWCurveConfig>(count: usize) -> Vec<Affine<C>> {
        let generator = C::GENERATOR.into_group();
        let mut p = generator;
        let mut points = vec![Affine::identity(), C::GENERATOR];
        for _ in 0..count {
            p.double_in_place();
            p += &generator;
            points.push(p.into_affine());
        }
        points
    }

    fn check_additions<C: SWCurveConfig>()
    where
        C::BaseField: CopyAssign,
    {
        let points = multiples::<C>(3);
        let negated: Vec<_> = points.iter().map(|p| -*p).collect();
        for a in points.iter().chain(&negated) {
            for b in points.iter().chain(&negated) {
                let expected = (a.into_group() + b).into_affine();
                assert_eq!(add_affine(a, b, &mut Counting(0)), expected);
            }
        }
    }

    fn check_steps<C: SWCurveConfig>()
    where
        C::BaseField: CopyAssign,
    {
        let points = multiples::<C>(3);
        let negated: Vec<_> = points.iter().map(|p| -*p).collect();
        for a in points.iter().chain(&negated) {
            for b in points.iter().chain(&negated).filter(|b| !b.infinity) {
                for negate in [false, true] {
                    let mut slot = MaybeUninit::uninit();
                    let r = Point::init_infinity(&mut slot);
                    if !a.infinity {
                        r.set(Addend {
                            x: &a.x,
                            y: &a.y,
                            negate: false,
                        });
                    }
                    r.double_and_add(
                        Addend {
                            x: &b.x,
                            y: &b.y,
                            negate,
                        },
                        &mut Counting(0),
                    );
                    let b = if negate { -*b } else { *b };
                    let expected = (a.into_group().double() + b).into_affine();
                    assert_eq!(r.infinity, expected.infinity);
                    if !r.infinity {
                        assert_eq!((r.x, r.y), (expected.x, expected.y));
                    }
                }
            }
        }
    }

    fn check_multiplication<C>()
    where
        C: SWCurveConfig + GLVConfig + GLVConfigNoAllocator,
        C::BaseField: CopyAssign,
    {
        let mut rng = ark_std::test_rng();
        let mut scalars = vec![
            C::ScalarField::ZERO,
            C::ScalarField::ONE,
            -C::ScalarField::ONE,
            C::ScalarField::from(2u64),
            C::LAMBDA,
            -C::LAMBDA,
            C::LAMBDA + C::ScalarField::ONE,
            C::LAMBDA - C::ScalarField::ONE,
        ];
        scalars.extend((0..6).map(|_| C::ScalarField::rand(&mut rng)));
        for p in multiples::<C>(2) {
            for k in &scalars {
                let mut divider = Counting(0);
                let expected = p.into_group().mul_bigint(k.into_bigint()).into_affine();
                assert_eq!(glv_mul_affine(&p, *k, &mut divider), expected);
                // one division per digit and per addition, and those of the table
                assert!(divider.0 <= DIGITS + 2 * DIGITS.div_ceil(WINDOW) + TABLE_SIZE);
            }
        }
    }

    #[test]
    fn non_adjacent_form() {
        let mut k = 0x9e37_79b9_7f4a_7c15_d1b5_4a32_d192_ed03u128;
        let mut scalars = vec![0, 1, 2, 15, 16, 17, u128::MAX, u128::MAX - 1, 1 << 127];
        for _ in 0..200 {
            k = k.wrapping_mul(k).wrapping_add(0x1234_5678_9abc_def1);
            scalars.push(k);
            scalars.push(k >> (k as u32 % 128));
        }
        for k in scalars {
            let mut digits = [0i8; DIGITS];
            let len = wnaf(&mut digits, k);
            assert!(digits[len..].iter().all(|digit| *digit == 0));
            assert!(len == 0 || digits[len - 1] != 0);
            // evaluated from the top modulo 2^128, with the digit at 2^128 dropping out
            let mut value = 0u128;
            let mut zeros = WINDOW;
            for digit in digits[..len.min(128)].iter().rev() {
                value = value.wrapping_mul(2).wrapping_add(*digit as i128 as u128);
                if *digit == 0 {
                    zeros += 1;
                } else {
                    assert!(digit & 1 == 1 && digit.unsigned_abs() < 1 << (WINDOW - 1));
                    assert!(zeros >= WINDOW - 1);
                    zeros = 0;
                }
            }
            assert_eq!(value, k);
            assert!(len <= 128 || digits[128] == 1);
        }
    }

    fn check_two_and_small<C>()
    where
        C: SWCurveConfig + GLVConfig + GLVConfigNoAllocator,
        C::BaseField: CopyAssign,
    {
        let mut rng = ark_std::test_rng();
        let points = multiples::<C>(1);
        let scalars = [
            C::ScalarField::ZERO,
            -C::ScalarField::ONE,
            C::LAMBDA,
            C::ScalarField::rand(&mut rng),
        ];
        // the pairs of points and of scalars: the products of the emulated delegations are slow
        for (i, (p, q)) in points.iter().zip(points.iter().rev()).enumerate() {
            for (k, l) in scalars
                .iter()
                .zip(scalars.iter().cycle().skip(i))
                .chain([(&scalars[3], &scalars[3])])
            {
                let expected = (p.into_group().mul_bigint(k.into_bigint())
                    + q.into_group().mul_bigint(l.into_bigint()))
                .into_affine();
                assert_eq!(glv_mul_two_affine(p, *k, q, *l, &mut Counting(0)), expected);
            }
        }
        for p in &points {
            for k in [0u64, 1, 3, 0xd201000000010000, u64::MAX] {
                let expected = p.into_group().mul_bigint([k]).into_affine();
                assert_eq!(mul_u64_affine(p, k, &mut Counting(0)), expected);
            }
        }
    }

    #[test]
    fn bn254_g1() {
        check_additions::<crate::bn254::curves::g1::Config>();
        check_steps::<crate::bn254::curves::g1::Config>();
        check_multiplication::<crate::bn254::curves::g1::Config>();
        check_two_and_small::<crate::bn254::curves::g1::Config>();
    }

    #[test]
    fn bls12_381_g1() {
        check_additions::<crate::bls12_381::curves::g1::Config>();
        check_steps::<crate::bls12_381::curves::g1::Config>();
        check_multiplication::<crate::bls12_381::curves::g1::Config>();
        check_two_and_small::<crate::bls12_381::curves::g1::Config>();
    }
}
