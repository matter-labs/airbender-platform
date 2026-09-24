//! G2 subgroup membership test of bn254, in Jacobian coordinates over the in-place tower.
//!
//! A point `Q` of the twist `E'(Fq2)` (assumed to be on the curve) is in the prime-order
//! subgroup `G2` iff
//!
//! `[x + 1]Q + ψ([x]Q) + ψ²([x]Q) = ψ³([2x]Q)`
//!
//! where `x` is the BN parameter and `ψ` the untwist-Frobenius-twist endomorphism
//! (El Housni, Guillevic, Piellard, "Co-factor clearing and subgroup membership testing on
//! pairing-friendly curves", <https://eprint.iacr.org/2022/348>, sections 3 and 5.1; the same
//! test is used by gnark-crypto). Compared with the `[6x²]Q = ψ(Q)` test it replaces, it needs
//! a 63-bit instead of a 128-bit scalar multiplication, and the group law below updates the
//! delegated base field in place instead of going through the by-value arkworks arithmetic.

use core::mem::MaybeUninit;

use ark_ec::bn::BnConfig;
use ark_ff::{AdditiveGroup, Field, Zero};

use super::g2::{G2Affine, P_POWER_ENDOMORPHISM_COEFF_0, P_POWER_ENDOMORPHISM_COEFF_1};
use crate::bn254::fields::Fq2;
use crate::extension_tower::*;

/// The BN parameter `x` (positive, 63 bits)
const X: u64 = 4965661367192848881;
const _: () = assert!(
    super::Config::X.len() == 1 && super::Config::X[0] == X && !super::Config::X_IS_NEGATIVE
);

/// Jacobian point `(X : Y : Z)` = `(X/Z², Y/Z³)`; the point at infinity iff `Z = 0`
struct G2Jac {
    x: Fq2,
    y: Fq2,
    z: Fq2,
}

macro_rules! g2jac_tmp {
    ($name:ident = $src:expr) => {
        let mut slot = MaybeUninit::<G2Jac>::uninit();
        let $name: &mut G2Jac = G2Jac::init(&mut slot, $src);
    };
}

