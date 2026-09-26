use super::{delegation, DelegatedBarretParams, DelegatedModParams, DelegatedMontParams};
use crate::{BigInt, BigInteger};
use core::{fmt::Debug, marker::PhantomData};

pub(super) type U256 = BigInt<4>;

static ONE: U256 = U256::one();
static ZERO: U256 = U256::zero();
static TWO: U256 = BigInt::<4>([2, 0, 0, 0]);

struct ScratchSpace {
    copy_place_0: U256,
    copy_place_1: U256,
    copy_place_2: U256,
    copy_place_3: U256,
    scratch: U256,
}

#[cfg(not(test))]
static mut SCRATCH_SPACE: ScratchSpace = ScratchSpace {
    copy_place_0: U256::zero(),
    copy_place_1: U256::zero(),
    copy_place_2: U256::zero(),
    copy_place_3: U256::zero(),
    scratch: U256::zero(),
};

#[cfg(test)]
use std::cell::UnsafeCell;

#[cfg(test)]
thread_local! {
    static SCRATCH_SPACE: UnsafeCell<Box<ScratchSpace>> = UnsafeCell::new(Box::new(ScratchSpace {
        copy_place_0: U256::zero(),
        copy_place_1: U256::zero(),
        copy_place_2: U256::zero(),
        copy_place_3: U256::zero(),
        scratch: U256::zero(),
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

pub const fn from_bytes_unchecked(bytes: &[u8; 32]) -> U256 {
    BigInt::<4>([
        u64::from_le_bytes([
            bytes[31], bytes[30], bytes[29], bytes[28], bytes[27], bytes[26], bytes[25], bytes[24],
        ]),
        u64::from_le_bytes([
            bytes[23], bytes[22], bytes[21], bytes[20], bytes[19], bytes[18], bytes[17], bytes[16],
        ]),
        u64::from_le_bytes([
            bytes[15], bytes[14], bytes[13], bytes[12], bytes[11], bytes[10], bytes[9], bytes[8],
        ]),
        u64::from_le_bytes([
            bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0],
        ]),
    ])
}

pub fn to_be_bytes(a: U256) -> [u8; 32] {
    let mut r = [0u8; 32];
    r[0..8].copy_from_slice(&a.0[3].to_be_bytes());
    r[8..16].copy_from_slice(&a.0[2].to_be_bytes());
    r[16..24].copy_from_slice(&a.0[1].to_be_bytes());
    r[24..32].copy_from_slice(&a.0[0].to_be_bytes());

    r
}

#[inline(always)]
/// adds `rhs` to `self` and returns the carry
pub fn add_assign(a: &mut U256, b: &U256) -> bool {
    delegation::add(a, b) != 0
}

#[inline(always)]
/// subtracts `rhs` from `self` and returns the borrow
pub fn sub_assign(a: &mut U256, b: &U256) -> bool {
    delegation::sub(a, b) != 0
}

#[inline(always)]
/// subtracts `self` from `rhs` and reutrns the borrow
pub fn sub_and_negate_assign(a: &mut U256, b: &U256) -> bool {
    delegation::sub_and_negate(a, b) != 0
}

#[inline(always)]
/// multiplies `self` with `rhs` and storest the lowest 256 bits in self
pub fn mul_low_assign(a: &mut U256, b: &U256) {
    delegation::mul_low(a, b);
}

#[inline(always)]
/// multiplies `self` with `rhs` and storest the highest 256 bits in self
pub fn mul_high_assign(a: &mut U256, b: &U256) {
    delegation::mul_high(a, b);
}

#[inline(always)]
pub fn mul_wide(a: &U256, b: &U256) -> (U256, U256) {
    let mut low = U256::zero();
    let mut high = U256::zero();

    delegation::memcpy(&mut low, a);
    delegation::memcpy(&mut high, a);

    delegation::mul_low(&mut low, b);
    delegation::mul_high(&mut high, b);

    (low, high)
}

#[inline(always)]
/// computes `self = self - rhs - carry` and returns the borrow
pub fn sub_with_carry_bit(a: &mut U256, b: &U256, carry: bool) -> bool {
    delegation::sub_with_carry_bit(a, b, carry) != 0
}

#[inline(always)]
/// computes `self = self + rhs + carry` and returns the carry
pub fn add_with_carry_bit(a: &mut U256, b: &U256, carry: bool) -> bool {
    delegation::add_with_carry_bit(a, b, carry) != 0
}

#[inline(always)]
/// computes `self = rhs - self - carry` and returns the borrow
pub fn sub_and_negate_with_carry(a: &mut U256, b: &U256, carry: bool) -> bool {
    delegation::sub_and_negate_with_carry_bit(a, b, carry) != 0
}

#[inline(always)]
/// Tries to get `self` in the range `[0..modulus)`.
/// Note: we assume `self < 2*modulus`, otherwise the result might not be in the range
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
unsafe fn sub_mod_with_carry<T: DelegatedModParams<4>>(a: &mut U256, carry: bool) {
    let borrow = delegation::sub(a, T::modulus()) != 0;
    if borrow && !carry {
        delegation::add(a, T::modulus());
    }
}

#[inline(always)]
/// Brings `a < 4 modulus` into the redundant range `[0, 2 modulus)`: one conditional
/// subtraction of `2 modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
unsafe fn reduce_below_double_modulus<T: DelegatedModParams<4>>(a: &mut U256) {
    debug_assert!(T::REDUNDANT);
    let borrow = delegation::sub(a, T::double_modulus()) != 0;
    if borrow {
        delegation::add(a, T::double_modulus());
    }
}

#[inline(always)]
/// The canonical representative of `a < 2 modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn reduce_to_canonical<T: DelegatedModParams<4>>(a: &mut U256) {
    sub_mod_with_carry::<T>(a, false);
}

#[inline(always)]
/// computes `self = self + rhs mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn add_mod_assign<T: DelegatedModParams<4>>(a: &mut U256, b: &U256) {
    if T::REDUNDANT {
        // both below 2 modulus: the sum is below 4 modulus < 2^256, no carry
        let carry = delegation::add(a, b) != 0;
        debug_assert!(!carry);
        reduce_below_double_modulus::<T>(a);
    } else {
        let carry = delegation::add(a, b) != 0;
        sub_mod_with_carry::<T>(a, carry);
    }
}

#[inline(always)]
/// computes `self = self - rhs mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn sub_mod_assign<T: DelegatedModParams<4>>(a: &mut U256, b: &U256) {
    let borrow = delegation::sub(a, b);
    if borrow != 0 {
        // redundant: `a - b > -2 modulus`, so adding `2 modulus` lands in `(0, 2 modulus)`
        let back = if T::REDUNDANT {
            T::double_modulus()
        } else {
            T::modulus()
        };
        delegation::add(a, back);
    }
}

