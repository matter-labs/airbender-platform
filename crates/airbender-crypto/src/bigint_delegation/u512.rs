use super::{
    delegation,
    u256::{self, is_non_zero_vartime, low_word, U256},
    DelegatedModParams, DelegatedMontParams,
};
use crate::{BigInt, BigInteger};

pub(super) type U512 = BigInt<8>;
/// An unreduced product of two elements, or a short signed combination of such products
/// offset into the positive range: three 256-bit limbs
pub(super) type U768 = BigInt<12>;

/// The constants of the deferred reduction: multiples of the modulus squared that keep a
/// combination of products non-negative (see `fp2_mul_assign_lazy`, `WideFp2`)
pub trait DelegatedLazyParams: DelegatedMontParams<8> {
    fn four_modulus_squared() -> &'static U768;
    fn eight_modulus_squared() -> &'static U768;
}

static ZERO: U256 = U256::zero();
static ONE: U256 = U256::one();

struct ScratchSpace {
    copy_place_0: U512,
    copy_place_1: U512,
    sum_a: U512,
    sum_b: U512,
    wide_0: U768,
    wide_1: U768,
    wide_2: U768,
    low_word_scratch: U256,
    mul_copy_place_0: U256,
    mul_copy_place_1: U256,
    mul_copy_place_2: U256,
    mul_copy_place_3: U256,
    mul_copy_place_4: U256,
    mul_copy_place_5: U256,
}

#[cfg(not(test))]
static mut SCRATCH_SPACE: ScratchSpace = ScratchSpace {
    copy_place_0: U512::zero(),
    copy_place_1: U512::zero(),
    sum_a: U512::zero(),
    sum_b: U512::zero(),
    wide_0: U768::zero(),
    wide_1: U768::zero(),
    wide_2: U768::zero(),
    low_word_scratch: U256::zero(),
    mul_copy_place_0: U256::zero(),
    mul_copy_place_1: U256::zero(),
    mul_copy_place_2: U256::zero(),
    mul_copy_place_3: U256::zero(),
    mul_copy_place_4: U256::zero(),
    mul_copy_place_5: U256::zero(),
};

#[cfg(test)]
use std::cell::UnsafeCell;

#[cfg(test)]
thread_local! {
    static SCRATCH_SPACE: UnsafeCell<Box<ScratchSpace>> = UnsafeCell::new(Box::new(ScratchSpace {
        copy_place_0: U512::zero(),
        copy_place_1: U512::zero(),
        sum_a: U512::zero(),
        sum_b: U512::zero(),
        wide_0: U768::zero(),
        wide_1: U768::zero(),
        wide_2: U768::zero(),
        low_word_scratch: U256::zero(),
        mul_copy_place_0: U256::zero(),
        mul_copy_place_1: U256::zero(),
        mul_copy_place_2: U256::zero(),
        mul_copy_place_3: U256::zero(),
        mul_copy_place_4: U256::zero(),
        mul_copy_place_5: U256::zero(),
    }))
}

#[cfg(test)]
macro_rules! with_scratch {
    ($scratch:ident => $($body:tt)*) => {
        SCRATCH_SPACE.with(|cell| unsafe {
            let $scratch = &mut **cell.get();
            $($body)*
        })
    };
}

#[cfg(not(test))]
macro_rules! with_scratch {
    ($scratch:ident => $($body:tt)*) => {
        unsafe {
            let $scratch = &mut SCRATCH_SPACE;
            $($body)*
        }
    };
}

/// The `i`-th 256-bit limb of a wide value
#[inline(always)]
fn wide_limb(a: &U768, i: usize) -> &U256 {
    debug_assert!(i < 3);
    // SAFETY: a `BigInt<12>` is three 32-byte aligned `BigInt<4>`
    unsafe { &*(a as *const U768 as *const U256).add(i) }
}

