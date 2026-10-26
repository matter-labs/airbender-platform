// Ported from zksync-os 185a0568:supporting_crates/u256/src/risc_v/mod.rs
// Ported from zksync-os 185a0568:supporting_crates/delegated_u256/src/lib.rs
// Ported from zksync-os 185a0568:supporting_crates/delegated_u256/src/delegation.rs
// Ported from zksync-os 185a0568:supporting_crates/delegated_u256/src/copy.rs
// Ported from zksync-os 185a0568:supporting_crates/delegated_u256/src/arithmetic.rs
// Ported from zksync-os 185a0568:supporting_crates/delegated_u256/src/utils.rs
//! The `U256` of the RISC-V target with the bigint delegation (zksync-os `u256::risc_v::U256` over
//! `delegated_u256::DelegatedU256`), with the delegation sequences of zksync-os: a clone is one
//! copy, a test for zero or one is one comparison, `<` is a copy into a scratch, a comparison and
//! a subtraction, a widening multiplication is the low half then the high half. Only the
//! operations that ModExp uses are ported. The delegation is called through
//! [`crate::bigint_op_delegation_with_carry_bit_raw`], which the host emulates (and counts, in tests).

use crate::{bigint_op_delegation_with_carry_bit_raw, BigIntOps};
use core::cmp::Ordering;
use core::mem::MaybeUninit;

// Even though we derive, internally we use delegation circuit for equality, ordering and cloning
// See DelegatedU256 implementations for details
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Debug)]
pub struct U256(DelegatedU256);

impl U256 {
    pub const fn from_limbs(limbs: [u64; 4]) -> Self {
        Self(DelegatedU256::from_limbs(limbs))
    }

    /// # Safety
    /// `dst` must be 32 byte aligned and point to 32 bytes of accessible memory.
    pub unsafe fn write_into_ptr_unchecked(dst: *mut Self, source: &Self) {
        write_into_ptr_unchecked(dst.cast(), &source.0);
    }

    #[inline(always)]
    pub fn zero() -> Self {
        Self::from_limbs([0, 0, 0, 0])
    }

    #[inline(always)]
    pub fn one() -> Self {
        Self::from_limbs([1, 0, 0, 0])
    }

    #[inline(always)]
    pub const fn as_limbs(&self) -> &[u64; 4] {
        self.0.as_limbs()
    }

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[inline(always)]
    pub fn is_one(&self) -> bool {
        self.0.is_one()
    }

    #[inline(always)]
    pub fn overflowing_add_assign(&mut self, rhs: &Self) -> bool {
        self.0.overflowing_add_assign(&rhs.0)
    }

    #[inline(always)]
    pub fn overflowing_sub_assign(&mut self, rhs: &Self) -> bool {
        self.0.overflowing_sub_assign(&rhs.0)
    }

    #[inline(always)]
    pub fn overflowing_sub_assign_with_borrow_propagation(
        &mut self,
        rhs: &Self,
        borrow: bool,
    ) -> bool {
        self.0.overflowing_sub_assign_with_borrow(&rhs.0, borrow)
    }

    #[inline(always)]
    pub fn widening_mul_assign_into(&mut self, high: &mut Self, rhs: &Self) {
        self.0.widening_mul_assign_into(&mut high.0, &rhs.0);
    }

    #[inline(always)]
    pub fn from_be_bytes(input: &[u8; 32]) -> Self {
        Self(DelegatedU256::from_be_bytes(input))
    }

    #[inline(always)]
    pub fn to_be_bytes(&self) -> [u8; 32] {
        self.0.to_be_bytes()
    }
}

impl From<ruint::aliases::U256> for U256 {
    #[inline(always)]
    fn from(value: ruint::aliases::U256) -> Self {
        // NOTE: we can not use precompile call due to alignment requirements
        Self::from_limbs(*value.as_limbs())
    }
}

impl From<U256> for ruint::aliases::U256 {
    #[inline(always)]
    fn from(value: U256) -> Self {
        ruint::aliases::U256::from_limbs(value.0.to_limbs())
    }
}

// delegated_u256/src/lib.rs

#[repr(align(32))]
struct DelegatedU256([u64; 4]);

// delegated_u256/src/delegation.rs

#[inline(always)]
/// # Safety
/// `a` and `b` must be 32 bytes aligned and point to 32 bytes of accessible memory.
unsafe fn bigint_op_delegation(
    a: *mut DelegatedU256,
    b: *const DelegatedU256,
    op: BigIntOps,
) -> u32 {
    bigint_op_delegation_with_carry_bit(a, b, false, op)
}