impl G2Jac {
    /// Initializes `slot` with the affine point `p`: `Z = 1`, or `Z = 0` for infinity
    #[inline(always)]
    fn init_from_affine<'a>(slot: &'a mut MaybeUninit<Self>, p: &G2Affine) -> &'a mut Self {
        // SAFETY: all fields are written before the value is used
        unsafe {
            let ptr = slot.as_mut_ptr();
            fp2_init(&mut *core::ptr::addr_of_mut!((*ptr).x).cast(), &p.x);
            fp2_init(&mut *core::ptr::addr_of_mut!((*ptr).y).cast(), &p.y);
            let z = if p.infinity { &Fq2::ZERO } else { &Fq2::ONE };
            fp2_init(&mut *core::ptr::addr_of_mut!((*ptr).z).cast(), z);
            slot.assume_init_mut()
        }
    }

    /// Initializes `slot` with a copy of `src`, component by component
    #[inline(always)]
    fn init<'a>(slot: &'a mut MaybeUninit<Self>, src: &Self) -> &'a mut Self {
        // SAFETY: all fields are written before the value is used
        unsafe {
            let ptr = slot.as_mut_ptr();
            fp2_init(&mut *core::ptr::addr_of_mut!((*ptr).x).cast(), &src.x);
            fp2_init(&mut *core::ptr::addr_of_mut!((*ptr).y).cast(), &src.y);
            fp2_init(&mut *core::ptr::addr_of_mut!((*ptr).z).cast(), &src.z);
            slot.assume_init_mut()
        }
    }

    #[inline(always)]
    fn is_infinity(&self) -> bool {
        self.z.is_zero()
    }

    #[inline(always)]
    fn set_infinity(&mut self) {
        fp2_assign(&mut self.z, &Fq2::ZERO);
    }

    /// `self = 2 self` ("dbl-2009-l" for `a = 0`: 2M + 5S)
    fn double_in_place(&mut self) {
        // A = X², B = Y², C = B²
        fp2_tmp!(a = &self.x);
        fp2_square_in_place(a);
        fp2_tmp!(b = &self.y);
        fp2_square_in_place(b);
        fp2_tmp!(c = &*b);
        fp2_square_in_place(c);
        // D = 2((X + B)² - A - C)
        fp2_tmp!(d = &self.x);
        fp2_add_assign(d, b);
        fp2_square_in_place(d);
        fp2_sub_assign(d, a);
        fp2_sub_assign(d, c);
        fp2_double_in_place(d);
        // E = 3A
        fp2_tmp!(e = &*a);
        fp2_double_in_place(e);
        fp2_add_assign(e, a);
        // Z3 = 2YZ
        fp2_mul_assign(&mut self.z, &self.y);
        fp2_double_in_place(&mut self.z);
        // X3 = E² - 2D
        fp2_assign(&mut self.x, e);
        fp2_square_in_place(&mut self.x);
        fp2_sub_assign(&mut self.x, d);
        fp2_sub_assign(&mut self.x, d);
        // Y3 = E(D - X3) - 8C
        fp2_sub_assign(d, &self.x);
        fp2_mul_assign(d, e);
        fp2_double_in_place(c);
        fp2_double_in_place(c);
        fp2_double_in_place(c);
        fp2_sub_assign(d, c);
        fp2_assign(&mut self.y, d);
    }

    /// `self = self + p` for an affine `p` ("madd-2007-bl": 7M + 4S)
    fn add_assign_affine(&mut self, p: &G2Affine) {
        if p.infinity {
            return;
        }
        if self.is_infinity() {
            fp2_assign(&mut self.x, &p.x);
            fp2_assign(&mut self.y, &p.y);
            fp2_assign(&mut self.z, &Fq2::ONE);
            return;
        }
        // Z1Z1 = Z1²
        fp2_tmp!(z1z1 = &self.z);
        fp2_square_in_place(z1z1);
        // H = U2 - X1 = X2 Z1Z1 - X1
        fp2_tmp!(h = &p.x);
        fp2_mul_assign(h, z1z1);
        fp2_sub_assign(h, &self.x);
        // r = S2 - Y1 = Y2 Z1 Z1Z1 - Y1 (doubled below)
        fp2_tmp!(r = &p.y);
        fp2_mul_assign(r, &self.z);
        fp2_mul_assign(r, z1z1);
        fp2_sub_assign(r, &self.y);
        if h.is_zero() {
            // same x: either the same point or its negation
            if r.is_zero() {
                self.double_in_place();
            } else {
                self.set_infinity();
            }
            return;
        }
        fp2_double_in_place(r);
        // HH = H²
        fp2_tmp!(hh = &*h);
        fp2_square_in_place(hh);
        // Z3 = (Z1 + H)² - Z1Z1 - HH
        fp2_add_assign(&mut self.z, h);
        fp2_square_in_place(&mut self.z);
        fp2_sub_assign(&mut self.z, z1z1);
        fp2_sub_assign(&mut self.z, hh);
        // I = 4HH, J = H I, V = X1 I
        fp2_double_in_place(hh);
        fp2_double_in_place(hh);
        fp2_mul_assign(h, hh);
        fp2_mul_assign(&mut self.x, hh);
        // X3 = r² - J - 2V
        fp2_tmp!(x3 = &*r);
        fp2_square_in_place(x3);
        fp2_sub_assign(x3, h);
        fp2_sub_assign(x3, &self.x);
        fp2_sub_assign(x3, &self.x);
        // Y3 = r(V - X3) - 2 Y1 J
        fp2_sub_assign(&mut self.x, x3);
        fp2_mul_assign(&mut self.x, r);
        fp2_mul_assign(&mut self.y, h);
        fp2_double_in_place(&mut self.y);
        fp2_sub_assign(&mut self.x, &self.y);
        fp2_assign(&mut self.y, &self.x);
        fp2_assign(&mut self.x, x3);
    }

    /// `self = self + q` ("add-2007-bl": 11M + 5S)
    fn add_assign(&mut self, q: &Self) {
        if q.is_infinity() {
            return;
        }
        if self.is_infinity() {
            fp2_assign(&mut self.x, &q.x);
            fp2_assign(&mut self.y, &q.y);
            fp2_assign(&mut self.z, &q.z);
            return;
        }
        // Z1Z1 = Z1², Z2Z2 = Z2²
        fp2_tmp!(z1z1 = &self.z);
        fp2_square_in_place(z1z1);
        fp2_tmp!(z2z2 = &q.z);
        fp2_square_in_place(z2z2);
        // U1 = X1 Z2Z2, H = U2 - U1 = X2 Z1Z1 - U1
        fp2_tmp!(u1 = &self.x);
        fp2_mul_assign(u1, z2z2);
        fp2_tmp!(h = &q.x);
        fp2_mul_assign(h, z1z1);
        fp2_sub_assign(h, u1);
        // S1 = Y1 Z2 Z2Z2, r = S2 - S1 = Y2 Z1 Z1Z1 - S1 (doubled below)
        fp2_tmp!(s1 = &self.y);
        fp2_mul_assign(s1, &q.z);
        fp2_mul_assign(s1, z2z2);
        fp2_tmp!(r = &q.y);
        fp2_mul_assign(r, &self.z);
        fp2_mul_assign(r, z1z1);
        fp2_sub_assign(r, s1);
        if h.is_zero() {
            // same x: either the same point or its negation
            if r.is_zero() {
                self.double_in_place();
            } else {
                self.set_infinity();
            }
            return;
        }
        fp2_double_in_place(r);
        // Z3 = ((Z1 + Z2)² - Z1Z1 - Z2Z2) H
        fp2_add_assign(&mut self.z, &q.z);
        fp2_square_in_place(&mut self.z);
        fp2_sub_assign(&mut self.z, z1z1);
        fp2_sub_assign(&mut self.z, z2z2);
        fp2_mul_assign(&mut self.z, h);
        // I = (2H)², J = H I, V = U1 I
        fp2_tmp!(i = &*h);
        fp2_double_in_place(i);
        fp2_square_in_place(i);
        fp2_mul_assign(h, i);
        fp2_mul_assign(u1, i);
        // X3 = r² - J - 2V
        fp2_assign(&mut self.x, r);
        fp2_square_in_place(&mut self.x);
        fp2_sub_assign(&mut self.x, h);
        fp2_sub_assign(&mut self.x, u1);
        fp2_sub_assign(&mut self.x, u1);
        // Y3 = r(V - X3) - 2 S1 J
        fp2_sub_assign(u1, &self.x);
        fp2_mul_assign(u1, r);
        fp2_mul_assign(s1, h);
        fp2_double_in_place(s1);
        fp2_sub_assign(u1, s1);
        fp2_assign(&mut self.y, u1);
    }

    /// `self = -self`
    #[inline(always)]
    fn neg_in_place(&mut self) {
        fp2_neg_in_place(&mut self.y);
    }

    /// `self = ψ(self)`, the untwist-Frobenius-twist endomorphism
    /// `(x, y) -> (x̄ (u+9)^((p-1)/3), ȳ (u+9)^((p-1)/2))`; in Jacobian coordinates the
    /// conjugation applies to `Z` as well
    fn psi_in_place(&mut self) {
        self.x.c1.neg_in_place();
        fp2_mul_assign(&mut self.x, &P_POWER_ENDOMORPHISM_COEFF_0);
        self.y.c1.neg_in_place();
        fp2_mul_assign(&mut self.y, &P_POWER_ENDOMORPHISM_COEFF_1);
        self.z.c1.neg_in_place();
    }

    /// `self = [x] p` by binary double-and-add over the 63 bits of `x`
    fn mul_by_x_in_place(&mut self, p: &G2Affine) {
        debug_assert_eq!(X.leading_zeros(), 1);
        for i in (0..63).rev() {
            self.double_in_place();
            if (X >> i) & 1 == 1 {
                self.add_assign_affine(p);
            }
        }
    }
}