#[inline(always)]
fn wide_limbs_mut(a: &mut U768) -> (&mut U256, &mut U256, &mut U256) {
    // SAFETY: as in `wide_limb`, three disjoint limbs
    unsafe {
        let ptr = a as *mut U768 as *mut U256;
        (&mut *ptr, &mut *ptr.add(1), &mut *ptr.add(2))
    }
}

/// `out = a b`, the full 768-bit product, for a modulus of at most 384 bits and operands below
/// `2^384` (so their high limbs are below `2^128` and their product has no high limb). 7
/// multiplications, no reduction.
/// # Safety
/// `DelegatedMontParams` should only provide references to statics.
unsafe fn mul_wide<T: DelegatedMontParams<8>>(a: &U512, b: &U512, out: &mut U768) {
    assert!(T::MODULUS_BITSIZE <= 384);
    let (a0, a1) = (as_low(a), as_high(a));
    let (b0, b1) = (as_low(b), as_high(b));
    let (o0, o1, o2) = wide_limbs_mut(out);
    with_scratch!(s => {
        // (o1, o0) = a0 b0
        delegation::memcpy(o0, a0);
        delegation::memcpy(o1, o0);
        u256::mul_low_assign(o0, b0);
        u256::mul_high_assign(o1, b0);
        // (o2, o1) += a1 b0
        let x = &mut s.mul_copy_place_0;
        delegation::memcpy(x, a1);
        delegation::memcpy(o2, x);
        u256::mul_low_assign(x, b0);
        u256::mul_high_assign(o2, b0);
        let carry = u256::add_assign(o1, x);
        u256::add_with_carry_bit(o2, &ZERO, carry);
        // (o2, o1) += a0 b1
        let y = &mut s.mul_copy_place_1;
        delegation::memcpy(x, a0);
        delegation::memcpy(y, x);
        u256::mul_low_assign(x, b1);
        u256::mul_high_assign(y, b1);
        let carry = u256::add_assign(o1, x);
        let overflow = u256::add_with_carry_bit(o2, y, carry);
        debug_assert!(!overflow);
        // o2 += low(a1 b1), the whole of that product
        delegation::memcpy(x, a1);
        u256::mul_low_assign(x, b1);
        let overflow = u256::add_assign(o2, x);
        debug_assert!(!overflow);
    })
}

/// `acc += x` on wide values, no overflow expected
#[inline(always)]
fn wide_add_assign(acc: &mut U768, x: &U768) {
    let (a0, a1, a2) = wide_limbs_mut(acc);
    let carry = u256::add_assign(a0, wide_limb(x, 0));
    let carry = u256::add_with_carry_bit(a1, wide_limb(x, 1), carry);
    let overflow = u256::add_with_carry_bit(a2, wide_limb(x, 2), carry);
    debug_assert!(!overflow);
}

/// `acc -= x` on wide values, `acc >= x` expected
#[inline(always)]
fn wide_sub_assign(acc: &mut U768, x: &U768) {
    let (a0, a1, a2) = wide_limbs_mut(acc);
    let borrow = u256::sub_assign(a0, wide_limb(x, 0));
    let borrow = u256::sub_with_carry_bit(a1, wide_limb(x, 1), borrow);
    let underflow = u256::sub_with_carry_bit(a2, wide_limb(x, 2), borrow);
    debug_assert!(!underflow);
}