#[inline(always)]
/// # Safety
/// `a` and `b` must be 32 bytes aligned and point to 32 bytes of accessible memory.
unsafe fn bigint_op_delegation_with_carry_bit(
    a: *mut DelegatedU256,
    b: *const DelegatedU256,
    carry: bool,
    op: BigIntOps,
) -> u32 {
    // SAFETY: guaranteed by the caller
    unsafe { bigint_op_delegation_with_carry_bit_raw(a.cast(), b.cast(), carry, op) }
}

// delegated_u256/src/copy.rs

#[cfg(not(test))]
static mut SCRATCH_FOR_MUT: MaybeUninit<DelegatedU256> = MaybeUninit::uninit();

// The scratch of each thread, as the tests run in parallel. It is on the heap: thread-local
// storage is not 32-byte aligned on every host.
#[cfg(test)]
std::thread_local! {
    static SCRATCH_FOR_MUT: std::boxed::Box<core::cell::UnsafeCell<MaybeUninit<DelegatedU256>>> =
        std::boxed::Box::new(core::cell::UnsafeCell::new(MaybeUninit::uninit()));
}

#[inline(always)]
unsafe fn copy_from_operand(source: *const DelegatedU256) -> DelegatedU256 {
    let mut result = MaybeUninit::<DelegatedU256>::uninit();
    unsafe {
        let _ = bigint_op_delegation(result.as_mut_ptr(), source, BigIntOps::MemCpy);

        result.assume_init()
    }
}

impl Clone for DelegatedU256 {
    #[inline(always)]
    fn clone(&self) -> Self {
        unsafe { copy_from_operand(self as *const Self) }
    }

    #[inline(always)]
    fn clone_from(&mut self, source: &Self) {
        unsafe {
            let _ = bigint_op_delegation(
                self.0.as_mut_ptr().cast(),
                source.0.as_ptr().cast(),
                BigIntOps::MemCpy,
            );
        }
    }
}

/// # Safety
/// `dst` must be 32 bytes aligned and point to 32 bytes of accessible memory.
unsafe fn write_into_ptr_unchecked(dst: *mut DelegatedU256, source: &DelegatedU256) {
    unsafe {
        bigint_op_delegation(dst, source, BigIntOps::MemCpy);
    }
}

#[inline(always)]
/// Copies the operand into the static scratch slot, for the destructive
/// delegations (e.g. a subtraction used as a comparison) on a shared reference.
///
/// # Safety
/// `operand` must be 32 bytes aligned and point to 32 bytes of accessible memory.
unsafe fn copy_to_scratch(operand: *const DelegatedU256) -> *mut DelegatedU256 {
    #[cfg(not(test))]
    #[allow(static_mut_refs)]
    unsafe {
        let _ = bigint_op_delegation(SCRATCH_FOR_MUT.as_mut_ptr(), operand, BigIntOps::MemCpy);
        SCRATCH_FOR_MUT.as_mut_ptr()
    }

    #[cfg(test)]
    unsafe {
        let scratch = SCRATCH_FOR_MUT.with(|scratch| scratch.get().cast::<DelegatedU256>());
        let _ = bigint_op_delegation(scratch, operand, BigIntOps::MemCpy);
        scratch
    }
}

// delegated_u256/src/arithmetic.rs

// Immutable statics land in `.rodata` and are valid delegation operands without a runtime
// initialization
static ZERO: DelegatedU256 = DelegatedU256::ZERO;
static ONE: DelegatedU256 = DelegatedU256::ONE;

impl PartialEq for DelegatedU256 {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            // equality is non-destructive, so we can cast
            let eq = bigint_op_delegation(
                (self as *const Self).cast_mut(),
                other as *const Self,
                BigIntOps::Eq,
            );

            eq != 0
        }
    }
}

impl Eq for DelegatedU256 {}

impl Ord for DelegatedU256 {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe {
            let scratch = copy_to_scratch(self as *const Self);
            let other = other as *const Self;
            let eq = bigint_op_delegation(scratch, other, BigIntOps::Eq);
            if eq != 0 {
                Ordering::Equal
            } else {
                let borrow = bigint_op_delegation(scratch, other, BigIntOps::Sub);
                if borrow != 0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}

impl PartialOrd for DelegatedU256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl DelegatedU256 {
    const ZERO: Self = Self([0; 4]);
    const ONE: Self = Self([1, 0, 0, 0]);

    fn is_zero(&self) -> bool {
        self.eq_static(core::ptr::addr_of!(ZERO))
    }

    fn is_one(&self) -> bool {
        self.eq_static(core::ptr::addr_of!(ONE))
    }

    #[inline(always)]
    fn eq_static(&self, constant: *const Self) -> bool {
        // we can cast constness since equality is non-destructive
        let eq = unsafe {
            bigint_op_delegation((self as *const Self).cast_mut(), constant, BigIntOps::Eq)
        };

        eq != 0
    }

