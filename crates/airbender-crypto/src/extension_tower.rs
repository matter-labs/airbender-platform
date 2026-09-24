//! In-place arithmetic of the `Fp2` / `Fp6` / `Fp12` extension towers over a delegated base
//! field (bn254 and bls12-381 share the shape `Fp2 = Fp[u]/(u² + 1)`, `Fp6 = Fp2[v]/(v³ - ξ)`,
//! `Fp12 = Fp6[w]/(w² - v)`).
//!
//! The formulas are those of the generic arkworks extension fields (Karatsuba products, the
//! sparse `mul_by_01` / `mul_by_034` / `mul_by_014` of the Miller loops, the cyclotomic
//! squaring), but every operand is updated in place and temporaries are initialized component
//! by component: a `Fp2`, `Fp6` or `Fp12` value is never returned or moved as a whole, which on
//! the proving target turned into a `memcpy` call per operation. The multiplication by `ξ`
//! comes from the curve's `Fp6Config`.

use core::mem::MaybeUninit;

use ark_ff::{AdditiveGroup, Fp12, Fp12Config, Fp2, Fp2Config, Fp6, Fp6Config};

/// Copies of a base-field element. On the proving target a 256-bit copy is one delegated
/// `memcpy` (4 instructions) instead of a 16-word loop, and a 512-bit one is two instead of a
/// `memcpy` call.
pub(crate) trait CopyAssign: Copy {
    fn copy_assign(&mut self, src: &Self);
    /// Initializes the memory at `dst` with a copy of `src`
    /// # Safety
    /// `dst` must be valid for writes of `Self` and aligned for it
    unsafe fn init_copy(dst: *mut Self, src: &Self);
}

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    feature = "testing",
    test
))]
impl<P: crate::ark_ff_delegation::fp::FpConfig<N>, const N: usize> CopyAssign
    for crate::ark_ff_delegation::Fp<P, N>
{
    #[inline(always)]
    fn copy_assign(&mut self, src: &Self) {
        use crate::bigint_delegation::{u256, u512};
        // SAFETY: the casts only restate the limb count of a `BigInt<N>`
        if N == 4 {
            unsafe {
                u256::copy_assign(
                    &mut *(&mut self.0 as *mut _ as *mut crate::BigInt<4>),
                    &*(&src.0 as *const _ as *const crate::BigInt<4>),
                )
            }
        } else if N == 8 {
            unsafe {
                u512::copy_assign(
                    &mut *(&mut self.0 as *mut _ as *mut crate::BigInt<8>),
                    &*(&src.0 as *const _ as *const crate::BigInt<8>),
                )
            }
        } else {
            *self = *src;
        }
    }

    #[inline(always)]
    unsafe fn init_copy(dst: *mut Self, src: &Self) {
        use crate::bigint_delegation::{u256, u512};
        let limbs = core::ptr::addr_of_mut!((*dst).0);
        if N == 4 {
            u256::init_copy(
                limbs.cast(),
                &*(&src.0 as *const _ as *const crate::BigInt<4>),
            )
        } else if N == 8 {
            u512::init_copy(
                limbs.cast(),
                &*(&src.0 as *const _ as *const crate::BigInt<8>),
            )
        } else {
            dst.write(*src);
        }
    }
}

// the arkworks field types are the base fields of the host builds
impl<P: ark_ff::FpConfig<N>, const N: usize> CopyAssign for ark_ff::Fp<P, N> {
    #[inline(always)]
    fn copy_assign(&mut self, src: &Self) {
        *self = *src;
    }

    #[inline(always)]
    unsafe fn init_copy(dst: *mut Self, src: &Self) {
        dst.write(*src);
    }
}

/// Quadratic extensions with `u² = -1`: the `Fp2` products below hard-code that non-residue,
/// so it is opted into per configuration
pub(crate) trait NonresidueMinusOne: Fp2Config<Fp: CopyAssign> {
    /// `a *= b` with the three products of the Karatsuba multiplication reduced only once
    /// combined, where the base field offers it (`true`); otherwise `a` is left untouched
    #[inline(always)]
    fn mul_assign_lazy(_a: &mut Fp2<Self>, _b: &Fp2<Self>) -> bool {
        false
    }
}

impl NonresidueMinusOne for crate::bn254::fields::Fq2Config {}
impl NonresidueMinusOne for crate::bls12_381::fields::Fq2Config {
    #[cfg(any(
        all(target_arch = "riscv32", feature = "bigint_ops"),
        test,
        feature = "proving"
    ))]
    #[inline(always)]
    fn mul_assign_lazy(a: &mut Fp2<Self>, b: &Fp2<Self>) -> bool {
        crate::bls12_381::fields::fq::fq2_mul_assign_lazy(&mut a.c0, &mut a.c1, &b.c0, &b.c1);
        true
    }
}