/// `out = t / 2^512 mod modulus`, the Montgomery reduction of a wide value below `2^767`: two
/// rounds on 256-bit limbs as in `mul_assign_montgomery`, the result below `2 modulus`
/// (`t / 2^512 < 2^255 < modulus`).
/// # Safety
/// `DelegatedMontParams` should only provide references to statics.
unsafe fn reduce_wide<T: DelegatedMontParams<8>>(t: &U768, out: &mut U512) {
    let (n0, n1) = (as_low(T::modulus()), as_high(T::modulus()));
    let (t0, t1, t2) = (wide_limb(t, 0), wide_limb(t, 1), wide_limb(t, 2));
    with_scratch!(s => {
        // --- round 0: (u1, u0) = (t2, t1, t0 + k N) / 2^256 ---
        let k = &mut s.mul_copy_place_0;
        delegation::memcpy(k, t0);
        u256::mul_low_assign(k, T::reduction_const());
        let low_carry = is_non_zero_vartime(t0);
        // u0 = t1 + high(k n0) + low_carry + low(k n1)
        let u0 = &mut s.mul_copy_place_1;
        delegation::memcpy(u0, n0);
        u256::mul_high_assign(u0, k);
        let c0 = u256::add_with_carry_bit(u0, t1, low_carry);
        let v = &mut s.mul_copy_place_2;
        delegation::memcpy(v, n1);
        u256::mul_low_assign(v, k);
        let c1 = u256::add_assign(u0, v);
        // u1 = t2 + high(k n1) + c0 + c1 (below 2^256: t2 < 2^255)
        u256::mul_high_assign(k, n1);
        let overflow = u256::add_with_carry_bit(k, t2, c0);
        debug_assert!(!overflow);
        if c1 {
            let overflow = u256::add_assign(k, &ONE);
            debug_assert!(!overflow);
        }
        let (u0, u1) = (&*u0, &*k);

        // --- round 1: (out1, out0) = (u1, u0 + k' N) / 2^256 ---
        let k2 = &mut s.mul_copy_place_3;
        delegation::memcpy(k2, u0);
        u256::mul_low_assign(k2, T::reduction_const());
        let low_carry = is_non_zero_vartime(u0);
        let (out0, out1) = as_low_high_mut(out);
        delegation::memcpy(out0, n0);
        u256::mul_high_assign(out0, k2);
        let d0 = u256::add_with_carry_bit(out0, u1, low_carry);
        let v = &mut s.mul_copy_place_2;
        delegation::memcpy(v, n1);
        u256::mul_low_assign(v, k2);
        let d1 = u256::add_assign(out0, v);
        delegation::memcpy(out1, n1);
        u256::mul_high_assign(out1, k2);
        let extra = d0 as u64 + d1 as u64;
        if extra != 0 {
            s.low_word_scratch.0[0] = extra;
            let overflow = u256::add_assign(out1, &s.low_word_scratch);
            debug_assert!(!overflow);
        }
    })
}

/// `(a0 + a1 u) *= (b0 + b1 u)` in the quadratic extension with `u² = -1`, with the three
/// products of the Karatsuba multiplication reduced only once combined: `c0 = v0 - v1`,
/// `c1 = (a0 + a1)(b0 + b1) - v0 - v1`, each shifted by a multiple of the modulus squared to
/// stay non-negative. For operands below `2 modulus` the combinations are below `24 modulus²`,
/// which `reduce_wide` takes. Two reductions instead of three, and the sums of the operands
/// are plain additions.
/// # Safety
/// `DelegatedLazyParams` should only provide references to statics.
pub unsafe fn fp2_mul_assign_lazy<T: DelegatedLazyParams>(
    a0: &mut U512,
    a1: &mut U512,
    b0: &U512,
    b1: &U512,
) {
    with_scratch!(s => {
        mul_wide::<T>(a0, b0, &mut s.wide_0);
        mul_wide::<T>(a1, b1, &mut s.wide_1);
        // the sums of the operands, below 4 modulus < 2^384
        copy(&mut s.sum_a, a0);
        let (l, h) = as_low_high_mut(&mut s.sum_a);
        let carry = u256::add_assign(l, as_low(a1));
        u256::add_with_carry_bit(h, as_high(a1), carry);
        copy(&mut s.sum_b, b0);
        let (l, h) = as_low_high_mut(&mut s.sum_b);
        let carry = u256::add_assign(l, as_low(b1));
        u256::add_with_carry_bit(h, as_high(b1), carry);
        mul_wide::<T>(&s.sum_a, &s.sum_b, &mut s.wide_2);
        // c1 = v2 + 8 N² - v0 - v1
        wide_add_assign(&mut s.wide_2, T::eight_modulus_squared());
        wide_sub_assign(&mut s.wide_2, &s.wide_0);
        wide_sub_assign(&mut s.wide_2, &s.wide_1);
        // c0 = v0 + 4 N² - v1
        wide_add_assign(&mut s.wide_0, T::four_modulus_squared());
        wide_sub_assign(&mut s.wide_0, &s.wide_1);
        reduce_wide::<T>(&s.wide_0, a0);
        reduce_wide::<T>(&s.wide_2, a1);
    })
}