    fn overflowing_add_assign(&mut self, rhs: &Self) -> bool {
        unsafe {
            let carry = bigint_op_delegation(self as *mut Self, rhs as *const Self, BigIntOps::Add);
            carry != 0
        }
    }

    fn overflowing_sub_assign(&mut self, rhs: &Self) -> bool {
        unsafe {
            let borrow =
                bigint_op_delegation(self as *mut Self, rhs as *const Self, BigIntOps::Sub);

            borrow != 0
        }
    }

    fn overflowing_sub_assign_with_borrow(&mut self, rhs: &Self, borrow: bool) -> bool {
        unsafe {
            let borrow = bigint_op_delegation_with_carry_bit(
                self as *mut Self,
                rhs as *const Self,
                borrow,
                BigIntOps::Sub,
            );

            borrow != 0
        }
    }

    fn widening_mul_assign_into(&mut self, high: &mut Self, rhs: &Self) {
        unsafe {
            bigint_op_delegation(self as *mut Self, rhs as *const Self, BigIntOps::MulLow);
            bigint_op_delegation(high as *mut Self, rhs as *const Self, BigIntOps::MulHigh);
        }
    }
}

// delegated_u256/src/utils.rs

impl DelegatedU256 {
    const fn as_limbs(&self) -> &[u64; 4] {
        &self.0
    }

    // as in zksync-os
    #[allow(clippy::wrong_self_convention)]
    const fn to_limbs(self) -> [u64; 4] {
        self.0
    }

    const fn from_limbs(limbs: [u64; 4]) -> Self {
        Self(limbs)
    }

    #[inline(always)]
    fn from_be_bytes(input: &[u8; 32]) -> Self {
        unsafe {
            let mut result = MaybeUninit::<DelegatedU256>::uninit();
            Self::write_be_bytes_into_slot(input.as_ptr(), result.as_mut_ptr());
            result.assume_init()
        }
    }

    /// The big-endian bytes of the value. Without delegation, RISC-V returns the
    /// `rev8`-reversed words by value, with no branch or scratch buffer, so the
    /// caller's copy can stay in registers.
    #[inline(always)]
    fn to_be_bytes(&self) -> [u8; 32] {
        #[cfg(target_arch = "riscv32")]
        // SAFETY: `self` is an aligned, initialized slot of eight words
        unsafe {
            crate::byte_order::words_to_be_bytes_32((self as *const Self).cast::<u32>())
        }

        #[cfg(not(target_arch = "riscv32"))]
        {
            let mut out = MaybeUninit::<[u8; 32]>::uninit();
            // SAFETY: `out` is 32 writable bytes, fully written below
            unsafe {
                self.write_be_bytes_into(&mut *out.as_mut_ptr());
                out.assume_init()
            }
        }
    }

    /// Writes the big-endian bytes of the value into `dst`. Other targets use a reversed
    /// byte-by-byte copy.
    #[cfg(not(target_arch = "riscv32"))]
    #[inline(always)]
    fn write_be_bytes_into(&self, dst: &mut [u8; 32]) {
        // SAFETY: `self` is 32 initialized bytes, `dst` is 32 writable bytes
        unsafe {
            Self::reversed_byte_copy((self as *const Self).cast::<u8>(), dst.as_mut_ptr());
        }
    }

    /// Copies 32 bytes from `src` to `dst` in reverse order, byte by byte: the
    /// big-endian/little-endian conversion without the delegation.
    ///
    /// # Safety
    /// `src` must be readable and `dst` writable for 32 bytes; they must not overlap.
    #[cfg(not(target_arch = "riscv32"))]
    #[inline(always)]
    unsafe fn reversed_byte_copy(src: *const u8, dst: *mut u8) {
        unsafe {
            for i in 0..32 {
                dst.add(31 - i).write(src.add(i).read());
            }
        }
    }

    /// Writes the big-endian integer at `src` into the slot `dst`. Without delegation, RISC-V
    /// reads source words using `rev8` (shifting aligned words together if `src` is
    /// unaligned); other targets use a reversed byte-by-byte copy.
    ///
    /// # Safety
    /// `src` must be readable for 32 bytes, `dst` must be 32 bytes aligned and writable;
    /// source and destination must not overlap.
    #[inline(always)]
    unsafe fn write_be_bytes_into_slot(src: *const u8, dst: *mut Self) {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            crate::byte_order::read_be_into_words::<32>(src, dst.cast::<u32>());
        }

        #[cfg(not(target_arch = "riscv32"))]
        unsafe {
            Self::reversed_byte_copy(src, dst.cast::<u8>());
        }
    }
}

impl core::fmt::Debug for DelegatedU256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(self, f)
    }
}

impl core::fmt::LowerHex for DelegatedU256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for word in self.as_limbs().iter().rev() {
            write!(f, "{word:016x}")?;
        }

        core::fmt::Result::Ok(())
    }
}
