//! The two `G2` computations of the pairing check in affine coordinates, with their field
//! inversions supplied from outside: the line precomputation of the Miller loop (the
//! double-and-add of the loop count on `Q`, one line per step) and the subgroup membership
//! test of `Q`. An affine step costs about half the field multiplications of a projective one
//! but needs an inversion; with a hinted inverse, checked with one multiplication, the whole
//! chain of a point costs one query and about half the delegations.
//!
//! The chains are deterministic functions of `Q`, so a prover computes every inverse of a
//! chain in advance ([`line_inverses`], [`subgroup_inverses`]) and the verifier consumes them
//! in the same order ([`prepare_into`], [`membership`] with its own [`Inverter`], or the
//! array-backed [`prepare_with_inverses`], [`is_in_subgroup_with_inverses`]). Each hint
//! is checked to be the inverse of the step's denominator (the inverse is unique, so a hint
//! cannot steer the result: a wrong one is a broken prover). A chain hits a zero denominator
//! only on exceptional inputs (a point of even order, or a step where the two points
//! coincide or are opposite); for a point of the prime-order subgroup neither computation
//! does, which was also checked on the specific additions of the membership test. Such
//! inputs get no hints, and the caller falls back to the projective computations.
//!
//! Lines are normalized to a leading coefficient of one: `l = y_P - λ x_P w + (λ x_T - y_T) w³`
//! for the D-type twist, which is the projective line divided by an element of `Fq2`, a
//! scaling the pairing check is invariant to (every element of a proper subfield is an
//! `r`-th residue).

use super::g2::{P_POWER_ENDOMORPHISM_COEFF_0, P_POWER_ENDOMORPHISM_COEFF_1};
use super::pairing_impl::{mul_by_char, write_coeff, G2PreparedNoAlloc, BN254_NUM_ELL_COEFFS};
use super::{Config, G2Affine};
use crate::bn254::fields::Fq2;
use crate::extension_tower::*;
use ark_ec::bn::g2::EllCoeff;
use ark_ec::bn::BnConfig;
use ark_ff::{AdditiveGroup, Field, One};
use core::mem::MaybeUninit;

/// The BN parameter `x` (positive, 63 bits)
const X: u64 = 4965661367192848881;
const _: () = assert!(
    Config::X.len() == 1 && Config::X[0] == X && !Config::X_IS_NEGATIVE && X.leading_zeros() == 1
);

/// Inverses of the line precomputation: one per doubling and per non-zero digit of the loop
/// count, and two for the additions of the Frobenius images
pub const LINE_INVERSES: usize = BN254_NUM_ELL_COEFFS;
/// Inverses of the membership test: `[x]Q` by 62 doublings and `popcount(x) - 1 = 27`
/// additions, the doubling of `ψ³([x]Q)`, then the three additions of the identity
pub const SUBGROUP_INVERSES: usize = 62 + (X.count_ones() as usize - 1) + 1 + 3;

/// Where the inverses of a chain come from
pub trait Inverter {
    /// The inverse of `den`, `None` if it is zero or the hint is not the inverse
    fn inverse(&mut self, den: &Fq2) -> Option<Fq2>;
}

/// Computes the inverses (the prover), recording them in order
struct Computing<'a> {
    out: &'a mut [Fq2],
    next: usize,
}

impl Inverter for Computing<'_> {
    fn inverse(&mut self, den: &Fq2) -> Option<Fq2> {
        let inverse = den.inverse()?;
        self.out[self.next] = inverse;
        self.next += 1;
        Some(inverse)
    }
}

/// Takes the inverses from the hints (the verifier), checking each one
struct Hinted<'a> {
    hints: &'a [Fq2],
    next: usize,
}

impl Inverter for Hinted<'_> {
    fn inverse(&mut self, den: &Fq2) -> Option<Fq2> {
        let hint = self.hints[self.next];
        self.next += 1;
        verify_inverse(den, hint)
    }
}

/// `hint` if it is the inverse of `den` (one multiplication), for an [`Inverter`] over hints
pub fn verify_inverse(den: &Fq2, hint: Fq2) -> Option<Fq2> {
    fp2_tmp!(product = den);
    fp2_mul_assign(product, &hint);
    product.is_one().then_some(hint)
}

/// An affine point of the twist, never the point at infinity
struct Point {
    x: Fq2,
    y: Fq2,
}

