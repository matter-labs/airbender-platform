use super::{
    delegation,
    u256::{self, U256},
    DelegatedModParams, DelegatedMontParams,
};
use crate::{BigInt, BigInteger};

pub(super) type U512 = BigInt<8>;

static ZERO: U256 = U256::zero();
static ONE: U256 = U256::one();

struct ScratchSpace {
    copy_place_0: U512,
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

/// Computes `self = self + rhs mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn add_mod_assign<T: DelegatedModParams<8>>(a: &mut U512, b: &U512) {
    let (low, high) = as_low_high_mut(a);

    let carry = u256::add_assign(low, as_low(b));
    let carry = u256::add_with_carry_bit(high, as_high(b), carry);

    sub_mod_with_carry::<T>(a, carry);
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
        let carry = u256::add_assign(low, as_low(T::modulus()));
        u256::add_with_carry_bit(high, as_high(T::modulus()), carry);
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

    let is_low_zero = delegation::eq(low, &ZERO) != 0;
    let is_high_zero = delegation::eq(high, &ZERO) != 0;

    if !is_low_zero || !is_high_zero {
        let borrow = u256::sub_and_negate_assign(low, as_low(T::modulus()));
        u256::sub_and_negate_with_carry(high, as_high(T::modulus()), borrow);
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
        let low_carry = !u256::is_zero(t0);
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
        delegation::memcpy(t2, y);
        u256::mul_low_assign(y, b1);
        let c2 = u256::add_assign(t1, y);
        // t2 = high(a1 b1) (+ c1 + c2, folded below)
        u256::mul_high_assign(t2, b1);

        // --- reduction 1, the result lands in `a` ---
        let k = &mut s.mul_copy_place_2;
        delegation::memcpy(k, t0);
        u256::mul_low_assign(k, T::reduction_const());
        let low_carry = !u256::is_zero(t0);
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
        let overflow = u256::add_with_carry_bit(a1, t2, c1);
        debug_assert!(!overflow);
        let extra = c2 as u64 + d0 as u64 + d1 as u64;
        if extra != 0 {
            s.low_word_scratch.0[0] = extra;
            let overflow = u256::add_assign(a1, &s.low_word_scratch);
            debug_assert!(!overflow);
        }

        // one conditional subtraction brings the result below N
        let borrow = u256::sub_assign(a0, n0);
        let borrow = u256::sub_with_carry_bit(a1, n1, borrow);
        if borrow {
            let carry = u256::add_assign(a0, n0);
            let _ = u256::add_with_carry_bit(a1, n1, carry);
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
