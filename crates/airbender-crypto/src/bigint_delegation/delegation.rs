use super::u256::U256;
use crate::BigIntOps;

#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
const CARRY_BIT_IDX: usize = 6;

#[inline(always)]
pub(super) fn add(a: &mut U256, b: &U256) -> u32 {
    bigint_op_delegation(a, b, BigIntOps::Add)
}

#[inline(always)]
pub(super) fn sub(a: &mut U256, b: &U256) -> u32 {
    bigint_op_delegation(a, b, BigIntOps::Sub)
}

#[inline(always)]
pub(super) fn sub_and_negate(a: &mut U256, b: &U256) -> u32 {
    bigint_op_delegation(a, b, BigIntOps::SubAndNegate)
}

#[inline(always)]
pub(super) fn mul_low(a: &mut U256, b: &U256) {
    bigint_op_delegation(a, b, BigIntOps::MulLow);
}

#[inline(always)]
pub(super) fn mul_high(a: &mut U256, b: &U256) {
    bigint_op_delegation(a, b, BigIntOps::MulHigh);
}

#[inline(always)]
pub(super) fn eq(a: &U256, b: &U256) -> u32 {
    let a = a as *const _ as *mut _;
    bigint_op_delegation(a, b, BigIntOps::Eq)
}

#[inline(always)]
pub(super) fn memcpy(a: &mut U256, b: &U256) {
    bigint_op_delegation(a, b, BigIntOps::MemCpy);
}

/// `memcpy` into possibly uninitialized, 32-byte aligned memory
#[inline(always)]
pub(super) fn memcpy_to_ptr(dst: *mut U256, src: &U256) {
    bigint_op_delegation(dst, src, BigIntOps::MemCpy);
}

#[inline(always)]
pub(super) fn sub_with_carry_bit(a: &mut U256, b: &U256, carry: bool) -> u32 {
    bigint_op_delegation_with_carry_bit(a, b, carry, BigIntOps::Sub)
}

#[inline(always)]
pub(super) fn add_with_carry_bit(a: &mut U256, b: &U256, carry: bool) -> u32 {
    bigint_op_delegation_with_carry_bit(a, b, carry, BigIntOps::Add)
}

#[inline(always)]
pub(super) fn sub_and_negate_with_carry_bit(a: &mut U256, b: &U256, carry: bool) -> u32 {
    bigint_op_delegation_with_carry_bit(a, b, carry, BigIntOps::SubAndNegate)
}

// Every call takes pointers in x10 and x11, and they survive it. The compiler knows that (they are
// `in` operands), but the pointer lives in some other register too, and after a branch, or another
// block with its own idea about x10, it is moved into x10 again. So:
// - the "chained" calls give the pointer to `a` back as the value x10 has after the call. It is
//   the same pointer, but for the compiler it is a value that is born in x10, so the next call
//   on it finds it there.
// - the sequences that every multiplication starts with are single blocks, that take their first
//   operands in x10/x11 (so they can be produced right there), change them by hand, and also
//   end with `a` in x10.

#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
#[inline(always)]
fn bigint_op_delegation_chained<'a>(
    a: &'a mut U256,
    b: &U256,
    carry: bool,
    op: BigIntOps,
) -> (u32, &'a mut U256) {
    let a: *mut U256 = a;
    debug_assert!(a.cast_const() != b as *const U256);
    debug_assert!(a.addr() % 32 == 0);
    debug_assert!((b as *const U256).addr() % 32 == 0);

    let mut mask = (1u32 << (op as usize)) | ((carry as u32) << CARRY_BIT_IDX);
    let a_after: *mut U256;

    unsafe {
        core::arch::asm!(
            "csrrw x0, 0x7ca, x0",
            inout("x10") a => a_after,
            in("x11") b as *const U256,
            inlateout("x12") mask,
            options(nostack, preserves_flags)
        );

        // SAFETY: the call does not change x10, so that's `a`
        (mask, &mut *a_after)
    }
}

#[cfg(not(all(target_arch = "riscv32", feature = "bigint_ops")))]
#[inline(always)]
fn bigint_op_delegation_chained<'a>(
    a: &'a mut U256,
    b: &U256,
    carry: bool,
    op: BigIntOps,
) -> (u32, &'a mut U256) {
    (bigint_op_delegation_with_carry_bit(a, b, carry, op), a)
}

#[inline(always)]
pub(super) fn add_chained<'a>(a: &'a mut U256, b: &U256) -> (u32, &'a mut U256) {
    bigint_op_delegation_chained(a, b, false, BigIntOps::Add)
}

#[inline(always)]
pub(super) fn add_with_carry_bit_chained<'a>(
    a: &'a mut U256,
    b: &U256,
    carry: bool,
) -> (u32, &'a mut U256) {
    bigint_op_delegation_chained(a, b, carry, BigIntOps::Add)
}