/// Multiplication by the cubic non-residue `ξ` of `Fp6 = Fp2[v]/(v³ - ξ)`, in place and without
/// moving the operand (the arkworks `mul_fp2_by_nonresidue_in_place` hooks copy it)
pub(crate) trait MulByXi: Fp6Config {
    fn mul_by_xi_in_place(a: &mut Fp2<Self::Fp2Config>);
}

impl MulByXi for crate::bn254::fields::Fq6Config {
    /// `ξ = 9 + u`: `(9 c0 - c1) + (9 c1 + c0) u`
    #[inline(always)]
    fn mul_by_xi_in_place(a: &mut Fp2<Self::Fp2Config>) {
        fp_tmp!(c0 = &a.c0);
        fp_tmp!(c1 = &a.c1);
        a.c0.double_in_place();
        a.c0.double_in_place();
        a.c0.double_in_place();
        a.c0 += &*c0;
        a.c0 -= &*c1;
        a.c1.double_in_place();
        a.c1.double_in_place();
        a.c1.double_in_place();
        a.c1 += &*c1;
        a.c1 += &*c0;
    }
}

impl MulByXi for crate::bls12_381::fields::Fq6Config {
    /// `ξ = 1 + u`: `(c0 - c1) + (c0 + c1) u`
    #[inline(always)]
    fn mul_by_xi_in_place(a: &mut Fp2<Self::Fp2Config>) {
        fp_tmp!(c0 = &a.c0);
        a.c0 -= &a.c1;
        a.c1 += &*c0;
    }
}

type Fp2Of<P6> = Fp2<<P6 as Fp6Config>::Fp2Config>;
type Fp6Of<P12> = Fp6<<P12 as Fp12Config>::Fp6Config>;
type Fp2Of12<P12> = Fp2Of<<P12 as Fp12Config>::Fp6Config>;

// --- temporaries ---------------------------------------------------------------------------

/// Initializes `slot` with a copy of `src`
#[inline(always)]
pub(crate) fn fp_init<'a, F: CopyAssign>(slot: &'a mut MaybeUninit<F>, src: &F) -> &'a mut F {
    // SAFETY: the slot is written before the value is used
    unsafe {
        F::init_copy(slot.as_mut_ptr(), src);
        slot.assume_init_mut()
    }
}

/// A named base-field temporary initialized from a reference
macro_rules! fp_tmp {
    ($name:ident = $src:expr) => {
        let mut slot = MaybeUninit::uninit();
        let $name = $crate::extension_tower::fp_init(&mut slot, $src);
    };
}
pub(crate) use fp_tmp;

/// Initializes `slot` with a copy of `src`, component by component
#[inline(always)]
pub(crate) fn fp2_init<'a, P: NonresidueMinusOne>(
    slot: &'a mut MaybeUninit<Fp2<P>>,
    src: &Fp2<P>,
) -> &'a mut Fp2<P> {
    // SAFETY: both fields are written before the value is used
    unsafe {
        let p = slot.as_mut_ptr();
        P::Fp::init_copy(core::ptr::addr_of_mut!((*p).c0), &src.c0);
        P::Fp::init_copy(core::ptr::addr_of_mut!((*p).c1), &src.c1);
        slot.assume_init_mut()
    }
}

#[inline(always)]
pub(crate) fn fp6_init<'a, P: Fp6Config>(
    slot: &'a mut MaybeUninit<Fp6<P>>,
    src: &Fp6<P>,
) -> &'a mut Fp6<P>
where
    P::Fp2Config: NonresidueMinusOne,
{
    type F<P> = <<P as Fp6Config>::Fp2Config as Fp2Config>::Fp;
    // SAFETY: all fields are written before the value is used
    unsafe {
        let p = slot.as_mut_ptr();
        F::<P>::init_copy(core::ptr::addr_of_mut!((*p).c0.c0), &src.c0.c0);
        F::<P>::init_copy(core::ptr::addr_of_mut!((*p).c0.c1), &src.c0.c1);
        F::<P>::init_copy(core::ptr::addr_of_mut!((*p).c1.c0), &src.c1.c0);
        F::<P>::init_copy(core::ptr::addr_of_mut!((*p).c1.c1), &src.c1.c1);
        F::<P>::init_copy(core::ptr::addr_of_mut!((*p).c2.c0), &src.c2.c0);
        F::<P>::init_copy(core::ptr::addr_of_mut!((*p).c2.c1), &src.c2.c1);
        slot.assume_init_mut()
    }
}