#[inline(always)]
/// Computes `self = self + self mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn double_mod_assign<T: DelegatedModParams<4>>(a: &mut U256) {
    if T::REDUNDANT {
        // `2a` as the product by 2: one delegation, where an addition would need a copy of
        // `a` first (a delegation cannot take the same operand twice). `a` is below twice the
        // modulus, below 2^255 (a redundant field has `4 modulus < 2^256`), so the product does
        // not overflow.
        debug_assert!(a.0[3] >> 63 == 0);
        delegation::mul_low(a, &TWO);
        reduce_below_double_modulus::<T>(a);
    } else if T::MODULUS_BITSIZE < 256 {
        // the same for a canonical `a` below a modulus of at most 255 bits
        debug_assert!(a.0[3] >> 63 == 0);
        delegation::mul_low(a, &TWO);
        sub_mod_with_carry::<T>(a, false);
    } else {
        // a 256-bit modulus (secp256r1): the sum can carry, which the product would drop
        with_scratch!(s => {
            delegation::memcpy(&mut s.copy_place_0, a);
            let carry = delegation::add(a, &s.copy_place_0) != 0;
            sub_mod_with_carry::<T>(a, carry);
        })
    }
}

#[inline(always)]
/// Computes `self = -self mod modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn neg_mod_assign<T: DelegatedModParams<4>>(a: &mut U256) {
    if T::REDUNDANT {
        // modulus - a: a representative of `-a` in `[0, modulus]` for `a <= modulus`; for a
        // larger `a` it borrows, and `3 modulus - a` is in `(modulus, 2 modulus)`
        let borrow = delegation::sub_and_negate(a, T::modulus()) != 0;
        if borrow {
            delegation::add(a, T::double_modulus());
        }
    } else if !is_zero(a) {
        delegation::sub_and_negate(a, T::modulus());
    }
}