#[inline(always)]
pub(super) fn sub_chained<'a>(a: &'a mut U256, b: &U256) -> (u32, &'a mut U256) {
    bigint_op_delegation_chained(a, b, false, BigIntOps::Sub)
}

#[inline(always)]
pub(super) fn sub_and_negate_chained<'a>(a: &'a mut U256, b: &U256) -> (u32, &'a mut U256) {
    bigint_op_delegation_chained(a, b, false, BigIntOps::SubAndNegate)
}

#[inline(always)]
pub(super) fn mul_low_chained<'a>(a: &'a mut U256, b: &U256) -> &'a mut U256 {
    bigint_op_delegation_chained(a, b, false, BigIntOps::MulLow).1
}

#[inline(always)]
pub(super) fn mul_high_chained<'a>(a: &'a mut U256, b: &U256) -> &'a mut U256 {
    bigint_op_delegation_chained(a, b, false, BigIntOps::MulHigh).1
}

/// `a * b = high * 2^256 + low`: `a` becomes `low`, and `high` (its content is ignored) is `high`.
/// The same as `memcpy(high, a); mul_high(high, b); mul_low(a, b)`. Gives `a` back (see above).
#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
#[inline(always)]
pub(super) fn widening_mul<'a>(a: &'a mut U256, high: &mut U256, b: &U256) -> &'a mut U256 {
    let a: *mut U256 = a;
    debug_assert!(a.addr() % 32 == 0);
    debug_assert!((high as *mut U256).addr() % 32 == 0);
    debug_assert!((b as *const U256).addr() % 32 == 0);

    let a_after: *mut U256;

    unsafe {
        core::arch::asm!(
            // high = a: x10 = high, x11 = a
            "li x12, {memcpy}",
            "csrrw x0, 0x7ca, x0",
            // high = high(high * b), x10 stays
            "mv x11, {b}",
            "li x12, {mul_high}",
            "csrrw x0, 0x7ca, x0",
            // a = low(a * b), x11 stays
            "mv x10, {a}",
            "li x12, {mul_low}",
            "csrrw x0, 0x7ca, x0",
            inout("x10") high as *mut U256 => a_after,
            inout("x11") a => _,
            a = in(reg) a,
            b = in(reg) b as *const U256,
            memcpy = const 1 << (BigIntOps::MemCpy as usize),
            mul_high = const 1 << (BigIntOps::MulHigh as usize),
            mul_low = const 1 << (BigIntOps::MulLow as usize),
            out("x12") _,
            options(nostack, preserves_flags)
        );

        // SAFETY: x10 is `a` at the end
        &mut *a_after
    }
}

#[cfg(not(all(target_arch = "riscv32", feature = "bigint_ops")))]
#[inline(always)]
pub(super) fn widening_mul<'a>(a: &'a mut U256, high: &mut U256, b: &U256) -> &'a mut U256 {
    memcpy(high, a);
    mul_high(high, b);
    mul_low(a, b);
    a
}

/// `a * a = high * 2^256 + low`: `a` becomes `low`, and `high` (its content is ignored) is `high`.
/// A call can not have the same input twice, so it needs a place for a copy of `a`.
/// Gives `a` back (see above).
#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
#[inline(always)]
pub(super) fn widening_square<'a>(
    a: &'a mut U256,
    high: &mut U256,
    copy: &mut U256,
) -> &'a mut U256 {
    let a: *mut U256 = a;
    debug_assert!(a.addr() % 32 == 0);
    debug_assert!((high as *mut U256).addr() % 32 == 0);
    debug_assert!((copy as *mut U256).addr() % 32 == 0);

    let a_after: *mut U256;

    unsafe {
        core::arch::asm!(
            // copy = a: x10 = copy, x11 = a
            "li x12, {memcpy}",
            "csrrw x0, 0x7ca, x0",
            // high = a, x11 stays
            "mv x10, {high}",
            "li x12, {memcpy}",
            "csrrw x0, 0x7ca, x0",
            // high = high(high * copy), x10 stays
            "mv x11, {copy}",
            "li x12, {mul_high}",
            "csrrw x0, 0x7ca, x0",
            // a = low(a * copy), x11 stays
            "mv x10, {a}",
            "li x12, {mul_low}",
            "csrrw x0, 0x7ca, x0",
            inout("x10") copy as *mut U256 => a_after,
            inout("x11") a => _,
            a = in(reg) a,
            high = in(reg) high as *mut U256,
            copy = in(reg) copy as *mut U256,
            memcpy = const 1 << (BigIntOps::MemCpy as usize),
            mul_high = const 1 << (BigIntOps::MulHigh as usize),
            mul_low = const 1 << (BigIntOps::MulLow as usize),
            out("x12") _,
            options(nostack, preserves_flags)
        );

        // SAFETY: x10 is `a` at the end
        &mut *a_after
    }
}