#[inline(always)]
pub(crate) fn fp12_init<'a, P: Fp12Config>(
    slot: &'a mut MaybeUninit<Fp12<P>>,
    src: &Fp12<P>,
) -> &'a mut Fp12<P>
where
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    // SAFETY: both halves are written before the value is used
    unsafe {
        let p = slot.as_mut_ptr();
        let c0 = &mut *(core::ptr::addr_of_mut!((*p).c0) as *mut MaybeUninit<Fp6Of<P>>);
        fp6_init(c0, &src.c0);
        let c1 = &mut *(core::ptr::addr_of_mut!((*p).c1) as *mut MaybeUninit<Fp6Of<P>>);
        fp6_init(c1, &src.c1);
        slot.assume_init_mut()
    }
}

/// A named `Fp2` temporary initialized from a reference
macro_rules! fp2_tmp {
    ($name:ident = $src:expr) => {
        let mut slot = MaybeUninit::uninit();
        let $name = $crate::extension_tower::fp2_init(&mut slot, $src);
    };
}
pub(crate) use fp2_tmp;

/// A named `Fp6` temporary initialized from a reference
macro_rules! fp6_tmp {
    ($name:ident = $src:expr) => {
        let mut slot = MaybeUninit::uninit();
        let $name = $crate::extension_tower::fp6_init(&mut slot, $src);
    };
}
#[allow(unused_imports)]
pub(crate) use fp6_tmp;

/// A named `Fp12` temporary initialized from a reference
macro_rules! fp12_tmp {
    ($name:ident = $src:expr) => {
        let mut slot = MaybeUninit::uninit();
        let $name = $crate::extension_tower::fp12_init(&mut slot, $src);
    };
}
pub(crate) use fp12_tmp;

// --- Fp2 = Fp[u] / (u² + 1) ------------------------------------------------------------------

#[inline(always)]
pub(crate) fn fp2_assign<P: NonresidueMinusOne>(dst: &mut Fp2<P>, src: &Fp2<P>) {
    dst.c0.copy_assign(&src.c0);
    dst.c1.copy_assign(&src.c1);
}

#[inline(always)]
pub(crate) fn fp2_add_assign<P: Fp2Config>(a: &mut Fp2<P>, b: &Fp2<P>) {
    a.c0 += &b.c0;
    a.c1 += &b.c1;
}

#[inline(always)]
pub(crate) fn fp2_sub_assign<P: Fp2Config>(a: &mut Fp2<P>, b: &Fp2<P>) {
    a.c0 -= &b.c0;
    a.c1 -= &b.c1;
}

#[inline(always)]
pub(crate) fn fp2_double_in_place<P: Fp2Config>(a: &mut Fp2<P>) {
    a.c0.double_in_place();
    a.c1.double_in_place();
}

#[inline(always)]
pub(crate) fn fp2_neg_in_place<P: Fp2Config>(a: &mut Fp2<P>) {
    a.c0.neg_in_place();
    a.c1.neg_in_place();
}

#[inline(always)]
pub(crate) fn fp2_mul_by_fp<P: Fp2Config>(a: &mut Fp2<P>, f: &P::Fp) {
    a.c0 *= f;
    a.c1 *= f;
}

/// `a *= b`, three base multiplications
#[inline(always)]
pub(crate) fn fp2_mul_assign<P: NonresidueMinusOne>(a: &mut Fp2<P>, b: &Fp2<P>) {
    if P::mul_assign_lazy(a, b) {
        return;
    }
    // two copies: a1 b1 needs the original a1 after a0 + a1 is formed in place, and the sum
    // b0 + b1 must not touch `b`; a0 b0 is computed in place in a0 once a0 + a1 is formed
    fp_tmp!(t1 = &a.c1);
    *t1 *= &b.c1;
    fp_tmp!(s = &b.c0);
    *s += &b.c1;
    // c1 = (a0 + a1)(b0 + b1) - a0 b0 - a1 b1
    a.c1 += &a.c0;
    a.c1 *= &*s;
    a.c0 *= &b.c0;
    a.c1 -= &a.c0;
    a.c1 -= &*t1;
    // c0 = a0 b0 - a1 b1
    a.c0 -= &*t1;
}

/// `a = a²`: `(c0 + c1)(c0 - c1) + 2 c0 c1 u`
#[inline(always)]
pub(crate) fn fp2_square_in_place<P: NonresidueMinusOne>(a: &mut Fp2<P>) {
    fp_tmp!(c0 = &a.c0);
    fp_tmp!(v0 = &a.c0);
    *v0 -= &a.c1;
    a.c0 += &a.c1;
    a.c0 *= &*v0;
    a.c1.double_in_place();
    a.c1 *= &*c0;
}

