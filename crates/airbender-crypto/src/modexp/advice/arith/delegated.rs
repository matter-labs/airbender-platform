//! The arithmetic of the multi-digit path over the bigint delegation: 256-bit digits, aligned
//! for it, and operations that are sequences of delegation calls (emulated off RV32). The
//! guest's costs are those sequences, which the tests of `modexp` pin.

use crate::{bigint_op_delegation_raw, bigint_op_delegation_with_carry_bit_raw, BigIntOps};
use core::mem::MaybeUninit;

/// The digit of the arithmetic
pub(crate) type Digit = DelegatedU256;

/// Bytes of a digit
pub(crate) const DIGIT_BYTES: usize = 32;

/// The digit 1
pub(crate) const DIGIT_ONE: Digit = DelegatedU256::ONE;

/// The working space of the products
pub(crate) type Scratch = ProductScratch;

// Immutable statics land in `.rodata`, above the ROM bound, so they are valid delegation operands
static ZERO: DelegatedU256 = DelegatedU256::ZERO;
static ONE: DelegatedU256 = DelegatedU256::ONE;

/// A 256-bit digit of the multi-digit path, aligned for the delegation
#[repr(align(32))]
pub(crate) struct DelegatedU256([u64; 4]);

impl core::fmt::Debug for DelegatedU256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x")?;
        for digit in self.0.iter().rev() {
            write!(f, "{digit:016x}")?;
        }

        Ok(())
    }
}

impl DelegatedU256 {
    pub(crate) const ZERO: Self = Self([0; 4]);
    pub(crate) const ONE: Self = Self([1, 0, 0, 0]);

    #[inline(always)]
    unsafe fn copy_from_ptr(source: *const Self) -> Self {
        let mut result = MaybeUninit::<Self>::uninit();
        let _ =
            bigint_op_delegation_raw(result.as_mut_ptr().cast(), source.cast(), BigIntOps::MemCpy);

        unsafe { result.assume_init() }
    }

    pub(crate) fn zero() -> Self {
        unsafe {
            #[allow(static_mut_refs)]
            Self::copy_from_ptr(core::ptr::addr_of!(ZERO))
        }
    }

    /// The zero digit as a delegation operand (it lives in `.rodata`)
    pub(crate) fn zero_ptr() -> *const Self {
        core::ptr::addr_of!(ZERO)
    }

    /// Writes zero into `dst`
    ///
    /// # Safety
    /// `dst` must be 32 bytes aligned and point to 32 bytes of accessible memory.
    pub(crate) unsafe fn write_zero_into_ptr(dst: *mut Self) {
        let _ = bigint_op_delegation_raw(dst.cast(), Self::zero_ptr().cast(), BigIntOps::MemCpy);
    }

    #[cfg(target_arch = "riscv32")]
    pub(crate) unsafe fn from_be_bytes_in_place(input: &[u8; 32], place: &mut MaybeUninit<Self>) {
        unsafe {
            // `Self` is 32-byte aligned and `place` is disjoint from `input`.
            crate::byte_order::read_be_into_words::<32>(input.as_ptr(), place.as_mut_ptr().cast());
        }
    }

    #[cfg(not(target_arch = "riscv32"))]
    pub(crate) unsafe fn from_be_bytes_in_place(input: &[u8; 32], place: &mut MaybeUninit<Self>) {
        unsafe {
            let ptr = place.as_mut_ptr().cast::<u64>();
            let src: *const [u8; 8] = input.as_ptr_range().end.cast();

            ptr.write(u64::from_be_bytes(src.sub(1).read()));
            ptr.add(1).write(u64::from_be_bytes(src.sub(2).read()));
            ptr.add(2).write(u64::from_be_bytes(src.sub(3).read()));
            ptr.add(3).write(u64::from_be_bytes(src.sub(4).read()));
        }
    }

    pub(crate) fn to_be_bytes(&self) -> [u8; 32] {
        let mut res = self.clone();
        res.bytereverse();
        unsafe { core::mem::transmute(res) }
    }

    #[cfg(not(target_arch = "riscv32"))]
    pub(crate) const fn as_limbs_mut(&mut self) -> &mut [u64; 4] {
        &mut self.0
    }