pub(super) fn as_low(a: &U512) -> &U256 {
    unsafe {
        let ptr = a as *const U512 as *const U256;

        debug_assert_eq!(ptr.addr() % 32, 0);

        ptr.as_ref().unwrap()
    }
}

pub(super) fn as_high(a: &U512) -> &U256 {
    unsafe {
        let ptr = (a as *const U512 as *const U256).add(1);

        debug_assert_eq!(ptr.addr() % 32, 0);

        ptr.as_ref().unwrap()
    }
}

/// Copies `b` into `a` with two delegated memcopies instead of a `memcpy` call
#[inline(always)]
pub fn copy_assign(a: &mut U512, b: &U512) {
    let (a_low, a_high) = as_low_high_mut(a);
    // SAFETY: a `U512` is two 32-byte aligned `U256` halves
    let (b_low, b_high) = unsafe {
        let low = b as *const U512 as *const U256;
        (&*low, &*low.add(1))
    };
    delegation::memcpy(a_low, b_low);
    delegation::memcpy(a_high, b_high);
}

/// Initializes the (32-byte aligned) memory at `dst` with a copy of `src`
/// # Safety
/// `dst` must be valid for writes of a `U512` and 32-byte aligned
#[inline(always)]
pub unsafe fn init_copy(dst: *mut U512, src: &U512) {
    let dst = dst.cast::<U256>();
    delegation::memcpy_to_ptr(dst, as_low(src));
    delegation::memcpy_to_ptr(dst.add(1), as_high(src));
}

pub(super) fn as_low_high_mut(a: &mut U512) -> (&mut U256, &mut U256) {
    unsafe {
        let low = a as *mut U512 as *mut U256;
        let high = (a as *mut U512 as *mut U256).add(1);

        // check alignment for U256
        debug_assert_eq!(low.addr() % 32, 0);
        debug_assert_eq!(high.addr() % 32, 0);

        (low.as_mut().unwrap(), high.as_mut().unwrap())
    }
}

fn copy(dst: &mut U512, src: &U512) {
    let (low_dst, high_dst) = as_low_high_mut(dst);

    delegation::memcpy(low_dst, as_low(src));
    delegation::memcpy(high_dst, as_high(src));
}

/// Tries to get `self` in the range `[0..modulus)`.
/// Note: we assume `self < 2*modulus`, otherwise the result might not be in the range
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
unsafe fn sub_mod_with_carry<T: DelegatedModParams<8>>(a: &mut U512, carry: bool) {
    let (low, high) = as_low_high_mut(a);

    let borrow = u256::sub_assign(low, as_low(T::modulus()));
    let borrow = u256::sub_with_carry_bit(high, as_high(T::modulus()), borrow);

    if borrow & !carry {
        let carry = u256::add_assign(low, as_low(T::modulus()));
        u256::add_with_carry_bit(high, as_high(T::modulus()), carry);
    }
}

/// Brings `a < 4 modulus` into the redundant range `[0, 2 modulus)`: one conditional
/// subtraction of `2 modulus` (see `DelegatedModParams::REDUNDANT`)
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
unsafe fn reduce_below_double_modulus<T: DelegatedModParams<8>>(a: &mut U512) {
    debug_assert!(T::REDUNDANT);
    let (low, high) = as_low_high_mut(a);
    let (n0, n1) = (as_low(T::double_modulus()), as_high(T::double_modulus()));
    let borrow = u256::sub_assign(low, n0);
    let borrow = u256::sub_with_carry_bit(high, n1, borrow);
    if borrow {
        let carry = u256::add_assign(low, n0);
        u256::add_with_carry_bit(high, n1, carry);
    }
}

