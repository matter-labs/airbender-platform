//! The affine group operations on elements in memory, updated in place: for the delegated
//! field, whose operations are delegation calls on memory operands, and for the fields with
//! lazy reduction (magnitudes). See the documentation of the parent module for the formulas.

use core::mem::MaybeUninit;

use super::write_copy;
use crate::secp256k1::{field::FieldElement, hooks::Secp256k1Hooks, points::Affine};

#[inline(always)]
fn assign(dst: &mut FieldElement, src: &FieldElement) {
    #[cfg(secp256k1_weak_field)]
    {
        dst.copy_from(src);
    }
    #[cfg(not(secp256k1_weak_field))]
    {
        *dst = *src;
    }
}

#[inline(always)]
fn triple(x: &mut FieldElement) {
    #[cfg(secp256k1_weak_field)]
    {
        x.triple_in_place();
    }
    #[cfg(not(secp256k1_weak_field))]
    {
        x.mul_int_in_place(3);
    }
}

/// `x = minuend - x`, where `x` has magnitude at most `magnitude` (see `sub`)
#[inline(always)]
fn subtract_from(x: &mut FieldElement, minuend: &FieldElement, magnitude: u32) {
    #[cfg(secp256k1_weak_field)]
    {
        let _ = magnitude;
        x.sub_and_negate_in_place(minuend);
    }
    #[cfg(not(secp256k1_weak_field))]
    {
        let mut difference = *minuend;
        difference.sub_in_place_magnitude(x, magnitude);
        *x = difference;
    }
}

/// The representations with magnitudes are brought back to magnitude 1 between the steps (a
/// weak normalization: the bounds below count on it); the delegated one needs nothing
#[inline(always)]
fn settle(x: &mut FieldElement) {
    #[cfg(secp256k1_weak_field)]
    {
        let _ = x;
    }
    #[cfg(not(secp256k1_weak_field))]
    {
        x.normalize_weak_in_place();
    }
}

/// `x -= rhs`, where `rhs` has magnitude at most `magnitude` in the representations with
/// magnitudes (1 for a settled value, a product or a table entry; see
/// `FieldElement::sub_in_place_magnitude`): the subtraction without normalizing `rhs`. The
/// magnitudes are tracked in the comments of the callers; a multiplication takes inputs of
/// magnitude at most 8.
#[inline(always)]
fn sub(x: &mut FieldElement, rhs: &FieldElement, magnitude: u32) {
    #[cfg(secp256k1_weak_field)]
    {
        let _ = magnitude;
        x.sub_in_place(rhs);
    }
    #[cfg(not(secp256k1_weak_field))]
    {
        x.sub_in_place_magnitude(rhs, magnitude);
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
/// given the slope: `x3 = slope² - x1 - x2`, `y3 = slope (x1 - x3) - y1`. `r`, `x2` and the
/// slope have magnitude 1 (the magnitudes in brackets).
#[inline(always)]
fn apply_slope(r: &mut Affine, slope: &FieldElement, x2: Option<&FieldElement>) {
    let mut x3 = MaybeUninit::uninit();
    let x3 = write_copy(&mut x3, slope);
    x3.square_in_place(); // [1]
    sub(x3, &r.x, 1); // [3]
    sub(x3, x2.unwrap_or(&r.x), 1); // [5]

    sub(&mut r.x, x3, 5); // [7]
    r.x.mul_in_place(slope); // [1]
    subtract_from(&mut r.y, &r.x, 1); // [3]
    assign(&mut r.x, x3); // [5]
    settle(&mut r.x);
    settle(&mut r.y);
}

/// `r = 2 r`
pub(super) fn double<H: Secp256k1Hooks>(r: &mut Affine, hooks: &mut H) {
    if r.infinity {
        return;
    }
    // the slope is 3 x² / 2 y
    let mut fraction = Fraction::uninit();
    let (numerator, denominator) = fraction.parts();
    let numerator = write_copy(numerator, &r.x);
    numerator.square_in_place();
    triple(numerator); // [3]
    let denominator = write_copy(denominator, &r.y);
    denominator.double_in_place(); // [2]
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
        numerator.add_in_place(&r.y); // [2]
        let denominator = write_copy(denominator, &r.x);
        sub(denominator, x, 1); // [3]
        (numerator, denominator)
    } else {
        let numerator = write_copy(numerator, y);
        sub(numerator, &r.y, 1); // [3]
        let denominator = write_copy(denominator, x);
        sub(denominator, &r.x, 1); // [3]
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
pub(super) fn set(r: &mut Affine, x: &FieldElement, y: &FieldElement, negate: bool) {
    assign(&mut r.x, x);
    assign(&mut r.y, y);
    if negate {
        // A valid input point can carry a non-normalized field representation.
        settle(&mut r.y);
        r.y.negate_in_place(1);
        settle(&mut r.y);
    }
    r.infinity = false;
}

/// `r = r + p`, or `r - p` if `negate`, for the point `p = (x, y)`
pub(super) fn add<H: Secp256k1Hooks>(
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
pub(super) fn double_and_add<H: Secp256k1Hooks>(
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
    x_t.square_in_place(); // [1]
    sub(x_t, &r.x, 1); // [3]
    sub(x_t, x, 1); // [5]
    settle(x_t); // [1]

    // the slope of (r + p) + r is -(slope + 2 y_r / (x_t - x_r))
    let (numerator, denominator) = fraction.parts();
    let numerator = write_copy(numerator, &r.y);
    numerator.double_in_place(); // [2]
    let denominator = write_copy(denominator, x_t);
    sub(denominator, &r.x, 1); // [3]
    if denominator.is_zero() {
        double(r, hooks);
        add(r, x, y, negate, hooks);
        return;
    }
    let mut negated_slope = MaybeUninit::uninit();
    // SAFETY: both parts were written, and the denominator is not zero
    let negated_slope = unsafe { fraction.divide(&mut negated_slope, hooks) };
    negated_slope.add_in_place(slope); // [2]
    settle(negated_slope); // [1]

    // x3 = slope_2² - x_r - x_t, y3 = slope_2 (x_r - x3) - y_r = -slope_2 (x3 - x_r) - y_r
    let mut x3 = MaybeUninit::uninit();
    let x3 = write_copy(&mut x3, negated_slope);
    x3.square_in_place(); // [1]
    sub(x3, &r.x, 1); // [3]
    sub(x3, x_t, 1); // [5]

    subtract_from(&mut r.x, x3, 1); // [7]
    r.x.mul_in_place(negated_slope); // [1]
    subtract_from(&mut r.y, &r.x, 1); // [3]
    assign(&mut r.x, x3); // [5]
    settle(&mut r.x);
    settle(&mut r.y);
}