#[inline(always)]
/// `a = b mod modulus` for representatives in `[0, 2 modulus)`: the difference is a multiple
/// of the modulus, i.e. `-modulus`, `0` or `modulus`
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn eq_mod<T: DelegatedModParams<4>>(a: &U256, b: &U256) -> bool {
    if !T::REDUNDANT {
        return delegation::eq(a, b) != 0;
    }
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);
        let borrow = delegation::sub(&mut s.copy_place_0, b) != 0;
        if borrow {
            // the difference is in (-2 modulus, 0): equal iff it is -modulus
            delegation::add(&mut s.copy_place_0, T::modulus());
            is_zero(&s.copy_place_0)
        } else {
            is_zero_mod::<T>(&s.copy_place_0)
        }
    })
}

#[inline(always)]
pub fn eq(a: &U256, b: &U256) -> bool {
    delegation::eq(a, b) != 0
}

/// The lowest word of `a`. Only `2^-32` of the integers have it zero, and the values that the
/// zero tests get are rarely zero, so the tests look at it first: the delegation compares only
/// the integers that it does not tell apart from zero (or from the modulus).
#[inline(always)]
pub(super) fn low_word(a: &U256) -> u32 {
    a.0[0] as u32
}

#[inline(always)]
/// Whether `a` is zero: one word test, and a comparison only if the lowest word is zero
pub fn is_zero(a: &U256) -> bool {
    low_word(a) == 0 && delegation::eq(a, &ZERO) != 0
}

#[inline(always)]
/// it takes `a` as mutable for the purposes of delegation calls, but doesn't mutate it
pub fn is_one(a: &U256) -> bool {
    delegation::eq(a, &ONE) != 0
}

#[inline(always)]
/// Whether `a` is `0` or the modulus, the representatives of zero below `2 modulus`. The
/// modulus is odd, so the lowest word tells which of the two `a` can be: at most one of them is
/// compared, and for most values none (see `low_word`).
/// # Safety
/// `DelegationModParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn is_zero_mod<T: DelegatedModParams<4>>(a: &U256) -> bool {
    let modulus = T::modulus();
    debug_assert!(low_word(modulus) != 0);
    match low_word(a) {
        0 => delegation::eq(a, &ZERO) != 0,
        word if word == low_word(modulus) => delegation::eq(a, modulus) != 0,
        _ => false,
    }
}

pub fn lt(a: &U256, b: &U256) -> bool {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);

        // if we get a borrow, then self < other
        delegation::sub(&mut s.copy_place_0, b) != 0
    })
}

pub fn leq(a: &U256, b: &U256) -> bool {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);

        // if we get a borrow, then self < other
        delegation::eq(&s.copy_place_0, b) != 0 || delegation::sub(&mut s.copy_place_0, b) != 0
    })
}

#[inline(always)]
/// modular multiplication with barret reduction
/// # Safety
/// `DelegationBarretParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn mul_assign_barret<T: DelegatedBarretParams<4>>(a: &mut U256, b: &U256) {
    with_scratch!(s => {
        // we will keep high part of product in temp0 until the very end
        delegation::memcpy(&mut s.copy_place_1, a);

        delegation::mul_low(a, b);
        delegation::mul_high(&mut s.copy_place_1, b);

        delegation::memcpy(&mut s.copy_place_2, &s.copy_place_1);

        // multiply copy_place0 by 2^256 - modulus
        delegation::mul_low(&mut s.copy_place_2, T::neg_modulus());
        delegation::mul_high(&mut s.copy_place_1, T::neg_modulus());

        // add and propagate the carry
        let carry = delegation::add(a, &s.copy_place_2) != 0;
        if carry {
            delegation::add(&mut s.copy_place_1, &ONE);
        }

        delegation::mul_low(&mut s.copy_place_1, T::neg_modulus());

        let carry = delegation::add(a, &s.copy_place_1) != 0;
        sub_mod_with_carry::<T>(a, carry);
    })
}