/// The canonical representative of `a < 2 modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
pub unsafe fn reduce_to_canonical<T: DelegatedModParams<8>>(a: &mut U512) {
    sub_mod_with_carry::<T>(a, false);
}

/// `a == b` as residues, for `a, b < 2 modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
pub unsafe fn eq_mod<T: DelegatedModParams<8>>(a: &U512, b: &U512) -> bool {
    with_scratch!(s => {
        copy(&mut s.copy_place_0, a);
        reduce_to_canonical::<T>(&mut s.copy_place_0);
        copy(&mut s.copy_place_1, b);
        reduce_to_canonical::<T>(&mut s.copy_place_1);
        delegation::eq(as_low(&s.copy_place_0), as_low(&s.copy_place_1)) != 0
            && delegation::eq(as_high(&s.copy_place_0), as_high(&s.copy_place_1)) != 0
    })
}

/// `a == 0` as a residue, for `a < 2 modulus`: zero or the modulus. The modulus is odd, so the
/// lowest word tells which of the two `a` can be: at most one of them is compared, and for most
/// values none (see `u256::low_word`).
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
pub unsafe fn is_zero_mod<T: DelegatedModParams<8>>(a: &U512) -> bool {
    let (low, high) = (as_low(a), as_high(a));
    let (n0, n1) = (as_low(T::modulus()), as_high(T::modulus()));
    debug_assert!(low_word(n0) != 0);
    match low_word(low) {
        0 => delegation::eq(low, &ZERO) != 0 && delegation::eq(high, &ZERO) != 0,
        word if word == low_word(n0) => {
            delegation::eq(low, n0) != 0 && delegation::eq(high, n1) != 0
        }
        _ => false,
    }
}

/// Computes `self = self + rhs mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn add_mod_assign<T: DelegatedModParams<8>>(a: &mut U512, b: &U512) {
    let (low, high) = as_low_high_mut(a);

    let carry = u256::add_assign(low, as_low(b));
    let carry = u256::add_with_carry_bit(high, as_high(b), carry);

    if T::REDUNDANT {
        // both below 2 modulus: the sum is below 4 modulus < 2^512, no carry
        debug_assert!(!carry);
        reduce_below_double_modulus::<T>(a);
    } else {
        sub_mod_with_carry::<T>(a, carry);
    }
}

/// Computes `self = self - rhs mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn sub_mod_assign<T: DelegatedModParams<8>>(a: &mut U512, b: &U512) {
    let (low, high) = as_low_high_mut(a);

    let borrow = u256::sub_assign(low, as_low(b));
    let borrow = u256::sub_with_carry_bit(high, as_high(b), borrow);

    if borrow {
        // the redundant representation adds `2 modulus` back: the difference is above
        // `-2 modulus`, so the result is in `(0, 2 modulus)`
        let n = if T::REDUNDANT {
            T::double_modulus()
        } else {
            T::modulus()
        };
        let carry = u256::add_assign(low, as_low(n));
        u256::add_with_carry_bit(high, as_high(n), carry);
    }
}

/// Computes `self = self + self mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn double_mod_assign<T: DelegatedModParams<8>>(a: &mut U512) {
    with_scratch!(s => {
        copy(&mut s.copy_place_0, a);
        add_mod_assign::<T>(a, &s.copy_place_0);
    })
}

/// Computes `self = -self mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn neg_mod_assign<T: DelegatedModParams<8>>(a: &mut U512) {
    let (low, high) = as_low_high_mut(a);

    // zero stays as it is; a non-zero lowest word rules it out without a comparison
    if !u256::is_zero(low) || delegation::eq(high, &ZERO) == 0 {
        // in the redundant representation `a` is in `(0, 2 modulus)`, and so is
        // `2 modulus - a`
        let n = if T::REDUNDANT {
            T::double_modulus()
        } else {
            T::modulus()
        };
        let borrow = u256::sub_and_negate_assign(low, as_low(n));
        u256::sub_and_negate_with_carry(high, as_high(n), borrow);
    }
}

