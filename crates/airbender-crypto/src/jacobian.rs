//! Jacobian point arithmetic over a delegated base field, in place, for short Weierstrass
//! curves with `a = 0` (bn254 and bls12-381 G1).
//!
//! The arkworks `Projective` group law moves every intermediate value by value; here every
//! operation updates its operand and temporaries are initialized with delegated copies, so a
//! doubling or a mixed addition costs its field operations and little else.

use core::mem::MaybeUninit;

use ark_ec::short_weierstrass::{Affine, Projective, SWCurveConfig};
use ark_ff::{AdditiveGroup, Field, Zero};

use crate::extension_tower::{fp_tmp, CopyAssign};

/// `(X : Y : Z)` = `(X/Z², Y/Z³)`; the point at infinity iff `Z = 0`
pub(crate) struct Jacobian<C: SWCurveConfig> {
    x: C::BaseField,
    y: C::BaseField,
    z: C::BaseField,
}

impl<C: SWCurveConfig> Jacobian<C>
where
    C::BaseField: CopyAssign,
{
    /// Initializes `slot` with the point at infinity
    #[inline(always)]
    pub(crate) fn init_infinity(slot: &mut MaybeUninit<Self>) -> &mut Self {
        // SAFETY: all fields are written before the value is used
        unsafe {
            let p = slot.as_mut_ptr();
            C::BaseField::init_copy(core::ptr::addr_of_mut!((*p).x), &C::BaseField::ONE);
            C::BaseField::init_copy(core::ptr::addr_of_mut!((*p).y), &C::BaseField::ONE);
            C::BaseField::init_copy(core::ptr::addr_of_mut!((*p).z), &C::BaseField::ZERO);
            slot.assume_init_mut()
        }
    }

    #[inline(always)]
    pub(crate) fn is_infinity(&self) -> bool {
        self.z.is_zero()
    }

    #[inline(always)]
    fn set_infinity(&mut self) {
        self.z.copy_assign(&C::BaseField::ZERO);
    }

    /// The point as an arkworks projective point
    pub(crate) fn to_projective(&self) -> Projective<C> {
        if self.is_infinity() {
            Projective::zero()
        } else {
            Projective::new_unchecked(self.x, self.y, self.z)
        }
    }

    /// `self = 2 self`: `S = 4XY²`, `M = 3X²`, `X' = M² - 2S`, `Y' = M(S - X') - 8Y⁴`,
    /// `Z' = 2YZ` (3S + 2M for `a = 0`)
    pub(crate) fn double_in_place(&mut self) {
        debug_assert!(C::COEFF_A.is_zero());
        // xx = X², yy = Y², yyyy = Y⁴
        // a square of a fresh copy is a product with its source: one copy fewer than
        // `square_in_place`, which copies its operand itself
        fp_tmp!(xx = &self.x);
        *xx *= &self.x;
        fp_tmp!(yy = &self.y);
        *yy *= &self.y;
        fp_tmp!(yyyy = &*yy);
        *yyyy *= &*yy;
        // s = 4 X yy
        *yy *= &self.x;
        yy.double_in_place();
        yy.double_in_place();
        // m = 3 xx
        fp_tmp!(m = &*xx);
        m.double_in_place();
        *m += &*xx;
        // Z' = 2 Y Z
        self.z *= &self.y;
        self.z.double_in_place();
        // X' = m² - 2 s
        self.x.copy_assign(m);
        self.x *= &*m;
        self.x -= &*yy;
        self.x -= &*yy;
        // Y' = m (s - X') - 8 yyyy
        *yy -= &self.x;
        *m *= &*yy;
        yyyy.double_in_place();
        yyyy.double_in_place();
        yyyy.double_in_place();
        *m -= &*yyyy;
        self.y.copy_assign(m);
    }

    /// Initializes `slot` with a copy of `src`
    #[inline(always)]
    pub(crate) fn init_copy<'a>(slot: &'a mut MaybeUninit<Self>, src: &Self) -> &'a mut Self {
        // SAFETY: all fields are written before the value is used
        unsafe {
            let p = slot.as_mut_ptr();
            C::BaseField::init_copy(core::ptr::addr_of_mut!((*p).x), &src.x);
            C::BaseField::init_copy(core::ptr::addr_of_mut!((*p).y), &src.y);
            C::BaseField::init_copy(core::ptr::addr_of_mut!((*p).z), &src.z);
            slot.assume_init_mut()
        }
    }

    /// `self = -self`
    #[inline(always)]
    pub(crate) fn neg_in_place(&mut self) {
        self.y.neg_in_place();
    }

    /// `self = self + q` ("add-2007-bl": 11M + 5S), all inputs handled
    pub(crate) fn add_assign(&mut self, q: &Self) {
        if q.is_infinity() {
            return;
        }
        if self.is_infinity() {
            self.x.copy_assign(&q.x);
            self.y.copy_assign(&q.y);
            self.z.copy_assign(&q.z);
            return;
        }
        // z1z1 = Z1², z2z2 = Z2² (products with the sources: see `double_in_place`)
        fp_tmp!(z1z1 = &self.z);
        *z1z1 *= &self.z;
        fp_tmp!(z2z2 = &q.z);
        *z2z2 *= &q.z;
        // u1 = X1 z2z2, h = U2 - u1 = X2 z1z1 - u1
        fp_tmp!(u1 = &self.x);
        *u1 *= &*z2z2;
        fp_tmp!(h = &q.x);
        *h *= &*z1z1;
        *h -= &*u1;
        // s1 = Y1 Z2 z2z2, r = S2 - s1 = Y2 Z1 z1z1 - s1 (doubled below)
        fp_tmp!(s1 = &self.y);
        *s1 *= &q.z;
        *s1 *= &*z2z2;
        fp_tmp!(r = &q.y);
        *r *= &self.z;
        *r *= &*z1z1;
        *r -= &*s1;
        if h.is_zero() {
            // same x: the same point or its negation
            if r.is_zero() {
                self.double_in_place();
            } else {
                self.set_infinity();
            }
            return;
        }
        r.double_in_place();
        // Z3 = ((Z1 + Z2)² - z1z1 - z2z2) h
        self.z += &q.z;
        self.z.square_in_place();
        self.z -= &*z1z1;
        self.z -= &*z2z2;
        self.z *= &*h;
        // i = (2h)², j = h i, v = u1 i
        fp_tmp!(i = &*h);
        i.double_in_place();
        i.square_in_place();
        *h *= &*i;
        *u1 *= &*i;
        // X3 = r² - j - 2 v
        self.x.copy_assign(r);
        self.x *= &*r;
        self.x -= &*h;
        self.x -= &*u1;
        self.x -= &*u1;
        // Y3 = r (v - X3) - 2 s1 j
        *u1 -= &self.x;
        *u1 *= &*r;
        *s1 *= &*h;
        s1.double_in_place();
        *u1 -= &*s1;
        self.y.copy_assign(u1);
    }

    /// `self = self + q` for an affine `q` ("madd-2007-bl": 7M + 4S), all inputs handled
    pub(crate) fn add_assign_affine(&mut self, q: &Affine<C>) {
        if q.infinity {
            return;
        }
        if self.is_infinity() {
            self.x.copy_assign(&q.x);
            self.y.copy_assign(&q.y);
            self.z.copy_assign(&C::BaseField::ONE);
            return;
        }
        // z1z1 = Z1² (products with the sources: see `double_in_place`)
        fp_tmp!(z1z1 = &self.z);
        *z1z1 *= &self.z;
        // h = U2 - X1 = X2 z1z1 - X1
        fp_tmp!(h = &q.x);
        *h *= &*z1z1;
        *h -= &self.x;
        // r = S2 - Y1 = Y2 Z1 z1z1 - Y1 (doubled below)
        fp_tmp!(r = &q.y);
        *r *= &self.z;
        *r *= &*z1z1;
        *r -= &self.y;
        if h.is_zero() {
            // same x: the same point or its negation
            if r.is_zero() {
                self.double_in_place();
            } else {
                self.set_infinity();
            }
            return;
        }
        r.double_in_place();
        // hh = h²
        fp_tmp!(hh = &*h);
        *hh *= &*h;
        // Z3 = (Z1 + h)² - z1z1 - hh
        self.z += &*h;
        self.z.square_in_place();
        self.z -= &*z1z1;
        self.z -= &*hh;
        // i = 4 hh, j = h i, v = X1 i
        hh.double_in_place();
        hh.double_in_place();
        *h *= &*hh;
        self.x *= &*hh;
        // X3 = r² - j - 2 v
        fp_tmp!(x3 = &*r);
        *x3 *= &*r;
        *x3 -= &*h;
        *x3 -= &self.x;
        *x3 -= &self.x;
        // Y3 = r (v - X3) - 2 Y1 j
        self.x -= &*x3;
        self.x *= &*r;
        self.y *= &*h;
        self.y.double_in_place();
        self.x -= &self.y;
        self.y.copy_assign(&self.x);
        self.x.copy_assign(x3);
    }
}