impl Point {
    fn from_affine(p: &G2Affine) -> Self {
        Self { x: p.x, y: p.y }
    }

    /// `self = 2 self`, returning the slope `λ = 3x² / 2y`; `None` for `y = 0` (a point of
    /// order 2) or a wrong hint
    fn double(&mut self, inverter: &mut impl Inverter) -> Option<Fq2> {
        fp2_tmp!(den = &self.y);
        fp2_double_in_place(den);
        let inverse = inverter.inverse(den)?;
        fp2_tmp!(lambda = &self.x);
        fp2_square_in_place(lambda);
        fp2_tmp!(t = lambda);
        fp2_double_in_place(t);
        fp2_add_assign(lambda, t);
        fp2_mul_assign(lambda, &inverse);
        self.move_along(lambda, None);
        Some(*lambda)
    }

    /// `self = self + q`, returning the slope; `None` if the points share their `x` (equal
    /// or opposite) or on a wrong hint
    fn add(&mut self, q: &Point, inverter: &mut impl Inverter) -> Option<Fq2> {
        fp2_tmp!(den = &q.x);
        fp2_sub_assign(den, &self.x);
        let inverse = inverter.inverse(den)?;
        fp2_tmp!(lambda = &q.y);
        fp2_sub_assign(lambda, &self.y);
        fp2_mul_assign(lambda, &inverse);
        self.move_along(lambda, Some(&q.x));
        Some(*lambda)
    }

    /// The point where the line of slope `λ` through `self` (and `other_x`, for an addition)
    /// meets the curve again, negated: `x' = λ² - x - x_other`, `y' = λ (x - x') - y`
    fn move_along(&mut self, lambda: &Fq2, other_x: Option<&Fq2>) {
        fp2_tmp!(x3 = lambda);
        fp2_square_in_place(x3);
        fp2_sub_assign(x3, &self.x);
        fp2_sub_assign(x3, other_x.unwrap_or(&self.x));
        fp2_sub_assign(&mut self.x, x3);
        fp2_mul_assign(&mut self.x, lambda);
        fp2_sub_assign(&mut self.x, &self.y);
        fp2_assign(&mut self.y, &self.x);
        fp2_assign(&mut self.x, x3);
    }

    /// The line of slope `λ` through the point before the last step (`x`, `y` given), as the
    /// Miller loop uses it: `(1, -λ, λ x - y)`
    fn line(out: &mut MaybeUninit<EllCoeff<Config>>, lambda: &Fq2, x: &Fq2, y: &Fq2) {
        fp2_tmp!(c2 = lambda);
        fp2_mul_assign(c2, x);
        fp2_sub_assign(c2, y);
        fp2_tmp!(c1 = lambda);
        fp2_neg_in_place(c1);
        write_coeff(out, &Fq2::ONE, c1, c2);
    }

    /// `ψ`, the untwist-Frobenius-twist endomorphism
    fn psi_in_place(&mut self) {
        self.x.c1.neg_in_place();
        fp2_mul_assign(&mut self.x, &P_POWER_ENDOMORPHISM_COEFF_0);
        self.y.c1.neg_in_place();
        fp2_mul_assign(&mut self.y, &P_POWER_ENDOMORPHISM_COEFF_1);
    }

    fn eq(&self, other: &Point) -> bool {
        self.x == other.x && self.y == other.y
    }
}

/// The line precomputation over `Q` (not the point at infinity), with the inverses from
/// `inverter`. `None` on a zero denominator or a rejected hint.
pub fn prepare(q: &G2Affine, inverter: &mut impl Inverter) -> Option<G2PreparedNoAlloc> {
    let mut out = MaybeUninit::uninit();
    prepare_into(q, inverter, &mut out)?;
    // SAFETY: `prepare_into` initialized it
    Some(unsafe { out.assume_init() })
}