#[inline(always)]
/// # Safety
/// `DelegationBarretParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn square_assign_barret<T: DelegatedBarretParams<4>>(a: &mut U256) {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);

        mul_assign_barret::<T>(a, &s.copy_place_0);
    })
}

#[inline(always)]
/// # Safety
/// `DelegationMontParams` should only provide references to mutable statics.
/// It is the responsibility of the caller to make sure that is the case
pub unsafe fn square_assign_montgomery<T: DelegatedMontParams<4>>(a: &mut U256) {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);

        mul_assign_montgomery::<T>(a, &s.copy_place_0);
    })
}

#[inline(always)]
/// Modular multiplication with montgomery reduction.
/// It's the responsibility of the caller to make sure the parameters are in montgomery form.
/// # Safety
///
pub unsafe fn mul_assign_montgomery<T: DelegatedMontParams<4>>(a: &mut U256, b: &U256) {
    with_scratch!(s => {
        // (a, low) = a * b: the high half stays in `a`
        let low = &mut s.copy_place_1;
        delegation::memcpy(low, a);
        delegation::mul_low(low, b);
        delegation::mul_high(a, b);

        // Montgomery reduction: with `m = low * (-N^-1) mod 2^256`, `low + low(m N) = 0 mod 2^256`,
        // so the low half of `a b + m N` is never needed, only its carry: both terms are in
        // `[0, 2^256)` and add up to `0` or `2^256`, i.e. the carry is set iff `low != 0`.
        let carry = is_non_zero_vartime(low);
        // low = high(m N)
        delegation::mul_low(low, T::reduction_const());
        delegation::mul_high(low, T::modulus());

        // a = (a b + m N) / 2^256 = high(a b) + high(m N) + carry, then one conditional
        // subtraction; in the redundant representation the inputs are below 2N and 4N < 2^256,
        // so the result is below 2N already and stays as it is
        let carry = delegation::add_with_carry_bit(a, low, carry) != 0;
        if T::REDUNDANT {
            debug_assert!(!carry);
        } else {
            sub_mod_with_carry::<T>(a, carry);
        }
    })
}

// ---------------------------------------------------------------------------------------------
// "Weakly reduced" arithmetic for pseudo-Mersenne moduli `p = 2^256 - c` with a small `c`
// (`c = T::neg_modulus()`).
//
// Every delegation call costs a few cycles on the guest and a row in the delegation circuit, so
// keeping each intermediate value canonical (`< p`) - an extra `sub p` / `add p` pair per
// operation - is a large part of the cost of a field operation. The functions below instead keep
// values as *any* 256-bit representative of the residue class: inputs and outputs are in
// `[0, 2^256)`. Since `2p > 2^256`, there are at most two representatives of a class, and zero
// is represented by either `0` or `p` (which is what `is_zero_mod` checks).
//
// A carry (borrow) out of 256 bits is worth `2^256 = c (mod p)`, so it is folded back by
// adding (subtracting) `c`. Use `normalize_weak` to get the canonical representative.
//
// All of them require `c^2 + c < 2^256`.
// ---------------------------------------------------------------------------------------------

#[inline(always)]
/// Copies `b` into `a`
pub fn copy_assign(a: &mut U256, b: &U256) {
    delegation::memcpy(a, b);
}

/// Initializes the (32-byte aligned) memory at `dst` with a copy of `src`
/// # Safety
/// `dst` must be valid for writes of a `U256` and 32-byte aligned
#[inline(always)]
pub unsafe fn init_copy(dst: *mut U256, src: &U256) {
    delegation::memcpy_to_ptr(dst, src);
}