/// `a *= ξ`, the non-residue of the cubic extension
#[inline(always)]
pub(crate) fn fp2_mul_by_xi<P: MulByXi>(a: &mut Fp2Of<P>) {
    P::mul_by_xi_in_place(a);
}

// --- Fp6 = Fp2[v] / (v³ - ξ) -----------------------------------------------------------------

#[inline(always)]
pub(crate) fn fp6_assign<P: Fp6Config>(dst: &mut Fp6<P>, src: &Fp6<P>)
where
    P::Fp2Config: NonresidueMinusOne,
{
    fp2_assign(&mut dst.c0, &src.c0);
    fp2_assign(&mut dst.c1, &src.c1);
    fp2_assign(&mut dst.c2, &src.c2);
}

#[inline(always)]
pub(crate) fn fp6_add_assign<P: Fp6Config>(a: &mut Fp6<P>, b: &Fp6<P>) {
    fp2_add_assign(&mut a.c0, &b.c0);
    fp2_add_assign(&mut a.c1, &b.c1);
    fp2_add_assign(&mut a.c2, &b.c2);
}

#[inline(always)]
pub(crate) fn fp6_sub_assign<P: Fp6Config>(a: &mut Fp6<P>, b: &Fp6<P>) {
    fp2_sub_assign(&mut a.c0, &b.c0);
    fp2_sub_assign(&mut a.c1, &b.c1);
    fp2_sub_assign(&mut a.c2, &b.c2);
}

#[inline(always)]
pub(crate) fn fp6_double_in_place<P: Fp6Config>(a: &mut Fp6<P>) {
    fp2_double_in_place(&mut a.c0);
    fp2_double_in_place(&mut a.c1);
    fp2_double_in_place(&mut a.c2);
}

#[inline(always)]
pub(crate) fn fp6_neg_in_place<P: Fp6Config>(a: &mut Fp6<P>) {
    fp2_neg_in_place(&mut a.c0);
    fp2_neg_in_place(&mut a.c1);
    fp2_neg_in_place(&mut a.c2);
}

/// `a *= v`: `(c0, c1, c2) -> (ξ c2, c0, c1)`
#[inline(always)]
pub(crate) fn fp6_mul_by_nonresidue_in_place<P: MulByXi>(a: &mut Fp6<P>)
where
    P::Fp2Config: NonresidueMinusOne,
{
    fp2_mul_by_xi::<P>(&mut a.c2);
    fp2_tmp!(t = &a.c2);
    fp2_assign(&mut a.c2, &a.c1);
    fp2_assign(&mut a.c1, &a.c0);
    fp2_assign(&mut a.c0, t);
}

/// `a *= b` (Karatsuba)
pub(crate) fn fp6_mul_assign<P: MulByXi>(a: &mut Fp6<P>, b: &Fp6<P>)
where
    P::Fp2Config: NonresidueMinusOne,
{
    fp2_tmp!(ad = &a.c0);
    fp2_mul_assign(ad, &b.c0);
    fp2_tmp!(be = &a.c1);
    fp2_mul_assign(be, &b.c1);
    fp2_tmp!(cf = &a.c2);
    fp2_mul_assign(cf, &b.c2);
    fp2_tmp!(s = &b.c1);
    // x = (a1 + a2)(b1 + b2) - be - cf
    fp2_tmp!(x = &a.c1);
    fp2_add_assign(x, &a.c2);
    fp2_add_assign(s, &b.c2);
    fp2_mul_assign(x, s);
    fp2_sub_assign(x, be);
    fp2_sub_assign(x, cf);
    // y = (a0 + a1)(b0 + b1) - ad - be
    fp2_tmp!(y = &a.c0);
    fp2_add_assign(y, &a.c1);
    fp2_assign(s, &b.c0);
    fp2_add_assign(s, &b.c1);
    fp2_mul_assign(y, s);
    fp2_sub_assign(y, ad);
    fp2_sub_assign(y, be);
    // c2 = z = (a0 + a2)(b0 + b2) - ad + be - cf
    fp2_add_assign(&mut a.c2, &a.c0);
    fp2_assign(s, &b.c0);
    fp2_add_assign(s, &b.c2);
    fp2_mul_assign(&mut a.c2, s);
    fp2_sub_assign(&mut a.c2, ad);
    fp2_add_assign(&mut a.c2, be);
    fp2_sub_assign(&mut a.c2, cf);
    // c0 = ad + ξ x
    fp2_mul_by_xi::<P>(x);
    fp2_assign(&mut a.c0, ad);
    fp2_add_assign(&mut a.c0, x);
    // c1 = y + ξ cf
    fp2_mul_by_xi::<P>(cf);
    fp2_assign(&mut a.c1, y);
    fp2_add_assign(&mut a.c1, cf);
}

