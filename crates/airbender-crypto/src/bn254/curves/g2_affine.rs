//! The two `G2` computations of the pairing check in affine coordinates, with their field
//! divisions supplied from outside: the line precomputation of the Miller loop (the
//! double-and-add of the loop count on `Q`, one line per step) and the subgroup membership
//! test of `Q`. An affine step costs about half the field multiplications of a projective one
//! but needs a division (the slope); with a hinted quotient, checked with one multiplication,
//! the whole chain of a point costs one query and less than half the delegations.
//!
//! The chains are deterministic functions of `Q`, so a prover computes every slope of a chain
//! in advance ([`line_slopes`], [`subgroup_slopes`]) and the verifier consumes them in the
//! same order ([`prepare_into`], [`membership`] with its own [`Divider`], or the array-backed
//! [`prepare_with_slopes`], [`is_in_subgroup_with_slopes`]). Each hint is checked to be the
//! quotient of the step (it is unique, so a hint cannot steer the result: a wrong one is a
//! broken prover). A chain hits a zero denominator only on exceptional inputs (a point of even
//! order, or a step where the two points coincide or are opposite); for a point of the
//! prime-order subgroup neither computation does, which was also checked on the specific
//! additions of the membership test. Such inputs get no hints, and the caller falls back to
//! the projective computations.
//!
//! Lines are normalized to a leading coefficient of one: `l = y_P - λ x_P w + (λ x_T - y_T) w³`
//! for the D-type twist, which is the projective line divided by an element of `Fq2`, a
//! scaling the pairing check is invariant to (every element of a proper subfield is an
//! `r`-th residue).

use super::g2::{P_POWER_ENDOMORPHISM_COEFF_0, P_POWER_ENDOMORPHISM_COEFF_1};
use super::pairing_impl::{mul_by_char, write_coeff, G2PreparedNoAlloc, BN254_NUM_ELL_COEFFS};
use super::{Config, G2Affine};
use crate::affine_glv::Divider;
use crate::bn254::fields::Fq2;
use crate::extension_tower::*;
use ark_ec::bn::g2::EllCoeff;
use ark_ec::bn::BnConfig;
use ark_ff::{AdditiveGroup, Field, Zero};
use core::mem::MaybeUninit;

/// The BN parameter `x` (positive, 63 bits)
const X: u64 = 4965661367192848881;
const _: () = assert!(
    Config::X.len() == 1 && Config::X[0] == X && !Config::X_IS_NEGATIVE && X.leading_zeros() == 1
);

/// Divisions of the line precomputation: one per doubling and per non-zero digit of the loop
/// count, and two for the additions of the Frobenius images
pub const LINE_DIVISIONS: usize = BN254_NUM_ELL_COEFFS;
/// Divisions of the membership test: `[x]Q` by 62 doublings and `popcount(x) - 1 = 27`
/// additions, the doubling of `ψ³([x]Q)`, then the three additions of the identity
pub const SUBGROUP_DIVISIONS: usize = 62 + (X.count_ones() as usize - 1) + 1 + 3;

/// Computes the quotients (the prover), recording them in order
struct Computing<'a> {
    out: &'a mut [Fq2],
    next: usize,
}

impl Divider<Fq2> for Computing<'_> {
    fn divide<'a>(
        &mut self,
        fraction: &mut [Fq2; 2],
        quotient: &'a mut MaybeUninit<Fq2>,
    ) -> &'a mut Fq2 {
        let inverse = fraction[1].inverse().expect("the denominator is not zero");
        let quotient = quotient.write(fraction[0] * inverse);
        self.out[self.next] = *quotient;
        self.next += 1;
        quotient
    }
}

/// Takes the quotients from the hints (the verifier), checking each one: a wrong one is
/// recorded, and fails the chain
struct Hinted<'a> {
    hints: &'a [Fq2],
    next: usize,
    wrong: bool,
}

impl Divider<Fq2> for Hinted<'_> {
    fn divide<'a>(
        &mut self,
        fraction: &mut [Fq2; 2],
        quotient: &'a mut MaybeUninit<Fq2>,
    ) -> &'a mut Fq2 {
        let quotient = quotient.write(self.hints[self.next]);
        self.next += 1;
        self.wrong |= !verify_quotient(fraction, quotient);
        quotient
    }
}

/// Whether `quotient` is the quotient of `fraction` (one multiplication), for a [`Divider`]
/// over hints. The denominator must not be zero. The fraction is overwritten.
#[inline(always)]
pub fn verify_quotient(fraction: &mut [Fq2; 2], quotient: &Fq2) -> bool {
    let [numerator, denominator] = fraction;
    fp2_mul_assign(denominator, quotient);
    fp2_sub_assign(denominator, numerator);
    denominator.is_zero()
}