#[inline(always)]
/// Folds the carry out of an addition back into `a`
unsafe fn fold_carry_weak<T: DelegatedBarretParams<4>>(a: &mut U256, carry: bool) {
    if carry {
        // `a` wrapped around, so the second carry can only happen if both operands of the
        // addition were non-canonical. After it `a < c`, so the third addition can not overflow.
        let (carry, a) = delegation::add_chained(a, T::neg_modulus());
        if carry != 0 {
            delegation::add(a, T::neg_modulus());
        }
    }
}

#[inline(always)]
/// Folds the borrow out of a subtraction back into `a`
unsafe fn fold_borrow_weak<T: DelegatedBarretParams<4>>(a: &mut U256, borrow: bool) {
    if borrow {
        // Second borrow happens only if `a < c`, after it `a >= 2^256 - c > c`
        let (borrow, a) = delegation::sub_chained(a, T::neg_modulus());
        if borrow != 0 {
            delegation::sub(a, T::neg_modulus());
        }
    }
}

#[inline(always)]
/// Computes `a = a + b mod modulus`, weakly reduced
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn add_mod_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256, b: &U256) {
    let (carry, a) = delegation::add_chained(a, b);
    fold_carry_weak::<T>(a, carry != 0);
}

#[inline(always)]
/// Computes `a = a - b mod modulus`, weakly reduced
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn sub_mod_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256, b: &U256) {
    let (borrow, a) = delegation::sub_chained(a, b);
    fold_borrow_weak::<T>(a, borrow != 0);
}

#[inline(always)]
/// Computes `a = b - a mod modulus`, weakly reduced
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn sub_and_negate_mod_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256, b: &U256) {
    let (borrow, a) = delegation::sub_and_negate_chained(a, b);
    fold_borrow_weak::<T>(a, borrow != 0);
}

#[inline(always)]
/// Computes `a = -a mod modulus`, weakly reduced. Zero can become `p`.
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn neg_mod_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256) {
    // borrow happens only for non-canonical `a`
    let (borrow, a) = delegation::sub_and_negate_chained(a, T::modulus());
    fold_borrow_weak::<T>(a, borrow != 0);
}

#[inline(always)]
/// Computes `a = 2 * a mod modulus`, weakly reduced
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn double_mod_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256) {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);
        let (carry, a) = delegation::add_chained(a, &s.copy_place_0);
        fold_carry_weak::<T>(a, carry != 0);
    })
}

#[inline(always)]
/// Computes `a = 3 * a mod modulus`, weakly reduced
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn triple_mod_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256) {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);
        let (carry, a) = delegation::add_chained(a, &s.copy_place_0);
        fold_carry_weak::<T>(a, carry != 0);
        let (carry, a) = delegation::add_chained(a, &s.copy_place_0);
        fold_carry_weak::<T>(a, carry != 0);
    })
}

#[inline(always)]
/// Computes `a = a * b mod modulus`, weakly reduced. Inputs can be any 256-bit integers.
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn mul_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256, b: &U256) {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_1, a);

        // a * b = hi * 2^256 + lo
        delegation::mul_low(a, b);
        delegation::mul_high(&mut s.copy_place_1, b);

        delegation::memcpy(&mut s.copy_place_2, &s.copy_place_1);

        // hi * 2^256 = hi * c = t_hi * 2^256 + t_lo, where t_hi < c
        delegation::mul_low(&mut s.copy_place_2, T::neg_modulus());
        delegation::mul_high(&mut s.copy_place_1, T::neg_modulus());

        let carry = delegation::add(a, &s.copy_place_2) != 0;
        if carry {
            delegation::add(&mut s.copy_place_1, &ONE);
        }

        // (t_hi + carry) * 2^256 = (t_hi + carry) * c <= c^2, so there is no high part
        delegation::mul_low(&mut s.copy_place_1, T::neg_modulus());

        let carry = delegation::add(a, &s.copy_place_1) != 0;
        if carry {
            // `a` wrapped around and is `< c^2`, so the addition below can not overflow
            delegation::add(a, T::neg_modulus());
        }
    })
}

#[inline(always)]
/// Computes `a = a^2 mod modulus`, weakly reduced
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn square_assign_weak<T: DelegatedBarretParams<4>>(a: &mut U256) {
    with_scratch!(s => {
        delegation::memcpy(&mut s.copy_place_0, a);

        mul_assign_weak::<T>(a, &s.copy_place_0);
    })
}