    #[cfg(target_arch = "riscv32")]
    pub(crate) fn bytereverse(&mut self) {
        unsafe {
            // `Self` is 32-byte aligned and contains eight initialized words.
            crate::byte_order::bytereverse_words_in_place::<8>(self.0.as_mut_ptr().cast());
        }
    }

    #[cfg(not(target_arch = "riscv32"))]
    pub(crate) fn bytereverse(&mut self) {
        let limbs = self.as_limbs_mut();
        unsafe {
            core::ptr::swap(&mut limbs[0] as *mut u64, &mut limbs[3] as *mut u64);
            core::ptr::swap(&mut limbs[1] as *mut u64, &mut limbs[2] as *mut u64);
        }
        for limb in limbs.iter_mut() {
            *limb = limb.swap_bytes();
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        #[allow(static_mut_refs)]
        unsafe {
            // equality is non-destructive, so we can cast
            let eq = bigint_op_delegation_raw(
                (self as *const Self).cast_mut().cast(),
                core::ptr::addr_of!(ZERO).cast(),
                BigIntOps::Eq,
            );

            eq != 0
        }
    }

    pub(crate) fn is_one(&self) -> bool {
        #[allow(static_mut_refs)]
        unsafe {
            // equality is non-destructive, so we can cast
            let eq = bigint_op_delegation_raw(
                (self as *const Self).cast_mut().cast(),
                core::ptr::addr_of!(ONE).cast(),
                BigIntOps::Eq,
            );

            eq != 0
        }
    }
}

impl Clone for DelegatedU256 {
    #[inline(always)]
    fn clone(&self) -> Self {
        // one delegated copy
        unsafe { Self::copy_from_ptr(self as *const Self) }
    }

    #[inline(always)]
    fn clone_from(&mut self, source: &Self) {
        unsafe {
            let _ = bigint_op_delegation_raw(
                self.0.as_mut_ptr().cast(),
                (source as *const Self).cast(),
                BigIntOps::MemCpy,
            );
        }
    }
}

impl PartialEq for DelegatedU256 {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            // equality is non-destructive, so we can cast
            let eq = bigint_op_delegation_raw(
                (self as *const Self).cast_mut().cast(),
                (other as *const Self).cast(),
                BigIntOps::Eq,
            );

            eq != 0
        }
    }
}

impl Eq for DelegatedU256 {}

/// # Safety
/// `src` must be allocated in non ROM.
/// `dst` must be 32 bytes aligned and point to 32 bytes of accessible memory.
pub(crate) unsafe fn write_into_ptr_unchecked(dst: *mut DelegatedU256, source: &DelegatedU256) {
    unsafe {
        bigint_op_delegation_raw(
            dst.cast(),
            (source as *const DelegatedU256).cast(),
            BigIntOps::MemCpy,
        );
    }
}

/// Writes the digit of the big-endian `input` into `place`
///
/// # Safety
/// `place` must be disjoint from `input`.
#[inline(always)]
pub(crate) unsafe fn from_be_bytes_in_place(
    input: &[u8; DIGIT_BYTES],
    place: &mut MaybeUninit<Digit>,
) {
    // SAFETY: guaranteed by the caller
    unsafe { DelegatedU256::from_be_bytes_in_place(input, place) }
}

/// The big-endian bytes of `digit`
#[inline(always)]
pub(crate) fn to_be_bytes(digit: &Digit) -> [u8; DIGIT_BYTES] {
    digit.to_be_bytes()
}

#[inline(always)]
pub(crate) fn is_zero(digit: &Digit) -> bool {
    digit.is_zero()
}

#[inline(always)]
pub(crate) fn is_one(digit: &Digit) -> bool {
    digit.is_one()
}

/// Copies `src` into the first slots of `dst`, a delegated copy each
pub(crate) fn copy(dst: &mut [MaybeUninit<Digit>], src: &[Digit]) {
    assert!(dst.len() >= src.len());
    for (dst, src) in dst.iter_mut().zip(src.iter()) {
        // SAFETY: an aligned slot, and an initialized digit outside ROM
        unsafe { write_into_ptr_unchecked(dst.as_mut_ptr(), src) };
    }
}