/// Compute `self = self * rhs mod modulus` using montgomery reduction.
/// Both `self` and `rhs` are assumed to be in montgomery form.
/// The reduction constant is expected to be `-1/modulus mod 2^256`
///
/// Note: we assume that modulus is strictly less than 512 bits
/// # Safety
/// `DelegationMontParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn mul_assign_montgomery<T: DelegatedMontParams<8>>(a: &mut U512, b: &U512) {
    // otherwise we may get a carry in the final addition
    assert!(T::MODULUS_BITSIZE < 512);
    // for a modulus of at most 384 bits the high limbs of the (reduced) operands are below
    // 2^128, so their product has no high limb
    let small_high_limbs = T::MODULUS_BITSIZE <= 384;

    // Two rounds of `t = (t + a b_i + k N) / 2^256` on 256-bit limbs (CIOS), `k` chosen so that
    // the low limb of `t + a b_i + k N` vanishes. That limb is therefore never computed: with
    // `k = t0 (-N^-1) mod 2^256`, `t0 + low(k n0)` is `0` or `2^256`, so its carry is `t0 != 0`.
    // Carries are folded into the next addition with the carry bit instead of separate `+ 1`
    // additions. Every scratch limb is a delegation-aligned `U256`.
    with_scratch!(s => {
        let (n0, n1) = (as_low(T::modulus()), as_high(T::modulus()));
        let (b0, b1) = (as_low(b), as_high(b));

        // --- round 0: (t2, t1, t0) = a b0 ---
        // t0 = low(a0 b0), c = high(a0 b0)
        let t0 = &mut s.mul_copy_place_0;
        delegation::memcpy(t0, as_low(a));
        let c = &mut s.mul_copy_place_1;
        delegation::memcpy(c, t0);
        u256::mul_low_assign(t0, b0);
        u256::mul_high_assign(c, b0);
        // t1 = low(a1 b0) + c, t2 = high(a1 b0) (+ t2_carry, folded below)
        let t1 = &mut s.mul_copy_place_2;
        delegation::memcpy(t1, as_high(a));
        let t2 = &mut s.mul_copy_place_3;
        delegation::memcpy(t2, t1);
        u256::mul_low_assign(t1, b0);
        let t2_carry = u256::add_assign(t1, c);
        u256::mul_high_assign(t2, b0);

        // --- reduction 0: (t1, t0) = (t2, t1, t0 + k N) / 2^256 ---
        let k = &mut s.mul_copy_place_1;
        delegation::memcpy(k, t0);
        u256::mul_low_assign(k, T::reduction_const());
        let low_carry = is_non_zero_vartime(t0);
        // u = t1 + high(k n0) + low_carry + low(k n1), the new t0
        let u = &mut s.mul_copy_place_0;
        delegation::memcpy(u, n0);
        u256::mul_high_assign(u, k);
        let c0 = u256::add_with_carry_bit(u, t1, low_carry);
        let v = &mut s.mul_copy_place_4;
        delegation::memcpy(v, n1);
        u256::mul_low_assign(v, k);
        let c1 = u256::add_assign(u, v);
        // k = t2 + high(k n1) + t2_carry + c0 + c1, the new t1 (below 2N / 2^256: no overflow)
        u256::mul_high_assign(k, n1);
        let overflow = u256::add_with_carry_bit(k, t2, t2_carry);
        debug_assert!(!overflow);
        let extra = c0 as u64 + c1 as u64;
        if extra != 0 {
            s.low_word_scratch.0[0] = extra;
            let overflow = u256::add_assign(k, &s.low_word_scratch);
            debug_assert!(!overflow);
        }
        let (t0, t1) = (u, k);

        // --- round 1: (t2, t1, t0) += a b1 ---
        let x = &mut s.mul_copy_place_2;
        delegation::memcpy(x, as_low(a));
        let xh = &mut s.mul_copy_place_3;
        delegation::memcpy(xh, x);
        u256::mul_low_assign(x, b1);
        u256::mul_high_assign(xh, b1);
        let c0 = u256::add_assign(t0, x);
        let c1 = u256::add_with_carry_bit(t1, xh, c0);
        let y = &mut s.mul_copy_place_2;
        delegation::memcpy(y, as_high(a));
        let t2 = &mut s.mul_copy_place_3;
        if !small_high_limbs {
            delegation::memcpy(t2, y);
        }
        u256::mul_low_assign(y, b1);
        let c2 = u256::add_assign(t1, y);
        // t2 = high(a1 b1) (+ c1 + c2, folded below), zero for small high limbs
        if !small_high_limbs {
            u256::mul_high_assign(t2, b1);
        }

        // --- reduction 1, the result lands in `a` ---
        let k = &mut s.mul_copy_place_2;
        delegation::memcpy(k, t0);
        u256::mul_low_assign(k, T::reduction_const());
        let low_carry = is_non_zero_vartime(t0);
        let (a0, a1) = as_low_high_mut(a);
        // a0 = t1 + high(k n0) + low_carry + low(k n1)
        delegation::memcpy(a0, n0);
        u256::mul_high_assign(a0, k);
        let d0 = u256::add_with_carry_bit(a0, t1, low_carry);
        let v = &mut s.mul_copy_place_4;
        delegation::memcpy(v, n1);
        u256::mul_low_assign(v, k);
        let d1 = u256::add_assign(a0, v);
        // a1 = t2 + high(k n1) + c1 + c2 + d0 + d1 (the result is below 2N < 2^512: no overflow)
        delegation::memcpy(a1, n1);
        u256::mul_high_assign(a1, k);
        let mut extra = c2 as u64 + d0 as u64 + d1 as u64;
        if small_high_limbs {
            extra += c1 as u64;
        } else {
            let overflow = u256::add_with_carry_bit(a1, t2, c1);
            debug_assert!(!overflow);
        }
        if extra != 0 {
            s.low_word_scratch.0[0] = extra;
            let overflow = u256::add_assign(a1, &s.low_word_scratch);
            debug_assert!(!overflow);
        }

        // one conditional subtraction brings the result below N; the redundant
        // representation keeps it below 2N, which the product of two such operands is
        // anyway (`2^512 > 4N`)
        if !T::REDUNDANT {
            let borrow = u256::sub_assign(a0, n0);
            let borrow = u256::sub_with_carry_bit(a1, n1, borrow);
            if borrow {
                let carry = u256::add_assign(a0, n0);
                let _ = u256::add_with_carry_bit(a1, n1, carry);
            }
        }

        debug_assert!(a.0[6..8].iter().all(|&x| x == 0));
    })
}