/// `a != 0` for values that are zero only in degenerate cases (the low half of a product): the
/// lowest word decides (see `low_word`), and the rest is looked at out of line, in software,
/// otherwise the compiler prefers to load and test all the words at once. `is_zero` is for the
/// values that can well be zero: its slow path is one delegated comparison.
#[inline(always)]
pub(super) fn is_non_zero_vartime(a: &U256) -> bool {
    #[cold]
    #[inline(never)]
    fn is_non_zero_slow(a: &U256) -> bool {
        a.0.iter().any(|limb| *limb != 0)
    }

    if low_word(a) != 0 {
        true
    } else {
        is_non_zero_slow(a)
    }
}

#[inline(always)]
/// Montgomery multiplication `a = a * b / 2^256 mod modulus`, weakly reduced. Inputs can be any
/// 256-bit integers. Also requires `DelegatedBarretParams` to fold the carry out.
///
/// `a * b = t_hi * 2^256 + t_lo`, and `m = t_lo * (-1/modulus) mod 2^256` makes `t_lo + low(m * modulus)`
/// zero modulo 2^256. Both terms are below 2^256, so the sum is either 0 (only if `t_lo == 0`, as
/// then `m == 0`), or exactly 2^256. So `low(m * modulus)` is never computed, and the result is
/// `t_hi + high(m * modulus) + (t_lo != 0)`: 4 multiplications, a copy and an addition.
/// # Safety
/// `DelegatedMontParams` and `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn mul_assign_montgomery_weak<T: DelegatedMontParams<4> + DelegatedBarretParams<4>>(
    a: &mut U256,
    b: &U256,
) {
    with_scratch!(s => {
        let a = delegation::widening_mul(a, &mut s.copy_place_1, b);

        montgomery_reduce_weak::<T>(a, &s.copy_place_1);
    })
}

#[inline(always)]
/// `a = (high * 2^256 + a) / 2^256 mod modulus`, weakly reduced
unsafe fn montgomery_reduce_weak<T: DelegatedMontParams<4> + DelegatedBarretParams<4>>(
    a: &mut U256,
    high: &U256,
) {
    let low_part_carry = is_non_zero_vartime(a);

    let a = delegation::mul_low_chained(a, T::reduction_const());
    let a = delegation::mul_high_chained(a, T::modulus());

    // t_hi < 2^256 and high(m * modulus) < modulus, so after the carry out `a < modulus`,
    // and the addition of `2^256 mod modulus` can not overflow
    let (carry, a) = delegation::add_with_carry_bit_chained(a, high, low_part_carry);
    if carry != 0 {
        delegation::add(a, T::neg_modulus());
    }
}

#[inline(always)]
/// Montgomery squaring, weakly reduced
/// # Safety
/// `DelegatedMontParams` and `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn square_assign_montgomery_weak<
    T: DelegatedMontParams<4> + DelegatedBarretParams<4>,
>(
    a: &mut U256,
) {
    with_scratch!(s => {
        let a = delegation::widening_square(a, &mut s.copy_place_1, &mut s.copy_place_0);

        montgomery_reduce_weak::<T>(a, &s.copy_place_1);
    })
}

#[inline(always)]
/// Brings a weakly reduced `a` into `[0, modulus)`
/// # Safety
/// `DelegatedBarretParams` should only provide references to statics.
pub unsafe fn normalize_weak<T: DelegatedModParams<4>>(a: &mut U256) {
    // a < 2^256 < 2p, so single subtraction is enough
    sub_mod_with_carry::<T>(a, false);
}

#[cfg(test)]
#[derive(Debug)]
pub struct U256Wrapper<T: DelegatedModParams<4>>(pub U256, PhantomData<T>);

#[cfg(test)]
impl<T: DelegatedModParams<4> + Debug> proptest::arbitrary::Arbitrary for U256Wrapper<T> {
    type Parameters = T;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::{any, Just, Strategy};