/// Whether the point `p` of the twist (assumed to be on the curve) is in the subgroup `G2`
pub(crate) fn is_in_subgroup(p: &G2Affine) -> bool {
    // a = [x]P
    let mut a_slot = MaybeUninit::<G2Jac>::uninit();
    let a = G2Jac::init_from_affine(&mut a_slot, &G2Affine::identity());
    a.mul_by_x_in_place(p);
    // b = ψ([x]P), c = ψ²([x]P), d = ψ³([2x]P)
    g2jac_tmp!(b = &*a);
    b.psi_in_place();
    g2jac_tmp!(c = &*b);
    c.psi_in_place();
    g2jac_tmp!(d = &*c);
    d.psi_in_place();
    d.double_in_place();
    // [x + 1]P + ψ([x]P) + ψ²([x]P) - ψ³([2x]P) = O
    a.add_assign_affine(p);
    a.add_assign(b);
    a.add_assign(c);
    d.neg_in_place();
    a.add_assign(d);
    a.is_infinity()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::bn254::curves::g2::{is_in_subgroup_reference, Config};
    use ark_ec::{AffineRepr, CurveGroup};
    use ark_ff::UniformRand;
    use ark_std::test_rng;

    fn to_affine(p: &G2Jac) -> G2Affine {
        if p.is_infinity() {
            return G2Affine::identity();
        }
        let z_inv = p.z.inverse().unwrap();
        let z_inv2 = z_inv.square();
        G2Affine::new_unchecked(p.x * z_inv2, p.y * z_inv2 * z_inv)
    }

    /// A random point of the twist, in the subgroup with negligible probability
    pub(crate) fn random_curve_point(rng: &mut impl ark_std::rand::Rng) -> G2Affine {
        use ark_ec::models::short_weierstrass::SWCurveConfig;
        loop {
            let x = Fq2::rand(rng);
            let rhs = x.square() * x + Config::COEFF_B;
            if let Some(y) = rhs.sqrt() {
                let p = G2Affine::new_unchecked(x, y);
                assert!(p.is_on_curve());
                return p;
            }
        }
    }

    /// The definition: `[r]P = O`
    fn definitive(p: &G2Affine) -> bool {
        use ark_ff::PrimeField;
        p.mul_bigint(ark_bn254::Fr::MODULUS.0).is_zero()
    }

    #[test]
    fn group_law_matches_arkworks() {
        let mut rng = test_rng();
        for _ in 0..20 {
            let p = random_curve_point(&mut rng);
            let q = random_curve_point(&mut rng);
            let mut slot = MaybeUninit::uninit();
            let a = G2Jac::init_from_affine(&mut slot, &G2Affine::identity());
            a.mul_by_x_in_place(&p);
            assert_eq!(to_affine(a), p.mul_bigint([X]).into_affine());
            // general addition, doubling through the equal-x branch, and the negation branch
            g2jac_tmp!(b = &*a);
            let mut qslot = MaybeUninit::uninit();
            let qj = G2Jac::init_from_affine(&mut qslot, &q);
            qj.double_in_place();
            b.add_assign(qj);
            assert_eq!(
                to_affine(b),
                (p.mul_bigint([X]) + q.into_group().double()).into_affine()
            );
            g2jac_tmp!(twice = &*a);
            twice.add_assign(a);
            assert_eq!(to_affine(twice), (p.mul_bigint([X]).double()).into_affine());
            g2jac_tmp!(minus = &*a);
            minus.neg_in_place();
            minus.add_assign(a);
            assert!(minus.is_infinity());
            let mut pslot = MaybeUninit::uninit();
            let pj = G2Jac::init_from_affine(&mut pslot, &p);
            pj.add_assign_affine(&p);
            assert_eq!(to_affine(pj), p.into_group().double().into_affine());
            pj.add_assign_affine(&(-p));
            pj.add_assign_affine(&(-p));
            assert!(pj.is_infinity());
            pj.add_assign_affine(&p);
            assert_eq!(to_affine(pj), p);
        }
    }

    #[test]
    fn membership_matches_the_definition_and_the_reference() {
        let mut rng = test_rng();
        assert!(is_in_subgroup(&G2Affine::identity()));
        assert!(is_in_subgroup(&G2Affine::generator()));
        assert!(is_in_subgroup(&-G2Affine::generator()));
        let mut outside = 0;
        for _ in 0..20 {
            let in_subgroup = G2Affine::rand(&mut rng);
            assert!(is_in_subgroup(&in_subgroup));
            assert!(definitive(&in_subgroup));
            let p = random_curve_point(&mut rng);
            let expected = definitive(&p);
            assert_eq!(is_in_subgroup(&p), expected);
            assert_eq!(is_in_subgroup_reference(&p), expected);
            outside += !expected as usize;
            let cleared = p.mul_by_cofactor();
            assert!(definitive(&cleared));
            assert!(is_in_subgroup(&cleared));
        }
        assert!(outside > 0, "no random point of the twist outside G2");
    }
}