/// Writes zero into every slot of `out`, and returns them as digits
fn fill_zero(out: &mut [MaybeUninit<Digit>]) -> &mut [Digit] {
    for slot in out.iter_mut() {
        // SAFETY: an aligned slot
        unsafe { DelegatedU256::write_zero_into_ptr(slot.as_mut_ptr()) };
    }
    // SAFETY: every slot was just written
    unsafe { core::slice::from_raw_parts_mut(out.as_mut_ptr().cast(), out.len()) }
}

/// `digits[k..] += lo + hi * 2^256` (a 512-bit value at digit position `k`), with carry
/// propagation. Returns whether the sum fits in `digits`; it never writes outside them, and
/// their content is unspecified when it does not fit.
///
/// # Safety
/// `lo` and `hi` must point to aligned, initialized digits outside `digits`.
#[must_use]
unsafe fn add_wide_at(
    digits: &mut [DelegatedU256],
    k: usize,
    lo: *const DelegatedU256,
    hi: *const DelegatedU256,
) -> bool {
    let len = digits.len();
    if k >= len {
        return false;
    }
    let base = digits.as_mut_ptr();
    let mut carry = bigint_op_delegation_raw(base.add(k).cast(), lo.cast(), BigIntOps::Add) != 0;
    let mut j = k + 1;
    if j < len {
        carry = bigint_op_delegation_with_carry_bit_raw(
            base.add(j).cast(),
            hi.cast(),
            carry,
            BigIntOps::Add,
        ) != 0;
        j += 1;
    } else if carry || !(*hi).is_zero() {
        return false;
    }
    while carry {
        if j >= len {
            return false;
        }
        carry = bigint_op_delegation_with_carry_bit_raw(
            base.add(j).cast(),
            DelegatedU256::zero_ptr().cast(),
            true,
            BigIntOps::Add,
        ) != 0;
        j += 1;
    }
    true
}

/// `out = a * b` (schoolbook) for non-empty `a` and `b`, in the `a.len() + b.len()` digits of
/// `out`, all written
pub(crate) fn mul(out: &mut [MaybeUninit<Digit>], a: &[Digit], b: &[Digit], scratch: &mut Scratch) {
    assert!(out.len() == a.len() + b.len());
    let out = fill_zero(out);
    for (j, b_digit) in b.iter().enumerate() {
        for (i, a_digit) in a.iter().enumerate() {
            // SAFETY: the scratch slots are distinct from `out`
            unsafe {
                scratch.product(a_digit, b_digit);
                let fits = add_wide_at(out, i + j, &scratch.lo, &scratch.hi);
                // a product of `a.len()` and `b.len()` digits, whatever the digits are
                assert!(fits, "the product does not fit");
            }
        }
    }
}

/// `out = a * a` for a non-empty `a`, in the `2 * a.len()` digits of `out`, all written: each
/// cross product `a_i a_j` (`i < j`) is computed once and added twice, `n (n + 1) / 2` digit
/// products instead of `n^2`
pub(crate) fn square(out: &mut [MaybeUninit<Digit>], a: &[Digit], scratch: &mut Scratch) {
    assert!(out.len() == 2 * a.len());
    let out = fill_zero(out);
    for (i, a_i) in a.iter().enumerate() {
        for (j, a_j) in a.iter().enumerate().skip(i + 1) {
            // SAFETY: the scratch slots are distinct from `out`
            unsafe {
                scratch.product(a_i, a_j);
                // a square of `a.len()` digits, whatever the digits are
                let fits = add_wide_at(out, i + j, &scratch.lo, &scratch.hi)
                    && add_wide_at(out, i + j, &scratch.lo, &scratch.hi);
                assert!(fits, "the product does not fit");
            }
        }
    }
    for (i, a_i) in a.iter().enumerate() {
        // SAFETY: as above
        unsafe {
            scratch.product(a_i, a_i);
            let fits = add_wide_at(out, 2 * i, &scratch.lo, &scratch.hi);
            assert!(fits, "the product does not fit");
        }
    }
}