/// `a = a²`
#[cfg(test)]
pub(crate) fn fp6_square_in_place<P: MulByXi>(a: &mut Fp6<P>)
where
    P::Fp2Config: NonresidueMinusOne,
{
    fp2_tmp!(s0 = &a.c0);
    fp2_square_in_place(s0);
    // s1 = 2 a0 a1
    fp2_tmp!(s1 = &a.c0);
    fp2_mul_assign(s1, &a.c1);
    fp2_double_in_place(s1);
    // s2 = (a0 - a1 + a2)²
    fp2_tmp!(s2 = &a.c0);
    fp2_sub_assign(s2, &a.c1);
    fp2_add_assign(s2, &a.c2);
    fp2_square_in_place(s2);
    // s3 = 2 a1 a2
    fp2_tmp!(s3 = &a.c1);
    fp2_mul_assign(s3, &a.c2);
    fp2_double_in_place(s3);
    // s4 = a2²
    fp2_square_in_place(&mut a.c2);
    // c2 = s1 + s2 + s3 - s0 - s4
    fp2_tmp!(c2 = s1);
    fp2_add_assign(c2, s2);
    fp2_add_assign(c2, s3);
    fp2_sub_assign(c2, s0);
    fp2_sub_assign(c2, &a.c2);
    // c1 = ξ s4 + s1
    fp2_mul_by_xi::<P>(&mut a.c2);
    fp2_add_assign(&mut a.c2, s1);
    fp2_assign(&mut a.c1, &a.c2);
    // c0 = ξ s3 + s0
    fp2_mul_by_xi::<P>(s3);
    fp2_add_assign(s3, s0);
    fp2_assign(&mut a.c0, s3);
    fp2_assign(&mut a.c2, c2);
}

/// `a *= c0 + c1 v`
pub(crate) fn fp6_mul_by_01<P: MulByXi>(a: &mut Fp6<P>, c0: &Fp2Of<P>, c1: &Fp2Of<P>)
where
    P::Fp2Config: NonresidueMinusOne,
{
    fp2_tmp!(a_a = &a.c0);
    fp2_mul_assign(a_a, c0);
    fp2_tmp!(b_b = &a.c1);
    fp2_mul_assign(b_b, c1);
    // t1 = ξ (c1 (a1 + a2) - b_b) + a_a
    fp2_tmp!(t1 = &a.c1);
    fp2_add_assign(t1, &a.c2);
    fp2_mul_assign(t1, c1);
    fp2_sub_assign(t1, b_b);
    fp2_mul_by_xi::<P>(t1);
    fp2_add_assign(t1, a_a);
    // t3 = c0 (a0 + a2) - a_a + b_b
    fp2_add_assign(&mut a.c2, &a.c0);
    fp2_mul_assign(&mut a.c2, c0);
    fp2_sub_assign(&mut a.c2, a_a);
    fp2_add_assign(&mut a.c2, b_b);
    // t2 = (c0 + c1)(a0 + a1) - a_a - b_b
    fp2_tmp!(s = c0);
    fp2_add_assign(s, c1);
    fp2_add_assign(&mut a.c1, &a.c0);
    fp2_mul_assign(&mut a.c1, s);
    fp2_sub_assign(&mut a.c1, a_a);
    fp2_sub_assign(&mut a.c1, b_b);
    fp2_assign(&mut a.c0, t1);
}

/// `a *= c1 v`: `(a0, a1, a2) -> (ξ a2 c1, a0 c1, a1 c1)`
pub(crate) fn fp6_mul_by_1<P: MulByXi>(a: &mut Fp6<P>, c1: &Fp2Of<P>)
where
    P::Fp2Config: NonresidueMinusOne,
{
    fp2_tmp!(t = &a.c2);
    fp2_mul_assign(t, c1);
    fp2_mul_by_xi::<P>(t);
    fp2_mul_assign(&mut a.c1, c1);
    fp2_mul_assign(&mut a.c0, c1);
    fp2_assign(&mut a.c2, &a.c1);
    fp2_assign(&mut a.c1, &a.c0);
    fp2_assign(&mut a.c0, t);
}

// --- Fp12 = Fp6[w] / (w² - v) ----------------------------------------------------------------

#[inline(always)]
pub(crate) fn fp12_assign<P: Fp12Config>(dst: &mut Fp12<P>, src: &Fp12<P>)
where
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    fp6_assign(&mut dst.c0, &src.c0);
    fp6_assign(&mut dst.c1, &src.c1);
}