/// A numerator and a denominator, adjacent as [`Divider::divide`] takes them
struct Fraction([MaybeUninit<Fq2>; 2]);

impl Fraction {
    #[inline(always)]
    fn uninit() -> Self {
        Self([const { MaybeUninit::uninit() }; 2])
    }

    /// Initializes the fraction with copies of a numerator and a denominator, to be changed
    /// in place
    #[inline(always)]
    fn set(&mut self, numerator: &Fq2, denominator: &Fq2) -> (&mut Fq2, &mut Fq2) {
        let [n, d] = &mut self.0;
        (fp2_init(n, numerator), fp2_init(d, denominator))
    }

    /// The quotient into `quotient`
    ///
    /// # Safety
    /// Both parts must be initialized, and the denominator must not be zero.
    #[inline(always)]
    unsafe fn divide<'a>(
        &mut self,
        quotient: &'a mut MaybeUninit<Fq2>,
        divider: &mut impl Divider<Fq2>,
    ) -> &'a mut Fq2 {
        // SAFETY: `MaybeUninit<T>` has the layout of `T`, and the parts are initialized
        let fraction = unsafe { &mut *(&mut self.0 as *mut [MaybeUninit<Fq2>; 2]).cast() };
        divider.divide(fraction, quotient)
    }
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

    /// `self = 2 self`, with the slope `λ = 3x² / 2y` into `slope`; `None` for `y = 0` (a
    /// point of order 2)
    fn double<'a>(
        &mut self,
        slope: &'a mut MaybeUninit<Fq2>,
        divider: &mut impl Divider<Fq2>,
    ) -> Option<&'a mut Fq2> {
        let mut fraction = Fraction::uninit();
        let (numerator, denominator) = fraction.set(&self.x, &self.y);
        fp2_double_in_place(denominator);
        if denominator.is_zero() {
            return None;
        }
        fp2_square_in_place(numerator);
        fp2_tmp!(square = numerator);
        fp2_double_in_place(numerator);
        fp2_add_assign(numerator, square);
        // SAFETY: both parts were set, and the denominator is not zero
        let slope = unsafe { fraction.divide(slope, divider) };
        self.move_along(slope, None);
        Some(slope)
    }

    /// `self = self + q`, with the slope into `slope`; `None` if the points share their `x`
    /// (equal or opposite)
    fn add<'a>(
        &mut self,
        q: &Point,
        slope: &'a mut MaybeUninit<Fq2>,
        divider: &mut impl Divider<Fq2>,
    ) -> Option<&'a mut Fq2> {
        let mut fraction = Fraction::uninit();
        let (numerator, denominator) = fraction.set(&q.y, &q.x);
        fp2_sub_assign(denominator, &self.x);
        if denominator.is_zero() {
            return None;
        }
        fp2_sub_assign(numerator, &self.y);
        // SAFETY: both parts were set, and the denominator is not zero
        let slope = unsafe { fraction.divide(slope, divider) };
        self.move_along(slope, Some(&q.x));
        Some(slope)
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

    /// The line of slope `λ` through the point `(x, y)`, as the Miller loop uses it:
    /// `(1, -λ, λ x - y)`
    fn line(out: &mut MaybeUninit<EllCoeff<Config>>, lambda: &Fq2, x: &Fq2, y: &Fq2) {
        fp2_tmp!(c2 = lambda);
        fp2_mul_assign(c2, x);
        fp2_sub_assign(c2, y);
        fp2_tmp!(c1 = lambda);
        fp2_neg_in_place(c1);
        write_coeff(out, &Fq2::ONE, c1, c2);
    }

    /// A step of the line precomputation: `self = 2 self`, or `self + q`, with the line
    /// through the points of the step into `out`
    fn step(
        &mut self,
        q: Option<&Point>,
        out: &mut MaybeUninit<EllCoeff<Config>>,
        divider: &mut impl Divider<Fq2>,
    ) -> Option<()> {
        // the line is through the point before the step
        fp2_tmp!(x = &self.x);
        fp2_tmp!(y = &self.y);
        let mut slope = MaybeUninit::uninit();
        let slope = match q {
            Some(q) => self.add(q, &mut slope, divider)?,
            None => self.double(&mut slope, divider)?,
        };
        Self::line(out, slope, x, y);
        Some(())
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

/// The line precomputation over `Q` (not the point at infinity), with the divisions of
/// `divider`. `None` on a zero denominator.
pub fn prepare(q: &G2Affine, divider: &mut impl Divider<Fq2>) -> Option<G2PreparedNoAlloc> {
    let mut out = MaybeUninit::uninit();
    prepare_into(q, divider, &mut out)?;
    // SAFETY: `prepare_into` initialized it
    Some(unsafe { out.assume_init() })
}

/// [`prepare`] writing the lines into `out` in place (they are 16 KB, and a verifier stores
/// them where the Miller loop reads them): `out` is initialized on `Some`.
pub fn prepare_into(
    q: &G2Affine,
    divider: &mut impl Divider<Fq2>,
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
        r.step(None, &mut ell_coeffs[i], divider)?;
        i += 1;
        let addend = match digit {
            1 => &plus_q,
            -1 => &minus_q,
            _ => continue,
        };
        r.step(Some(addend), &mut ell_coeffs[i], divider)?;
        i += 1;
    }
    // the two Frobenius images (the loop count is 6x + 2 + p - p² + p³)
    let q1 = mul_by_char(*q);
    let mut q2 = mul_by_char(q1);
    q2.y = -q2.y;
    for image in [q1, q2] {
        r.step(
            Some(&Point::from_affine(&image)),
            &mut ell_coeffs[i],
            divider,
        )?;
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

/// The slopes of the line precomputation over `Q` (not the point at infinity), in the order
/// the precomputation needs them; `None` if the chain hits a zero denominator (never for a
/// point of the prime-order subgroup)
pub fn line_slopes(q: &G2Affine) -> Option<[Fq2; LINE_DIVISIONS]> {
    let mut out = [Fq2::ZERO; LINE_DIVISIONS];
    let mut divider = Computing {
        out: &mut out,
        next: 0,
    };
    prepare(q, &mut divider)?;
    debug_assert_eq!(divider.next, LINE_DIVISIONS);
    Some(out)
}

/// The line precomputation over `Q` (not the point at infinity) from hinted slopes, each
/// checked; `None` on a rejected hint or a zero denominator
pub fn prepare_with_slopes(
    q: &G2Affine,
    slopes: &[Fq2; LINE_DIVISIONS],
) -> Option<G2PreparedNoAlloc> {
    let mut divider = Hinted {
        hints: slopes,
        next: 0,
        wrong: false,
    };
    let prepared = prepare(q, &mut divider)?;
    (!divider.wrong).then_some(prepared)
}

/// The lines of `Q` exactly as a verifier with hints computes them: the affine ones, or the
/// projective ones for an exceptional chain. A prover that computes a residue witness
/// (`crate::residue_witness`) for a verifier's Miller loop output must use these lines: the
/// affine and the projective lines differ by factors in `Fq2`, which the pairing check is
/// invariant to but the witness equation `f s = c^λ` is not.
pub fn prepare_as_verifier(q: &G2Affine) -> G2PreparedNoAlloc {
    match prepare(q, &mut crate::affine_glv::InvertingDivider) {
        Some(prepared) => prepared,
        None => G2PreparedNoAlloc::from(*q),
    }
}

/// The membership test `[x + 1]Q + ψ([x]Q) + ψ²([x]Q) = ψ³([2x]Q)` (see `g2_subgroup`) in
/// affine coordinates, for `Q` on the twist and not the point at infinity. `None` on a zero
/// denominator (an exceptional input, never a point of the subgroup).
pub fn membership(q: &G2Affine, divider: &mut impl Divider<Fq2>) -> Option<bool> {
    debug_assert!(!q.infinity);
    let mut slope = MaybeUninit::uninit();
    let base = Point::from_affine(q);
    // a = [x]Q: the top bit of x gives Q, then the 62 bits below
    let mut a = Point::from_affine(q);
    for i in (0..62).rev() {
        a.double(&mut slope, divider)?;
        if (X >> i) & 1 == 1 {
            a.add(&base, &mut slope, divider)?;
        }
    }
    // b = ψ(a), c = ψ²(a), d = ψ³([2]a)
    let mut b = Point { x: a.x, y: a.y };
    b.psi_in_place();
    let mut c = Point { x: b.x, y: b.y };
    c.psi_in_place();
    let mut d = Point { x: c.x, y: c.y };
    d.psi_in_place();
    d.double(&mut slope, divider)?;
    // [x + 1]Q + b + c == d
    a.add(&base, &mut slope, divider)?;
    a.add(&b, &mut slope, divider)?;
    a.add(&c, &mut slope, divider)?;
    Some(a.eq(&d))
}

/// The slopes of the membership test of `Q` (on the twist, not the point at infinity), in
/// the order the test needs them; `None` if the chain hits a zero denominator
pub fn subgroup_slopes(q: &G2Affine) -> Option<[Fq2; SUBGROUP_DIVISIONS]> {
    let mut out = [Fq2::ZERO; SUBGROUP_DIVISIONS];
    let mut divider = Computing {
        out: &mut out,
        next: 0,
    };
    membership(q, &mut divider)?;
    debug_assert_eq!(divider.next, SUBGROUP_DIVISIONS);
    Some(out)
}

/// The membership test of `Q` (on the twist, not the point at infinity) from hinted slopes,
/// each checked; `None` on a rejected hint or a zero denominator
pub fn is_in_subgroup_with_slopes(
    q: &G2Affine,
    slopes: &[Fq2; SUBGROUP_DIVISIONS],
) -> Option<bool> {
    let mut divider = Hinted {
        hints: slopes,
        next: 0,
        wrong: false,
    };
    let is_member = membership(q, &mut divider)?;
    (!divider.wrong).then_some(is_member)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bn254::curves::g2_subgroup::is_in_subgroup;
    use crate::bn254::curves::{Bn254, G1Affine};
    use ark_ec::pairing::Pairing;
    use ark_ff::One;
    use ark_ff::UniformRand;
    use ark_std::test_rng;

    #[test]
    fn affine_lines_give_the_same_pairing() {
        let mut rng = test_rng();
        for _ in 0..4 {
            let p = G1Affine::rand(&mut rng);
            let q = G2Affine::rand(&mut rng);
            let slopes = line_slopes(&q).expect("a subgroup point has no exceptional step");
            let prepared = prepare_with_slopes(&q, &slopes).expect("the slopes are right");
            let projective: G2PreparedNoAlloc = q.into();
            let with_affine = Bn254::multi_miller_loop_prepared([p], [&prepared]);
            let with_projective = Bn254::multi_miller_loop([p], [projective]);
            assert_eq!(
                Bn254::final_exponentiation(ark_ec::pairing::MillerLoopOutput(with_affine))
                    .unwrap(),
                Bn254::final_exponentiation(with_projective).unwrap()
            );
            // a wrong slope is rejected
            let mut wrong = slopes;
            wrong[17] += Fq2::ONE;
            assert!(prepare_with_slopes(&q, &wrong).is_none());
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
            let slopes = subgroup_slopes(&q).expect("a subgroup point has no exceptional step");
            assert_eq!(is_in_subgroup_with_slopes(&q, &slopes), Some(true));
            assert!(is_in_subgroup(&q));
            let mut wrong = slopes;
            wrong[40] += Fq2::ONE;
            assert!(is_in_subgroup_with_slopes(&q, &wrong).is_none());
        }
        // points of the twist outside the subgroup
        for _ in 0..4 {
            let q = crate::bn254::curves::g2_subgroup::tests::random_curve_point(&mut rng);
            assert!(!is_in_subgroup(&q));
            match subgroup_slopes(&q) {
                Some(slopes) => {
                    assert_eq!(is_in_subgroup_with_slopes(&q, &slopes), Some(false))
                }
                None => {} // an exceptional chain: the caller falls back
            }
        }
    }

    /// The Miller loop over all the pairs at once, with normalized `G1` points, gives the
    /// pairing product of the plain one, with affine and with projective lines, and takes the
    /// initial value as the plain one does
    #[test]
    fn normalized_miller_loop_gives_the_same_pairing() {
        use crate::affine_glv::InvertingDivider;
        use crate::bn254::curves::G1Evaluation;
        use crate::bn254::Fq12;
        use ark_ec::pairing::MillerLoopOutput;
        let mut rng = test_rng();
        let final_exponentiation =
            |f: Fq12| Bn254::final_exponentiation(MillerLoopOutput(f)).unwrap().0;
        for count in 1..4 {
            let g1: Vec<G1Affine> = (0..count).map(|_| G1Affine::rand(&mut rng)).collect();
            let g2: Vec<G2Affine> = (0..count).map(|_| G2Affine::rand(&mut rng)).collect();
            let expected = final_exponentiation(
                Bn254::multi_miller_loop(g1.iter().copied(), g2.iter().copied()).0,
            );
            let evaluations: Vec<G1Evaluation> = g1
                .iter()
                .map(|p| G1Evaluation::new(p, &mut InvertingDivider))
                .collect();
            for affine in [false, true] {
                let prepared: Vec<G2PreparedNoAlloc> = g2
                    .iter()
                    .enumerate()
                    // a mix of the two kinds of lines, too
                    .map(|(i, q)| {
                        if affine || i == 1 {
                            prepare_as_verifier(q)
                        } else {
                            G2PreparedNoAlloc::from(*q)
                        }
                    })
                    .collect();
                let pairs = || evaluations.iter().zip(prepared.iter());
                let f = Bn254::multi_miller_loop_normalized(None, pairs());
                assert_eq!(final_exponentiation(f), expected);

                let d = Fq12::rand(&mut rng);
                let c = d.inverse().unwrap();
                let l = Bn254::multi_miller_loop_normalized(Some((&d, &c)), pairs());
                assert_eq!(l, f * d.pow(crate::residue_witness::bn254::SIX_X_PLUS_2));
            }
        }
    }
}