/// `out = q * m + r` in exactly the digits of `out`. Returns whether the value fits, so that a
/// hint reconstructing a truncated value cannot pass the comparison with the product; when it
/// does, all of `out` is written. `q`, `m` and `r` may have any content and any number of digits
/// (they are untrusted): nothing is written outside `out`.
#[must_use]
pub(crate) fn mul_add(
    out: &mut [MaybeUninit<Digit>],
    q: &[Digit],
    m: &[Digit],
    r: &[Digit],
    scratch: &mut Scratch,
) -> bool {
    if r.len() > out.len() {
        return false;
    }
    let out = fill_zero(out);
    for (slot, r_digit) in out.iter_mut().zip(r) {
        // SAFETY: `r` is a distinct, initialized value
        unsafe { write_into_ptr_unchecked(slot, r_digit) };
    }
    for (j, m_digit) in m.iter().enumerate() {
        for (i, q_digit) in q.iter().enumerate() {
            // SAFETY: the scratch slots are distinct from `out`
            unsafe {
                scratch.product(q_digit, m_digit);
                if !add_wide_at(out, i + j, &scratch.lo, &scratch.hi) {
                    return false;
                }
            }
        }
    }
    true
}

/// Whether `a` and `b` are the same integer (zero top digits do not matter)
#[must_use]
pub(crate) fn eq_value(a: &[Digit], b: &[Digit]) -> bool {
    let meaningful_digits_floor = core::cmp::min(a.len(), b.len());
    for (a_digit, b_digit) in a.iter().zip(b.iter()) {
        if !a_digit.eq(b_digit) {
            return false;
        }
    }
    for input in [a, b] {
        if input.len() > meaningful_digits_floor {
            for el in input[meaningful_digits_floor..].iter() {
                if !el.is_zero() {
                    return false;
                }
            }
        }
    }
    true
}

/// Whether `a < m`, for strings of one length. `m` is scratch: `a - m`, which must borrow, is
/// written into it (`SubAndNegate` keeps `a`).
#[must_use]
pub(crate) fn is_below(a: &[Digit], m: &mut [Digit]) -> bool {
    assert!(a.len() == m.len());
    let mut borrow = 0;
    for (m_digit, a_digit) in m.iter_mut().zip(a.iter()) {
        // SAFETY: aligned, initialized, distinct digits
        borrow = unsafe {
            bigint_op_delegation_with_carry_bit_raw(
                (m_digit as *mut DelegatedU256).cast(),
                (a_digit as *const DelegatedU256).cast(),
                borrow > 0,
                BigIntOps::SubAndNegate,
            )
        };
    }
    borrow > 0
}

/// `digits` digits, in the 256-bit digits of the advice request: themselves
#[inline(always)]
pub(crate) const fn protocol_digits(digits: usize) -> usize {
    digits
}

/// The digits after a value that complete its last 256-bit digit of the advice request: none
#[inline(always)]
pub(crate) fn pad_for_query(_spare: &mut [MaybeUninit<Digit>], _digits: usize) {}

/// Two aligned slots holding the 512-bit product of two digits
pub(crate) struct ProductScratch {
    lo: DelegatedU256,
    hi: DelegatedU256,
}

impl ProductScratch {
    pub(crate) fn new() -> Self {
        Self {
            lo: DelegatedU256::zero(),
            hi: DelegatedU256::zero(),
        }
    }

    /// `(lo, hi) = a * b`
    ///
    /// # Safety
    /// `a` and `b` must be initialized digits outside ROM.
    #[inline(always)]
    unsafe fn product(&mut self, a: &DelegatedU256, b: &DelegatedU256) {
        write_into_ptr_unchecked(&mut self.lo, a);
        write_into_ptr_unchecked(&mut self.hi, a);
        let b = (b as *const DelegatedU256).cast();
        let _ = bigint_op_delegation_raw(
            (&mut self.lo as *mut DelegatedU256).cast(),
            b,
            BigIntOps::MulLow,
        );
        let _ = bigint_op_delegation_raw(
            (&mut self.hi as *mut DelegatedU256).cast(),
            b,
            BigIntOps::MulHigh,
        );
    }
}