/// [`prepare`] writing the lines into `out` in place (they are 16 KB, and a verifier stores
/// them where the Miller loop reads them): `out` is initialized on `Some`.
pub fn prepare_into(
    q: &G2Affine,
    inverter: &mut impl Inverter,
    out: &mut MaybeUninit<G2PreparedNoAlloc>,
) -> Option<()> {
    debug_assert!(!q.infinity);
    let out_ptr = out.as_mut_ptr();
    // SAFETY: a projection of the place being initialized, viewed as an array of
    // possibly-uninitialized coefficients; nothing is read from it before it is written
    let ell_coeffs: &mut [MaybeUninit<EllCoeff<Config>>; BN254_NUM_ELL_COEFFS] = unsafe {
        &mut *core::ptr::addr_of_mut!((*out_ptr).ell_coeffs)
            .cast::<[MaybeUninit<EllCoeff<Config>>; BN254_NUM_ELL_COEFFS]>()
    };
    let mut i = 0;
    let mut r = Point::from_affine(q);
    let plus_q = Point::from_affine(q);
    let neg_q = -*q;
    let minus_q = Point::from_affine(&neg_q);
    for digit in Config::ATE_LOOP_COUNT.iter().rev().skip(1) {
        // the line is through the point before the step
        let (x, y) = (r.x, r.y);
        let lambda = r.double(inverter)?;
        Point::line(&mut ell_coeffs[i], &lambda, &x, &y);
        i += 1;
        let addend = match digit {
            1 => &plus_q,
            -1 => &minus_q,
            _ => continue,
        };
        let (x, y) = (r.x, r.y);
        let lambda = r.add(addend, inverter)?;
        Point::line(&mut ell_coeffs[i], &lambda, &x, &y);
        i += 1;
    }
    // the two Frobenius images (the loop count is 6x + 2 + p - p² + p³)
    let q1 = mul_by_char(*q);
    let mut q2 = mul_by_char(q1);
    q2.y = -q2.y;
    for image in [q1, q2] {
        let (x, y) = (r.x, r.y);
        let lambda = r.add(&Point::from_affine(&image), inverter)?;
        Point::line(&mut ell_coeffs[i], &lambda, &x, &y);
        i += 1;
    }
    debug_assert_eq!(i, BN254_NUM_ELL_COEFFS);
    // SAFETY: every coefficient was written above; the flags complete the value
    unsafe {
        core::ptr::addr_of_mut!((*out_ptr).infinity).write(false);
        core::ptr::addr_of_mut!((*out_ptr).affine_lines).write(true);
    }
    Some(())
}

/// The inverses of the line precomputation over `Q` (not the point at infinity), in the
/// order the precomputation needs them; `None` if the chain hits a zero denominator (never
/// for a point of the prime-order subgroup)
pub fn line_inverses(q: &G2Affine) -> Option<[Fq2; LINE_INVERSES]> {
    let mut out = [Fq2::ZERO; LINE_INVERSES];
    let mut inverter = Computing {
        out: &mut out,
        next: 0,
    };
    prepare(q, &mut inverter)?;
    debug_assert_eq!(inverter.next, LINE_INVERSES);
    Some(out)
}

/// The line precomputation over `Q` (not the point at infinity) from hinted inverses, each
/// checked; `None` on a rejected hint or a zero denominator
pub fn prepare_with_inverses(
    q: &G2Affine,
    inverses: &[Fq2; LINE_INVERSES],
) -> Option<G2PreparedNoAlloc> {
    let mut inverter = Hinted {
        hints: inverses,
        next: 0,
    };
    prepare(q, &mut inverter)
}

/// The lines of `Q` exactly as a verifier with hints computes them: the affine ones, or the
/// projective ones for an exceptional chain. A prover that computes a residue witness
/// (`crate::residue_witness`) for a verifier's Miller loop output must use these lines: the
/// affine and the projective lines differ by factors in `Fq2`, which the pairing check is
/// invariant to but the witness equation `f s = c^λ` is not.
pub fn prepare_as_verifier(q: &G2Affine) -> G2PreparedNoAlloc {
    match line_inverses(q) {
        Some(inverses) => {
            prepare_with_inverses(q, &inverses).expect("the inverses were just computed")
        }
        None => G2PreparedNoAlloc::from(*q),
    }
}