#[cfg(not(all(target_arch = "riscv32", feature = "bigint_ops")))]
#[inline(always)]
pub(super) fn widening_square<'a>(
    a: &'a mut U256,
    high: &mut U256,
    copy: &mut U256,
) -> &'a mut U256 {
    memcpy(copy, a);
    memcpy(high, a);
    mul_high(high, copy);
    mul_low(a, copy);
    a
}

#[inline(always)]
fn bigint_op_delegation(a: *mut U256, b: *const U256, op: BigIntOps) -> u32 {
    bigint_op_delegation_with_carry_bit(a, b, false, op)
}

#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
#[inline(always)]
pub(crate) fn bigint_op_delegation_with_carry_bit(
    a: *mut U256,
    b: *const U256,
    carry: bool,
    op: BigIntOps,
) -> u32 {
    debug_assert!(a.cast_const() != b);

    let a_adrr = a.addr();
    let b_adrr = b.addr();

    debug_assert!(a_adrr % 32 == 0);
    debug_assert!(b_adrr % 32 == 0);

    let mut mask = (1u32 << (op as usize)) | ((carry as u32) << CARRY_BIT_IDX);

    unsafe {
        core::arch::asm!(
            "csrrw x0, 0x7ca, x0",
            in("x10") a_adrr,
            in("x11") b_adrr,
            inlateout("x12") mask,
            options(nostack, preserves_flags)
        )
    }

    mask
}

// Number of delegation calls made so far on this thread (host emulation, tests only): the
// cost model of the proving target is the number of these calls.
#[cfg(test)]
thread_local! {
    pub(crate) static DELEGATION_CALLS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// The same, per operation (indexed by `BigIntOps`)
    pub(crate) static DELEGATION_CALLS_BY_OP: core::cell::RefCell<[u64; 8]> = const { core::cell::RefCell::new([0; 8]) };
}

#[cfg(not(all(target_arch = "riscv32", feature = "bigint_ops")))]
#[inline(always)]
pub(crate) fn bigint_op_delegation_with_carry_bit(
    a_ptr: *mut U256,
    b_ptr: *const U256,
    carry: bool,
    op: BigIntOps,
) -> u32 {
    #[cfg(test)]
    DELEGATION_CALLS.with(|c| c.set(c.get() + 1));
    #[cfg(test)]
    DELEGATION_CALLS_BY_OP.with(|c| c.borrow_mut()[op as usize] += 1);
    debug_assert!(a_ptr.cast_const() != b_ptr);
    debug_assert!(a_ptr.addr() % 32 == 0);
    debug_assert!(b_ptr.addr() % 32 == 0);

    unsafe {
        use ruint::aliases::{U256 as rU256, U512 as rU512};

        use core::ptr::addr_of;

        let (a, b) = (
            rU256::from_limbs(addr_of!((*a_ptr).0).read()),
            rU256::from_limbs(addr_of!((*b_ptr).0).read()),
        );

        let carry_or_borrow = rU256::from(carry as u64);

        let result;
        let of = match op {
            BigIntOps::Add => {
                let (t, of0) = a.overflowing_add(b);
                let (t, of1) = t.overflowing_add(carry_or_borrow);
                result = t;

                of0 || of1
            }
            BigIntOps::Sub => {
                let (t, of0) = a.overflowing_sub(b);
                let (t, of1) = t.overflowing_sub(carry_or_borrow);
                result = t;

                of0 || of1
            }
            BigIntOps::SubAndNegate => {
                let (t, of0) = b.overflowing_sub(a);
                let (t, of1) = t.overflowing_sub(carry_or_borrow);
                result = t;

                of0 || of1
            }
            BigIntOps::MulLow => {
                let t: rU512 = a.widening_mul(b);
                result = rU256::from_limbs(t.as_limbs()[..4].try_into().unwrap());

                t.as_limbs()[4..].iter().any(|el| *el != 0)
            }
            BigIntOps::MulHigh => {
                let t: rU512 = a.widening_mul(b);
                result = rU256::from_limbs(t.as_limbs()[4..8].try_into().unwrap());

                false
            }
            BigIntOps::Eq => {
                result = a;

                a == b
            }
            BigIntOps::MemCpy => {
                let (t, of) = b.overflowing_add(carry_or_borrow);
                result = t;

                of
            }
        };

        use core::ptr::addr_of_mut;
        addr_of_mut!((*a_ptr).0).write(*result.as_limbs());

        of as u32
    }
}