/// `a *= b` (Karatsuba)
pub(crate) fn fp12_mul_assign<P: Fp12Config>(a: &mut Fp12<P>, b: &Fp12<P>)
where
    P::Fp6Config: MulByXi,
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    fp6_tmp!(v0 = &a.c0);
    fp6_mul_assign(v0, &b.c0);
    fp6_tmp!(v1 = &a.c1);
    fp6_mul_assign(v1, &b.c1);
    // c1 = (a0 + a1)(b0 + b1) - v0 - v1
    fp6_tmp!(s = &b.c0);
    fp6_add_assign(s, &b.c1);
    fp6_add_assign(&mut a.c1, &a.c0);
    fp6_mul_assign(&mut a.c1, s);
    fp6_sub_assign(&mut a.c1, v0);
    fp6_sub_assign(&mut a.c1, v1);
    // c0 = v0 + v v1
    fp6_mul_by_nonresidue_in_place(v1);
    fp6_assign(&mut a.c0, v0);
    fp6_add_assign(&mut a.c0, v1);
}

/// `a = a²`
pub(crate) fn fp12_square_in_place<P: Fp12Config>(a: &mut Fp12<P>)
where
    P::Fp6Config: MulByXi,
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    // v0 = (c0 - c1)(c0 - v c1)
    fp6_tmp!(v0 = &a.c0);
    fp6_sub_assign(v0, &a.c1);
    fp6_tmp!(v3 = &a.c1);
    fp6_mul_by_nonresidue_in_place(v3);
    fp6_neg_in_place(v3);
    fp6_add_assign(v3, &a.c0);
    fp6_mul_assign(v0, v3);
    // v2 = c0 c1
    fp6_mul_assign(&mut a.c1, &a.c0);
    // c0 = v0 + v2 + v v2
    fp6_assign(&mut a.c0, &a.c1);
    fp6_mul_by_nonresidue_in_place(&mut a.c0);
    fp6_add_assign(&mut a.c0, &a.c1);
    fp6_add_assign(&mut a.c0, v0);
    // c1 = 2 v2
    fp6_double_in_place(&mut a.c1);
}

/// `a *= c0 + (c3 + c4 v) w`, the sparse line multiplication of a D-type twist
pub(crate) fn fp12_mul_by_034<P: Fp12Config>(
    a: &mut Fp12<P>,
    c0: &Fp2Of12<P>,
    c3: &Fp2Of12<P>,
    c4: &Fp2Of12<P>,
) where
    P::Fp6Config: MulByXi,
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    // p = a0 c0 (component-wise)
    fp6_tmp!(p = &a.c0);
    fp2_mul_assign(&mut p.c0, c0);
    fp2_mul_assign(&mut p.c1, c0);
    fp2_mul_assign(&mut p.c2, c0);
    // b = a1 (c3 + c4 v)
    fp6_tmp!(b = &a.c1);
    fp6_mul_by_01(b, c3, c4);
    // e = (a0 + a1)(c0 + c3 + c4 v)
    fp2_tmp!(c0c3 = c0);
    fp2_add_assign(c0c3, c3);
    fp6_add_assign(&mut a.c1, &a.c0);
    fp6_mul_by_01(&mut a.c1, c0c3, c4);
    // c1 = e - p - b
    fp6_sub_assign(&mut a.c1, p);
    fp6_sub_assign(&mut a.c1, b);
    // c0 = v b + p
    fp6_mul_by_nonresidue_in_place(b);
    fp6_add_assign(b, p);
    fp6_assign(&mut a.c0, b);
}

/// `a *= (c0 + c1 v) + c4 v w`, the sparse line multiplication of an M-type twist
pub(crate) fn fp12_mul_by_014<P: Fp12Config>(
    a: &mut Fp12<P>,
    c0: &Fp2Of12<P>,
    c1: &Fp2Of12<P>,
    c4: &Fp2Of12<P>,
) where
    P::Fp6Config: MulByXi,
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    // p = a0 (c0 + c1 v)
    fp6_tmp!(p = &a.c0);
    fp6_mul_by_01(p, c0, c1);
    // b = a1 c4 v
    fp6_tmp!(b = &a.c1);
    fp6_mul_by_1(b, c4);
    // e = (a0 + a1)(c0 + (c1 + c4) v)
    fp2_tmp!(c1c4 = c1);
    fp2_add_assign(c1c4, c4);
    fp6_add_assign(&mut a.c1, &a.c0);
    fp6_mul_by_01(&mut a.c1, c0, c1c4);
    // c1 = e - p - b
    fp6_sub_assign(&mut a.c1, p);
    fp6_sub_assign(&mut a.c1, b);
    // c0 = v b + p
    fp6_mul_by_nonresidue_in_place(b);
    fp6_add_assign(b, p);
    fp6_assign(&mut a.c0, b);
}