/// The membership test `[x + 1]Q + ψ([x]Q) + ψ²([x]Q) = ψ³([2x]Q)` (see `g2_subgroup`) in
/// affine coordinates, for `Q` on the twist and not the point at infinity. `None` on a zero
/// denominator (an exceptional input, never a point of the subgroup) or a rejected hint.
pub fn membership(q: &G2Affine, inverter: &mut impl Inverter) -> Option<bool> {
    debug_assert!(!q.infinity);
    let base = Point::from_affine(q);
    // a = [x]Q: the top bit of x gives Q, then the 62 bits below
    let mut a = Point::from_affine(q);
    for i in (0..62).rev() {
        a.double(inverter)?;
        if (X >> i) & 1 == 1 {
            a.add(&base, inverter)?;
        }
    }
    // b = ψ(a), c = ψ²(a), d = ψ³([2]a)
    let mut b = Point { x: a.x, y: a.y };
    b.psi_in_place();
    let mut c = Point { x: b.x, y: b.y };
    c.psi_in_place();
    let mut d = Point { x: c.x, y: c.y };
    d.psi_in_place();
    d.double(inverter)?;
    // [x + 1]Q + b + c == d
    a.add(&base, inverter)?;
    a.add(&b, inverter)?;
    a.add(&c, inverter)?;
    Some(a.eq(&d))
}

/// The inverses of the membership test of `Q` (on the twist, not the point at infinity), in
/// the order the test needs them; `None` if the chain hits a zero denominator
pub fn subgroup_inverses(q: &G2Affine) -> Option<[Fq2; SUBGROUP_INVERSES]> {
    let mut out = [Fq2::ZERO; SUBGROUP_INVERSES];
    let mut inverter = Computing {
        out: &mut out,
        next: 0,
    };
    membership(q, &mut inverter)?;
    debug_assert_eq!(inverter.next, SUBGROUP_INVERSES);
    Some(out)
}

/// The membership test of `Q` (on the twist, not the point at infinity) from hinted
/// inverses, each checked; `None` on a rejected hint or a zero denominator
pub fn is_in_subgroup_with_inverses(
    q: &G2Affine,
    inverses: &[Fq2; SUBGROUP_INVERSES],
) -> Option<bool> {
    let mut inverter = Hinted {
        hints: inverses,
        next: 0,
    };
    membership(q, &mut inverter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bn254::curves::g2_subgroup::is_in_subgroup;
    use crate::bn254::curves::{Bn254, G1Affine};
    use ark_ec::pairing::Pairing;
    use ark_ff::UniformRand;
    use ark_std::test_rng;

    #[test]
    fn affine_lines_give_the_same_pairing() {
        let mut rng = test_rng();
        for _ in 0..4 {
            let p = G1Affine::rand(&mut rng);
            let q = G2Affine::rand(&mut rng);
            let inverses = line_inverses(&q).expect("a subgroup point has no exceptional step");
            let prepared = prepare_with_inverses(&q, &inverses).expect("the inverses are right");
            let projective: G2PreparedNoAlloc = q.into();
            let with_affine = Bn254::multi_miller_loop_prepared([p], [&prepared]);
            let with_projective = Bn254::multi_miller_loop([p], [projective]);
            assert_eq!(
                Bn254::final_exponentiation(ark_ec::pairing::MillerLoopOutput(with_affine))
                    .unwrap(),
                Bn254::final_exponentiation(with_projective).unwrap()
            );
            // a wrong inverse is rejected
            let mut wrong = inverses;
            wrong[17] += Fq2::ONE;
            assert!(prepare_with_inverses(&q, &wrong).is_none());
            // the identity e(P, Q) e(-P, Q) = 1 through the affine lines
            let identity = Bn254::multi_miller_loop_prepared([p, -p], [&prepared, &prepared]);
            assert!(
                Bn254::final_exponentiation(ark_ec::pairing::MillerLoopOutput(identity))
                    .unwrap()
                    .0
                    .is_one()
            );
        }
    }

    #[test]
    fn membership_matches_the_projective_test() {
        let mut rng = test_rng();
        for _ in 0..4 {
            let q = G2Affine::rand(&mut rng);
            let inverses = subgroup_inverses(&q).expect("a subgroup point has no exceptional step");
            assert_eq!(is_in_subgroup_with_inverses(&q, &inverses), Some(true));
            assert!(is_in_subgroup(&q));
            let mut wrong = inverses;
            wrong[40] += Fq2::ONE;
            assert!(is_in_subgroup_with_inverses(&q, &wrong).is_none());
        }
        // points of the twist outside the subgroup
        for _ in 0..4 {
            let q = crate::bn254::curves::g2_subgroup::tests::random_curve_point(&mut rng);
            assert!(!is_in_subgroup(&q));
            match subgroup_inverses(&q) {
                Some(inverses) => {
                    assert_eq!(is_in_subgroup_with_inverses(&q, &inverses), Some(false))
                }
                None => {} // an exceptional chain: the caller falls back
            }
        }
    }
}