/// Compute `self = self^2 mod modulus` using montgomery reduction.
/// `self` should be in montgomery form.
/// The reduction constant is expected to be `-1/modulus mod 2^256`
/// # Safety
/// `DelegationMontParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn square_assign_montgomery<T: DelegatedMontParams<8>>(a: &mut U512) {
    with_scratch!(s => {
        copy(&mut s.copy_place_0, a);
        mul_assign_montgomery::<T>(a, &s.copy_place_0);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark_ff_delegation::BigInt;

    // A simple modulus for testing: use a large prime in U512 form
    // We use BLS12-381 Fq modulus padded to 8 limbs
    #[derive(Default, Debug)]
    struct TestMod;

    static TEST_MODULUS: BigInt<8> = BigInt([
        13402431016077863595u64,
        2210141511517208575u64,
        7435674573564081700u64,
        7239337960414712511u64,
        5412103778470702295u64,
        1873798617647539866u64,
        0,
        0,
    ]);

    impl DelegatedModParams<8> for TestMod {
        const MODULUS_BITSIZE: usize = 381;

        fn modulus() -> &'static BigInt<8> {
            &TEST_MODULUS
        }
    }

    #[test]
    fn test_neg_mod_zero() {
        // Negating zero should give zero
        let mut a = U512::zero();
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }
        assert!(a.is_zero(), "neg(0) should be 0");
    }

    #[test]
    fn test_neg_mod_with_only_low_nonzero() {
        // Regression test: when only low part is non-zero, negation should still work
        let mut a = U512::zero();
        a.0[0] = 1; // low part is non-zero, high part is zero

        let original = a;
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }

        // Result should NOT be the original (unless original was 0, which it isn't)
        assert!(!a.is_zero(), "neg(1) should not be 0");
        assert_ne!(a.0, original.0, "neg(a) should differ from a when a != 0");

        // Verify: neg(a) + a should equal modulus (in non-reduced form) or 0 mod modulus
        // Actually let's verify by negating twice
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }
        assert_eq!(a.0, original.0, "neg(neg(a)) should equal a");
    }

    #[test]
    fn test_neg_mod_with_only_high_nonzero() {
        // Regression test: when only high part is non-zero, negation should still work
        let mut a = U512::zero();
        a.0[4] = 1; // high part is non-zero (limb index 4 is in the high U256), low part is zero

        let original = a;
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }

        assert!(!a.is_zero(), "neg(a) should not be 0 when a != 0");
        assert_ne!(a.0, original.0, "neg(a) should differ from a when a != 0");

        // Verify by negating twice
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }
        assert_eq!(a.0, original.0, "neg(neg(a)) should equal a");
    }

    /// `a` with its lowest word replaced
    fn with_low_word(mut a: U512, word: u32) -> U512 {
        a.0[0] = (a.0[0] & !u64::from(u32::MAX)) | u64::from(word);
        a
    }

    #[test]
    fn test_neg_mod_with_zero_lowest_word() {
        // the lowest word does not tell it from zero
        let a = with_low_word(TEST_MODULUS, 0);
        let mut negated = a;
        unsafe {
            neg_mod_assign::<TestMod>(&mut negated);
        }
        assert_ne!(negated.0, a.0, "neg(a) should differ from a when a != 0");
        unsafe {
            neg_mod_assign::<TestMod>(&mut negated);
        }
        assert_eq!(negated.0, a.0, "neg(neg(a)) should equal a");
    }

    /// Zero and the modulus are zero, and the integers that their lowest word does not tell
    /// apart from them are compared (and not zero); the others are not compared at all
    #[test]
    fn test_is_zero_mod() {
        let calls = || delegation::DELEGATION_CALLS.with(|c| c.get());
        let modulus = TEST_MODULUS;
        let mut high_only = U512::zero();
        high_only.0[4] = 1;
        let mut modulus_plus_high = modulus;
        modulus_plus_high.0[4] += 1;
        for (a, zero, compared) in [
            (U512::zero(), true, true),
            (modulus, true, true),
            (with_low_word(modulus, 0), false, true),
            (high_only, false, true),
            (modulus_plus_high, false, true),
            (with_low_word(U512::zero(), 1), false, false),
            (with_low_word(modulus, 1), false, false),
        ] {
            let before = calls();
            assert_eq!(unsafe { is_zero_mod::<TestMod>(&a) }, zero, "{a:?}");
            assert_eq!(calls() > before, compared, "{a:?}");
        }
    }

    #[test]
    fn test_neg_mod_both_parts_nonzero() {
        // When both parts are non-zero
        let mut a = U512::zero();
        a.0[0] = 42; // low part non-zero
        a.0[4] = 17; // high part non-zero

        let original = a;
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }

        assert!(!a.is_zero(), "neg(a) should not be 0 when a != 0");

        // Verify by negating twice
        unsafe {
            neg_mod_assign::<TestMod>(&mut a);
        }
        assert_eq!(a.0, original.0, "neg(neg(a)) should equal a");
    }
}