/// `a + b` for affine points, on the in-place group law (arkworks' `Projective + Affine`
/// moves every field element by value, which costs a copy per operation)
pub(crate) fn add_affine<C: SWCurveConfig>(a: &Affine<C>, b: &Affine<C>) -> Projective<C>
where
    C::BaseField: CopyAssign,
{
    let mut slot = MaybeUninit::uninit();
    let sum = Jacobian::<C>::init_infinity(&mut slot);
    sum.add_assign_affine(a);
    sum.add_assign_affine(b);
    sum.to_projective()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::{AffineRepr, CurveGroup};
    use ark_ff::{PrimeField, UniformRand};
    use ark_std::test_rng;

    fn check<C: SWCurveConfig>()
    where
        C::BaseField: CopyAssign,
    {
        let mut rng = test_rng();
        let g = Affine::<C>::generator();
        for _ in 0..20 {
            let k = C::ScalarField::rand(&mut rng).into_bigint();
            let p = g.mul_bigint(k).into_affine();
            let q = g
                .mul_bigint(C::ScalarField::rand(&mut rng).into_bigint())
                .into_affine();
            let mut slot = MaybeUninit::uninit();
            let a = Jacobian::<C>::init_infinity(&mut slot);
            assert!(a.is_infinity());
            a.double_in_place();
            assert!(a.is_infinity());
            a.add_assign_affine(&p);
            assert_eq!(a.to_projective().into_affine(), p);
            a.double_in_place();
            assert_eq!(a.to_projective().into_affine(), (p + p).into_affine());
            a.add_assign_affine(&q);
            assert_eq!(a.to_projective().into_affine(), (p + p + q).into_affine());
            // the exceptional branches: adding the same point and its negation
            let mut slot = MaybeUninit::uninit();
            let b = Jacobian::<C>::init_infinity(&mut slot);
            b.add_assign_affine(&p);
            b.add_assign_affine(&p);
            assert_eq!(b.to_projective().into_affine(), (p + p).into_affine());
            b.add_assign_affine(&(-p));
            assert_eq!(b.to_projective().into_affine(), p);
            b.add_assign_affine(&(-p));
            assert!(b.is_infinity());
            b.add_assign_affine(&Affine::identity());
            assert!(b.is_infinity());
            // full additions, including the exceptional branches
            let mut slot = MaybeUninit::uninit();
            let c = Jacobian::<C>::init_infinity(&mut slot);
            c.add_assign_affine(&p);
            c.double_in_place();
            let mut slot = MaybeUninit::uninit();
            let d = Jacobian::<C>::init_infinity(&mut slot);
            d.add_assign_affine(&q);
            d.add_assign_affine(&q);
            d.add_assign_affine(&q);
            c.add_assign(d);
            assert_eq!(
                c.to_projective().into_affine(),
                (p + p + q + q + q).into_affine()
            );
            let mut slot = MaybeUninit::uninit();
            let e = Jacobian::<C>::init_copy(&mut slot, c);
            e.add_assign(c);
            assert_eq!(e.to_projective(), c.to_projective().double());
            e.neg_in_place();
            e.add_assign(c);
            assert!(!e.is_infinity());
            let mut slot = MaybeUninit::uninit();
            let f = Jacobian::<C>::init_copy(&mut slot, c);
            f.neg_in_place();
            f.add_assign(c);
            assert!(f.is_infinity());
            f.add_assign(c);
            assert_eq!(f.to_projective(), c.to_projective());
            c.add_assign(f);
            assert_eq!(f.to_projective().double(), c.to_projective());
        }
    }

    #[test]
    fn bn254_g1_matches_arkworks() {
        check::<crate::bn254::curves::g1::Config>();
    }

    #[test]
    fn bls12_381_g1_matches_arkworks() {
        check::<crate::bls12_381::curves::g1::Config>();
    }
}