        any::<[u64; 4]>().prop_map(|words| {
            let mut res = BigInt::<4>(words);
            unsafe {
                sub_mod_with_carry::<Self::Parameters>(&mut res, false);
                sub_mod_with_carry::<Self::Parameters>(&mut res, false);
            }
            Self(res, PhantomData::default())
        })
    }

    type Strategy = proptest::arbitrary::Mapped<[u64; 4], Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark_ff_delegation::BigInt;

    use ark_ff::{BigInt as BigIntRef, BigInteger};
    use proptest::{prop_assert, prop_assert_eq, proptest};

    #[derive(Default, Debug)]
    struct ZeroMod;

    impl DelegatedModParams<4> for ZeroMod {
        const MODULUS_BITSIZE: usize = 0;

        fn modulus() -> &'static BigInt<4> {
            &ZERO
        }
    }

    #[test]
    fn test_mul_wide() {
        proptest!(|(x: U256Wrapper<ZeroMod>, y: U256Wrapper<ZeroMod>)| {
            let (x, y) = (x.0, y.0);
            let x_ref = BigIntRef::new(x.0);
            let y_ref = BigIntRef::new(y.0);

            let (ref_low, ref_high) = x_ref.mul(&y_ref);
            let (low, high) = mul_wide(&x, &y);

            prop_assert_eq!(low.0, ref_low.0);
            prop_assert_eq!(high.0, ref_high.0);

        })
    }

    /// The secp256k1 prime, for the zero tests
    #[derive(Default, Debug)]
    struct OddMod;

    static ODD_MODULUS: U256 = BigInt([0xFFFFFFFEFFFFFC2F, u64::MAX, u64::MAX, u64::MAX]);

    impl DelegatedModParams<4> for OddMod {
        const MODULUS_BITSIZE: usize = 256;

        fn modulus() -> &'static BigInt<4> {
            &ODD_MODULUS
        }
    }

    fn delegation_calls() -> u64 {
        delegation::DELEGATION_CALLS.with(|c| c.get())
    }

    /// `a` with its lowest word replaced
    fn with_low_word(mut a: U256, word: u32) -> U256 {
        a.0[0] = (a.0[0] & !u64::from(u32::MAX)) | u64::from(word);
        a
    }

    /// The zero tests agree with the plain comparison, and compare with the delegation only
    /// the integers that their lowest word does not tell apart: those whose lowest word is the
    /// one of zero, or of the modulus
    #[test]
    fn zero_tests() {
        let modulus = ODD_MODULUS;
        proptest!(|(x: [u64; 4])| {
            let x = BigInt(x);
            let lookalikes = [with_low_word(x, 0), with_low_word(x, low_word(&modulus))];
            for a in [x, U256::zero(), modulus].into_iter().chain(lookalikes) {
                let zero = a.0 == [0; 4];

                let calls = delegation_calls();
                prop_assert_eq!(is_zero(&a), zero);
                prop_assert_eq!(delegation_calls() - calls, u64::from(low_word(&a) == 0));

                let calls = delegation_calls();
                prop_assert_eq!(unsafe { is_zero_mod::<OddMod>(&a) }, zero || a.0 == modulus.0);
                let compared = low_word(&a) == 0 || low_word(&a) == low_word(&modulus);
                prop_assert_eq!(delegation_calls() - calls, u64::from(compared));

                prop_assert_eq!(is_non_zero_vartime(&a), !zero);
            }
        })
    }

    /// `-a` is `modulus - a`, and zero for zero, also when only the lowest word of `a` is zero
    #[test]
    fn neg_mod_zero_lowest_word() {
        proptest!(|(x: [u64; 4])| {
            let mut x = BigInt(x);
            // below the modulus
            x.0[3] >>= 1;
            for a in [x, with_low_word(x, 0), U256::zero()] {
                let mut negated = a;
                unsafe { neg_mod_assign::<OddMod>(&mut negated) };
                let mut sum = BigIntRef::new(negated.0);
                prop_assert!(!sum.add_with_carry(&BigIntRef::new(a.0)));
                let expected = if a.0 == [0; 4] { [0; 4] } else { ODD_MODULUS.0 };
                prop_assert_eq!(sum.0, expected);
            }
        })
    }
}