/// Conjugation, the inverse in the cyclotomic subgroup
#[inline(always)]
pub(crate) fn fp12_cyclotomic_inverse_in_place<P: Fp12Config>(a: &mut Fp12<P>) {
    fp6_neg_in_place(&mut a.c1);
}

/// Squaring in the cyclotomic subgroup (Granger-Scott)
pub(crate) fn fp12_cyclotomic_square_in_place<P: Fp12Config>(a: &mut Fp12<P>)
where
    P::Fp6Config: MulByXi,
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    // (r0, r1, r2, r3, r4, r5) = (c0.c0, c1.c1, c1.c0, c0.c2, c0.c1, c1.c2)
    // t_i for the pairs (r0, r1), (r2, r3), (r4, r5): t = (a + b)(a + ξ b) - ab - ξ ab, 2ab
    macro_rules! pair {
        ($ra:expr, $rb:expr, $t0:ident, $t1:ident) => {
            fp2_tmp!(ab = $ra);
            fp2_mul_assign(ab, $rb);
            fp2_tmp!($t0 = $ra);
            fp2_add_assign($t0, $rb);
            fp2_tmp!(nrb = $rb);
            fp2_mul_by_xi::<P::Fp6Config>(nrb);
            fp2_add_assign(nrb, $ra);
            fp2_mul_assign($t0, nrb);
            fp2_sub_assign($t0, ab);
            fp2_tmp!(nrab = ab);
            fp2_mul_by_xi::<P::Fp6Config>(nrab);
            fp2_sub_assign($t0, nrab);
            let $t1 = ab;
            fp2_double_in_place($t1);
        };
    }
    pair!(&a.c0.c0, &a.c1.c1, t0, t1);
    pair!(&a.c1.c0, &a.c0.c2, t2, t3);
    pair!(&a.c0.c1, &a.c1.c2, t4, t5);
    // z0 = 3 t0 - 2 z0
    let z0 = &mut a.c0.c0;
    fp2_neg_in_place(z0);
    fp2_add_assign(z0, t0);
    fp2_double_in_place(z0);
    fp2_add_assign(z0, t0);
    // z1 = 3 t1 + 2 z1
    let z1 = &mut a.c1.c1;
    fp2_add_assign(z1, t1);
    fp2_double_in_place(z1);
    fp2_add_assign(z1, t1);
    // z2 = 3 ξ t5 + 2 z2
    fp2_mul_by_xi::<P::Fp6Config>(t5);
    let z2 = &mut a.c1.c0;
    fp2_add_assign(z2, t5);
    fp2_double_in_place(z2);
    fp2_add_assign(z2, t5);
    // z3 = 3 t4 - 2 z3
    let z3 = &mut a.c0.c2;
    fp2_neg_in_place(z3);
    fp2_add_assign(z3, t4);
    fp2_double_in_place(z3);
    fp2_add_assign(z3, t4);
    // z4 = 3 t2 - 2 z4
    let z4 = &mut a.c0.c1;
    fp2_neg_in_place(z4);
    fp2_add_assign(z4, t2);
    fp2_double_in_place(z4);
    fp2_add_assign(z4, t2);
    // z5 = 3 t3 + 2 z5
    let z5 = &mut a.c1.c2;
    fp2_add_assign(z5, t3);
    fp2_double_in_place(z5);
    fp2_add_assign(z5, t3);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ark_ff::{CyclotomicMultSubgroup, Field, PrimeField};
    use ark_std::rand::Rng;
    use ark_std::test_rng;

    /// Uniform elements without rejection sampling on the limbs (the delegated bls12-381 base
    /// field has 512-bit limbs for a 381-bit modulus, so `UniformRand` practically never
    /// terminates for it)
    pub(crate) fn rand_fp<F: PrimeField>(rng: &mut impl Rng) -> F {
        // Horner over six random 64-bit digits: uniform on [0, 2^384) reduced mod p
        let base = F::from(1u64 << 63).double();
        let mut x = F::ZERO;
        for _ in 0..6 {
            x *= base;
            x += F::from(rng.next_u64());
        }
        x
    }

    pub(crate) fn rand_fp2<P: Fp2Config>(rng: &mut impl Rng) -> Fp2<P> {
        Fp2::new(rand_fp(rng), rand_fp(rng))
    }

    pub(crate) fn rand_fp6<P: Fp6Config>(rng: &mut impl Rng) -> Fp6<P> {
        Fp6::new(rand_fp2(rng), rand_fp2(rng), rand_fp2(rng))
    }

    pub(crate) fn rand_fp12<P: Fp12Config>(rng: &mut impl Rng) -> Fp12<P> {
        Fp12::new(rand_fp6(rng), rand_fp6(rng))
    }

    /// Every in-place operation against the arkworks by-value one, for one tower
    fn check_tower<P: Fp12Config>()
    where
        P::Fp6Config: MulByXi,
        <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
    {
        type F2<P> = Fp2Of12<P>;
        type F6<P> = Fp6Of<P>;
        let minus_one = -<<<P::Fp6Config as Fp6Config>::Fp2Config as Fp2Config>::Fp>::ONE;
        assert_eq!(
            <<P::Fp6Config as Fp6Config>::Fp2Config as Fp2Config>::NONRESIDUE,
            minus_one
        );
        let mut rng = test_rng();
        for _ in 0..100 {
            let (a2, b2): (F2<P>, F2<P>) = (rand_fp2(&mut rng), rand_fp2(&mut rng));
            let f: <F2<P> as Field>::BasePrimeField = rand_fp(&mut rng);
            let mut x = a2;
            fp2_mul_assign(&mut x, &b2);
            assert_eq!(x, a2 * b2);
            let mut x = a2;
            fp2_square_in_place(&mut x);
            assert_eq!(x, a2.square());
            let mut x = a2;
            fp2_mul_by_xi::<P::Fp6Config>(&mut x);
            assert_eq!(x, a2 * <P::Fp6Config as Fp6Config>::NONRESIDUE);
            let mut x = a2;
            fp2_mul_by_fp(&mut x, &f);
            let mut y = a2;
            y.mul_assign_by_fp(&f);
            assert_eq!(x, y);

            let (a6, b6): (F6<P>, F6<P>) = (rand_fp6(&mut rng), rand_fp6(&mut rng));
            let mut x = a6;
            fp6_mul_assign(&mut x, &b6);
            assert_eq!(x, a6 * b6);
            let mut x = a6;
            fp6_square_in_place(&mut x);
            assert_eq!(x, a6.square());
            let mut x = a6;
            fp6_mul_by_01(&mut x, &b6.c0, &b6.c1);
            let mut y = a6;
            y.mul_by_01(&b6.c0, &b6.c1);
            assert_eq!(x, y);
            let mut x = a6;
            fp6_mul_by_1(&mut x, &b6.c1);
            let mut y = a6;
            y.mul_by_1(&b6.c1);
            assert_eq!(x, y);
            let mut x = a6;
            fp6_mul_by_nonresidue_in_place(&mut x);
            assert_eq!(x, a6 * P::NONRESIDUE);

            let (a12, b12): (Fp12<P>, Fp12<P>) = (rand_fp12(&mut rng), rand_fp12(&mut rng));
            let mut x = a12;
            fp12_mul_assign(&mut x, &b12);
            assert_eq!(x, a12 * b12);
            let mut x = a12;
            fp12_square_in_place(&mut x);
            assert_eq!(x, a12.square());
            let mut x = a12;
            fp12_mul_by_034(&mut x, &b2, &a2, &b6.c2);
            let mut y = a12;
            y.mul_by_034(&b2, &a2, &b6.c2);
            assert_eq!(x, y);
            let mut x = a12;
            fp12_mul_by_014(&mut x, &b2, &a2, &b6.c2);
            let mut y = a12;
            y.mul_by_014(&b2, &a2, &b6.c2);
            assert_eq!(x, y);
            let mut x = a12;
            fp12_cyclotomic_inverse_in_place(&mut x);
            let mut y = a12;
            y.cyclotomic_inverse_in_place();
            assert_eq!(x, y);
            // cyclotomic squaring is only equal to squaring in the cyclotomic subgroup:
            // a^((p^6 - 1)(p^2 + 1)) is in it
            let mut c = a12;
            c.cyclotomic_inverse_in_place();
            c *= a12.inverse().unwrap();
            let mut c2 = c;
            c2.frobenius_map_in_place(2);
            c *= c2;
            let mut x = c;
            fp12_cyclotomic_square_in_place(&mut x);
            assert_eq!(x, c.square());
            let mut y = c;
            y.cyclotomic_square_in_place();
            assert_eq!(x, y);
        }
    }

    #[test]
    fn bn254_in_place_ops_match_arkworks() {
        check_tower::<crate::bn254::fields::Fq12Config>();
    }

    #[test]
    fn bls12_381_in_place_ops_match_arkworks() {
        check_tower::<crate::bls12_381::fields::Fq12Config>();
    }
}
