//! The base field in four saturated 64-bit limbs, for 64-bit targets without the bigint
//! delegation (hosts, and the RV64 guest, whose machine multiplies with `MUL` and `MULHU` only:
//! everything here is unsigned).
//!
//! `p = 2^256 - C` with `C = 2^32 + 977`. An element is any representative of its residue below
//! `2^256`: the canonical one, or for the residues below `C` also that plus `p` (the "weak" form,
//! as the delegated field has it). So there are no magnitudes and nothing to normalize between
//! operations; the comparisons are modulo `p`, and the encodings and the parity reduce first.
//! Nothing is constant time: the rare carries are branches.

use super::field_5x52::FieldStorage5x52;
use crate::k256::FieldBytes;

/// `2^256 mod p`
const C: u64 = 0x1_0000_03D1;
/// The lowest limb of `p`; the other three are `u64::MAX`
const P0: u64 = C.wrapping_neg();

/// The branches of the arithmetic, counted in tests on the thread that takes them: the tests
/// show that every one is taken, and how often on which operands. Nothing of it is in a build
/// that is not a test.
#[cfg(test)]
pub(crate) mod branches {
    use core::cell::Cell;

    macro_rules! branches {
        ($($name:ident: $what:literal,)+) => {
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            #[repr(usize)]
            pub(crate) enum Branch {
                $($name,)+
            }

            impl Branch {
                pub(crate) const ALL: &'static [(Branch, &'static str)] =
                    &[$((Branch::$name, $what),)+];
            }
        };
    }

    branches! {
        Add: "add: calls (also doubling, adding an integer)",
        AddCarry: "add: a carry of 2^256, folded as C without a branch",
        AddSecondCarry: "add: a carry out of the lowest limb after that (cold)",
        Sub: "sub: calls (also negation, the subtraction from)",
        SubBorrow: "sub: a borrow of 2^256, folded as C without a branch",
        SubSecondBorrow: "sub: a borrow from the lowest limb after that (cold)",
        SubTwo: "sub of two: calls",
        SubTwoBorrow: "sub of two: one or two borrows of 2^256, folded as C without a branch",
        SubTwoBorrows: "sub of two: two borrows of 2^256",
        SubTwoSecondBorrow: "sub of two: a borrow from the lowest limb after that (cold)",
        Reduce: "mul, square: calls",
        ReduceFoldCarry: "mul, square: the last fold carries into the second limb (no branch)",
        ReduceSecondCarry: "mul, square: that carry overflows the second limb (cold)",
        Small: "small multiple: calls",
        SmallFold: "small multiple: a top limb to fold (no branch)",
        SmallSecondCarry: "small multiple: the fold overflows the second limb (cold)",
        CarryIntoThird: "cold carries: on into the third limb",
        CarryIntoFourth: "cold carries: on into the fourth limb",
        CarryWrap: "cold carries: past 2^256, folded as C once more",
        CarryWrapLow: "cold carries: and that out of the lowest limb again",
        BorrowFromThird: "cold borrows: on from the third limb",
        BorrowFromFourth: "cold borrows: on from the fourth limb",
        BorrowWrap: "cold borrows: past 2^256, folded as C once more",
        ZeroCandidate: "is zero / is one: the lowest limb is that of a candidate",
        EqualOtherwise: "equality: the limbs differ, so the canonical forms are compared (cold)",
        NotCanonical: "canonical form: the element is p or above",
    }

    pub(crate) const COUNT: usize = Branch::ALL.len();

    thread_local! {
        static COUNTS: Cell<[u64; COUNT]> = const { Cell::new([0; COUNT]) };
    }

    pub(crate) fn hit(branch: Branch) {
        COUNTS.with(|counts| {
            let mut all = counts.get();
            all[branch as usize] += 1;
            counts.set(all);
        });
    }

    /// The counts of this thread so far
    pub(crate) fn snapshot() -> [u64; COUNT] {
        COUNTS.with(Cell::get)
    }
}

/// Counts a branch, in tests only
macro_rules! hit {
    ($branch:ident) => {
        #[cfg(test)]
        branches::hit(branches::Branch::$branch);
    };
}

/// `a * b + c + d`, which fits: `(2^64 - 1)^2 + 2 (2^64 - 1) = 2^128 - 1`
#[inline(always)]
fn mac(a: u64, b: u64, c: u64, d: u64) -> (u64, u64) {
    let wide = a as u128 * b as u128 + c as u128 + d as u128;
    (wide as u64, (wide >> 64) as u64)
}

/// `a + b + carry`
#[inline(always)]
fn adc(a: u64, b: u64, carry: bool) -> (u64, bool) {
    let (sum, first) = a.overflowing_add(b);
    let (sum, second) = sum.overflowing_add(carry as u64);
    (sum, first | second)
}

/// `a - b - borrow`
#[inline(always)]
fn sbb(a: u64, b: u64, borrow: bool) -> (u64, bool) {
    let (difference, first) = a.overflowing_sub(b);
    let (difference, second) = difference.overflowing_sub(borrow as u64);
    (difference, first | second)
}

/// `r + carry * 2^64` folded below `2^256`, for a small `carry` (1 or 2): the path of the rare
/// carries, out of line. Its callers are the addition, whose fold of `2^256` carried out of the
/// lowest limb, and the folds of the products, whose carry overflows the second limb.
///
/// A carry past `2^256` is `C` once more. The limbs above the second are zero then and the second
/// is below `carry`, so the sum is below `2^64 + 2^64 carry`: adding `C` cannot pass `2^256`
/// again, and a carry out of the lowest limb fits the second. (After an addition the lowest limb
/// is below `C` at that point, so there it does not carry.)
#[cold]
#[inline(never)]
fn carry_up(mut r: [u64; 4], carry: u64) -> [u64; 4] {
    let (sum, overflow) = r[1].overflowing_add(carry);
    r[1] = sum;
    if !overflow {
        return r;
    }
    hit!(CarryIntoThird);
    let (sum, overflow) = r[2].overflowing_add(1);
    r[2] = sum;
    if !overflow {
        return r;
    }
    hit!(CarryIntoFourth);
    let (sum, overflow) = r[3].overflowing_add(1);
    r[3] = sum;
    if !overflow {
        return r;
    }
    hit!(CarryWrap);
    let (sum, overflow) = r[0].overflowing_add(C);
    r[0] = sum;
    if overflow {
        hit!(CarryWrapLow);
        r[1] += 1;
    }
    r
}

/// `r - 2^64` brought back below `2^256`: the path of the rare borrows, out of line. Its callers
/// are the subtractions, whose fold of the borrowed `2^256` (subtracting `C` for each, at most
/// two) borrowed from the lowest limb, which is therefore `2^64 - 2 C` or more.
///
/// A borrow past `2^256` is `C` once more, which that lowest limb has: nothing borrows again.
#[cold]
#[inline(never)]
fn borrow_up(mut r: [u64; 4]) -> [u64; 4] {
    let (difference, borrow) = r[1].overflowing_sub(1);
    r[1] = difference;
    if !borrow {
        return r;
    }
    hit!(BorrowFromThird);
    let (difference, borrow) = r[2].overflowing_sub(1);
    r[2] = difference;
    if !borrow {
        return r;
    }
    hit!(BorrowFromFourth);
    let (difference, borrow) = r[3].overflowing_sub(1);
    r[3] = difference;
    if !borrow {
        return r;
    }
    hit!(BorrowWrap);
    r[0] -= C;
    r
}

/// `low + C * high` folded below `2^256`.
///
/// The sum is `carry * 2^256 + r` with `carry <= C`, and `carry * 2^256` is `carry * C`, below
/// `2^67`: it goes into the lowest limb, and what it carries (0, 1 or 2) into the second, which
/// overflows only if it is `2^64 - 2` or more: the cold path.
#[inline(always)]
fn reduce_wide(low: [u64; 4], high: [u64; 4]) -> [u64; 4] {
    hit!(Reduce);
    let (r0, carry) = mac(high[0], C, low[0], 0);
    let (r1, carry) = mac(high[1], C, low[1], carry);
    let (r2, carry) = mac(high[2], C, low[2], carry);
    let (r3, carry) = mac(high[3], C, low[3], carry);
    let (r0, carry) = mac(carry, C, r0, 0);
    #[cfg(test)]
    if carry != 0 {
        hit!(ReduceFoldCarry);
    }
    let (sum, overflow) = r1.overflowing_add(carry);
    if overflow {
        hit!(ReduceSecondCarry);
        return carry_up([r0, r1, r2, r3], carry);
    }
    [r0, sum, r2, r3]
}

/// The 512-bit product, low half and high half
#[inline(always)]
fn mul_wide(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    let (t0, carry) = mac(a[0], b[0], 0, 0);
    let (t1, carry) = mac(a[0], b[1], carry, 0);
    let (t2, carry) = mac(a[0], b[2], carry, 0);
    let (t3, t4) = mac(a[0], b[3], carry, 0);

    let (t1, carry) = mac(a[1], b[0], t1, 0);
    let (t2, carry) = mac(a[1], b[1], t2, carry);
    let (t3, carry) = mac(a[1], b[2], t3, carry);
    let (t4, t5) = mac(a[1], b[3], t4, carry);

    let (t2, carry) = mac(a[2], b[0], t2, 0);
    let (t3, carry) = mac(a[2], b[1], t3, carry);
    let (t4, carry) = mac(a[2], b[2], t4, carry);
    let (t5, t6) = mac(a[2], b[3], t5, carry);

    let (t3, carry) = mac(a[3], b[0], t3, 0);
    let (t4, carry) = mac(a[3], b[1], t4, carry);
    let (t5, carry) = mac(a[3], b[2], t5, carry);
    let (t6, t7) = mac(a[3], b[3], t6, carry);

    ([t0, t1, t2, t3], [t4, t5, t6, t7])
}

/// The 512-bit square: the six cross products once, doubled, then the four squares
#[inline(always)]
fn square_wide(a: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    let (t1, carry) = mac(a[0], a[1], 0, 0);
    let (t2, carry) = mac(a[0], a[2], carry, 0);
    let (t3, t4) = mac(a[0], a[3], carry, 0);

    let (t3, carry) = mac(a[1], a[2], t3, 0);
    let (t4, t5) = mac(a[1], a[3], t4, carry);

    let (t5, t6) = mac(a[2], a[3], t5, 0);

    let t7 = t6 >> 63;
    let t6 = t6 << 1 | t5 >> 63;
    let t5 = t5 << 1 | t4 >> 63;
    let t4 = t4 << 1 | t3 >> 63;
    let t3 = t3 << 1 | t2 >> 63;
    let t2 = t2 << 1 | t1 >> 63;
    let t1 = t1 << 1;

    let (t0, carry) = mac(a[0], a[0], 0, 0);
    let (t1, carry) = adc(t1, carry, false);
    let (t2, carry) = mac(a[1], a[1], t2, carry as u64);
    let (t3, carry) = adc(t3, carry, false);
    let (t4, carry) = mac(a[2], a[2], t4, carry as u64);
    let (t5, carry) = adc(t5, carry, false);
    let (t6, carry) = mac(a[3], a[3], t6, carry as u64);
    let (t7, _) = adc(t7, carry, false);

    ([t0, t1, t2, t3], [t4, t5, t6, t7])
}

/// `a + b` folded below `2^256`.
///
/// A carry of `2^256` (for every other pair of uniform operands) is `C`, added to the lowest
/// limb without a branch. That addition carries only if the limb is `2^64 - C` or more: the
/// cold path.
#[inline(always)]
fn add(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    hit!(Add);
    let (r0, carry) = a[0].overflowing_add(b[0]);
    let (r1, carry) = adc(a[1], b[1], carry);
    let (r2, carry) = adc(a[2], b[2], carry);
    let (r3, carry) = adc(a[3], b[3], carry);
    #[cfg(test)]
    if carry {
        hit!(AddCarry);
    }
    let (r0, overflow) = r0.overflowing_add((carry as u64).wrapping_neg() & C);
    if overflow {
        hit!(AddSecondCarry);
        return carry_up([r0, r1, r2, r3], 1);
    }
    [r0, r1, r2, r3]
}

/// `a - b` brought back below `2^256`.
///
/// A borrow of `2^256` (for every other pair of uniform operands) is `C`, subtracted from the
/// lowest limb without a branch. That subtraction borrows only if the limb is below `C`: the
/// cold path.
#[inline(always)]
fn sub(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    hit!(Sub);
    let (r0, borrow) = a[0].overflowing_sub(b[0]);
    let (r1, borrow) = sbb(a[1], b[1], borrow);
    let (r2, borrow) = sbb(a[2], b[2], borrow);
    let (r3, borrow) = sbb(a[3], b[3], borrow);
    #[cfg(test)]
    if borrow {
        hit!(SubBorrow);
    }
    let (r0, underflow) = r0.overflowing_sub((borrow as u64).wrapping_neg() & C);
    if underflow {
        hit!(SubSecondBorrow);
        return borrow_up([r0, r1, r2, r3]);
    }
    [r0, r1, r2, r3]
}

/// `a - b - c` brought back below `2^256`: one pass instead of two subtractions.
///
/// The borrows of `2^256` (none, one or two: one for two thirds of the uniform operands) are
/// that many `C`, subtracted from the lowest limb without a branch. That subtraction borrows
/// only if the limb is below it: the cold path.
#[inline(always)]
fn sub_two(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4]) -> [u64; 4] {
    hit!(SubTwo);
    let (difference, first) = a[0].overflowing_sub(b[0]);
    let (r0, second) = difference.overflowing_sub(c[0]);
    let mut borrow = first as u64 + second as u64;
    let mut r = [r0, 0, 0, 0];
    let mut i = 1;
    while i < 4 {
        let (difference, first) = a[i].overflowing_sub(b[i]);
        let (difference, second) = difference.overflowing_sub(c[i]);
        let (difference, third) = difference.overflowing_sub(borrow);
        r[i] = difference;
        // at most two: the limb is at least `-(2^65 - 2) - 2`
        borrow = first as u64 + second as u64 + third as u64;
        i += 1;
    }
    #[cfg(test)]
    if borrow != 0 {
        hit!(SubTwoBorrow);
        if borrow == 2 {
            hit!(SubTwoBorrows);
        }
    }
    let (r0, underflow) = r[0].overflowing_sub(borrow * C);
    if underflow {
        hit!(SubTwoSecondBorrow);
        return borrow_up([r0, r[1], r[2], r[3]]);
    }
    [r0, r[1], r[2], r[3]]
}

/// `a * small` folded below `2^256`.
///
/// The top limb of the product is below `small`, and times `C` below `2^65`: it goes into the
/// lowest limb, and what it carries (0, 1 or 2) into the second, as in `reduce_wide`.
#[inline(always)]
fn mul_small(a: &[u64; 4], small: u32) -> [u64; 4] {
    hit!(Small);
    let small = small as u64;
    let (r0, carry) = mac(a[0], small, 0, 0);
    let (r1, carry) = mac(a[1], small, carry, 0);
    let (r2, carry) = mac(a[2], small, carry, 0);
    let (r3, top) = mac(a[3], small, carry, 0);
    #[cfg(test)]
    if top != 0 {
        hit!(SmallFold);
    }
    let (r0, carry) = mac(top, C, r0, 0);
    let (sum, overflow) = r1.overflowing_add(carry);
    if overflow {
        hit!(SmallSecondCarry);
        return carry_up([r0, r1, r2, r3], carry);
    }
    [r0, sum, r2, r3]
}

/// Whether `a` is at least `p`, i.e. not canonical
#[inline(always)]
const fn is_at_least_p(a: &[u64; 4]) -> bool {
    (a[3] & a[2] & a[1]) == u64::MAX && a[0] >= P0
}

/// The canonical representative
#[inline(always)]
fn canonical(a: &[u64; 4]) -> [u64; 4] {
    if is_at_least_p(a) {
        hit!(NotCanonical);
        [a[0] - P0, 0, 0, 0]
    } else {
        *a
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FieldElement4x64(pub(super) [u64; 4]);

impl core::fmt::Debug for FieldElement4x64 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("0x")?;
        #[allow(deprecated)]
        for b in self.to_bytes().as_slice().iter() {
            f.write_fmt(format_args!("{b:02x}"))?;
        }
        Ok(())
    }
}

impl FieldElement4x64 {
    pub(super) const ZERO: Self = Self([0; 4]);
    pub(super) const ONE: Self = Self([1, 0, 0, 0]);
    pub(super) const BETA: Self = Self::from_bytes_unchecked(&[
        0x7a, 0xe9, 0x6a, 0x2b, 0x65, 0x7c, 0x07, 0x10, 0x6e, 0x64, 0x47, 0x9e, 0xac, 0x34, 0x34,
        0xe9, 0x9c, 0xf0, 0x49, 0x75, 0x12, 0xf5, 0x89, 0x95, 0xc1, 0x39, 0x6c, 0x28, 0x71, 0x95,
        0x01, 0xee,
    ]);
    /// The stored form (see `FieldStorage`) is the element itself, canonical
    pub(super) const DEFAULT: Self = Self::ZERO;

    /// The element of big-endian bytes, any 256-bit integer
    #[inline(always)]
    pub(super) const fn from_bytes_unchecked(bytes: &[u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        let mut i = 0;
        while i < 4 {
            let mut limb = 0u64;
            let mut j = 0;
            while j < 8 {
                limb = limb << 8 | bytes[8 * (3 - i) + j] as u64;
                j += 1;
            }
            limbs[i] = limb;
            i += 1;
        }
        Self(limbs)
    }

    /// The element of canonical big-endian bytes, `None` for the modulus or above
    #[inline(always)]
    pub(super) fn from_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let value = Self::from_bytes_unchecked(bytes);
        if is_at_least_p(&value.0) {
            None
        } else {
            Some(value)
        }
    }

    /// The element of little-endian limbs, any 256-bit integer: a representative of its residue
    #[inline(always)]
    pub(super) const fn from_limbs(limbs: [u64; 4]) -> Self {
        Self(limbs)
    }

    /// The element of the little-endian limbs of a canonical integer, `None` for the modulus or
    /// above
    #[inline(always)]
    pub(super) const fn from_canonical_limbs(limbs: [u64; 4]) -> Option<Self> {
        if is_at_least_p(&limbs) {
            None
        } else {
            Some(Self(limbs))
        }
    }

    /// The little-endian limbs of the canonical integer
    #[inline(always)]
    pub(super) fn to_canonical_limbs(self) -> [u64; 4] {
        canonical(&self.0)
    }

    /// Whether the elements are equal modulo `p`. Equal limbs are; elements that differ have
    /// the same residue only as its two forms (below `C`, and that plus `p`), which the
    /// canonical forms tell, out of line: the check of a hinted value compares a computed
    /// element with the one it must be, so the limbs are equal unless the hint is wrong or the
    /// residue is below `C`.
    #[inline(always)]
    pub(super) fn is_equal(&self, other: &Self) -> bool {
        let (a, b) = (&self.0, &other.0);
        if ((a[0] ^ b[0]) | (a[1] ^ b[1]) | (a[2] ^ b[2]) | (a[3] ^ b[3])) == 0 {
            return true;
        }
        #[cold]
        #[inline(never)]
        fn canonical_forms_are_equal(a: &[u64; 4], b: &[u64; 4]) -> bool {
            hit!(EqualOtherwise);
            canonical(a) == canonical(b)
        }
        canonical_forms_are_equal(a, b)
    }

    /// The canonical big-endian bytes
    #[inline(always)]
    pub(super) fn to_bytes(self) -> FieldBytes {
        let mut bytes = FieldBytes::default();
        self.write_bytes((&mut bytes[..]).try_into().expect("32 bytes"));
        bytes
    }

    /// The canonical big-endian bytes into `bytes`
    #[inline(always)]
    pub(super) fn write_bytes(self, bytes: &mut [u8; 32]) {
        let limbs = canonical(&self.0);
        for (chunk, limb) in bytes
            .as_chunks_mut::<8>()
            .0
            .iter_mut()
            .zip(limbs.iter().rev())
        {
            *chunk = limb.to_be_bytes();
        }
    }

    #[inline(always)]
    pub(super) fn mul_in_place(&mut self, rhs: &Self) {
        let (low, high) = mul_wide(&self.0, &rhs.0);
        self.0 = reduce_wide(low, high);
    }

    #[inline(always)]
    pub(super) fn square_in_place(&mut self) {
        let (low, high) = square_wide(&self.0);
        self.0 = reduce_wide(low, high);
    }

    #[inline(always)]
    pub(super) fn mul_int_in_place(&mut self, rhs: u32) {
        self.0 = mul_small(&self.0, rhs);
    }

    #[inline(always)]
    pub(super) fn add_in_place(&mut self, rhs: &Self) {
        self.0 = add(&self.0, &rhs.0);
    }

    #[inline(always)]
    pub(super) fn add_int_in_place(&mut self, rhs: u32) {
        self.0 = add(&self.0, &[rhs as u64, 0, 0, 0]);
    }

    #[inline(always)]
    pub(super) fn double_in_place(&mut self) {
        self.0 = add(&self.0, &self.0);
    }

    #[inline(always)]
    pub(super) fn triple_in_place(&mut self) {
        self.0 = mul_small(&self.0, 3);
    }

    #[inline(always)]
    pub(super) fn sub_in_place(&mut self, rhs: &Self) {
        self.0 = sub(&self.0, &rhs.0);
    }

    /// Computes `self = self - first - second`
    #[inline(always)]
    pub(super) fn sub_two_in_place(&mut self, first: &Self, second: &Self) {
        self.0 = sub_two(&self.0, &first.0, &second.0);
    }

    /// Computes `self = minuend - self`
    #[inline(always)]
    pub(super) fn sub_and_negate_in_place(&mut self, minuend: &Self) {
        self.0 = sub(&minuend.0, &self.0);
    }

    #[inline(always)]
    pub(super) const fn copy_from(&mut self, src: &Self) {
        *self = *src;
    }

    /// Initializes `dst` with a copy of `src`
    #[inline(always)]
    pub(super) const fn write_copy<'a>(
        dst: &'a mut core::mem::MaybeUninit<Self>,
        src: &Self,
    ) -> &'a mut Self {
        dst.write(*src)
    }

    /// The magnitude is that of the representations with lazy reduction: not used here
    #[inline(always)]
    pub(super) fn negate_in_place(&mut self, _magnitude: u32) {
        self.0 = sub(&[0; 4], &self.0);
    }

    /// The canonical representative: only the encodings, the parity and the stored form need it
    #[inline(always)]
    pub(super) fn normalize_in_place(&mut self) {
        self.0 = canonical(&self.0);
    }

    /// Zero is `0` or `p`. The lowest limb tells which one the element can be, and for most
    /// elements it is neither.
    #[inline(always)]
    pub(super) fn normalizes_to_zero(&self) -> bool {
        let a = &self.0;
        if a[0] == 0 {
            hit!(ZeroCandidate);
            (a[1] | a[2] | a[3]) == 0
        } else if a[0] == P0 {
            hit!(ZeroCandidate);
            (a[1] & a[2] & a[3]) == u64::MAX
        } else {
            false
        }
    }

    /// One is `1` or `p + 1`, told apart from the rest by the lowest limb as for zero
    #[inline(always)]
    pub(super) fn is_one(&self) -> bool {
        let a = &self.0;
        if a[0] == 1 {
            hit!(ZeroCandidate);
            (a[1] | a[2] | a[3]) == 0
        } else if a[0] == P0 + 1 {
            hit!(ZeroCandidate);
            (a[1] & a[2] & a[3]) == u64::MAX
        } else {
            false
        }
    }

    /// `p - 1` is the only representative of minus one below `2^256`
    #[inline(always)]
    pub(super) const fn is_minus_one(&self) -> bool {
        let a = &self.0;
        a[0] == P0 - 1 && (a[1] & a[2] & a[3]) == u64::MAX
    }

    /// Parity of the canonical representative
    #[inline(always)]
    pub(super) fn is_odd(&self) -> bool {
        canonical(&self.0)[0] & 1 != 0
    }

    /// The limbs of an element are its stored form, canonical for the tables
    #[inline(always)]
    pub(super) const fn from_storage_5x52(storage: FieldStorage5x52) -> Self {
        Self(storage.0)
    }

    /// NOTE: expects a normalized element
    #[cfg(test)]
    #[inline(always)]
    pub(super) const fn to_storage(self) -> Self {
        self
    }

    #[inline(always)]
    pub(super) const fn to_field_elem(self) -> Self {
        self
    }

    #[inline(always)]
    fn pow2k_in_place(&mut self, k: usize) {
        let mut i = 0;
        while i < k {
            self.square_in_place();
            i += 1;
        }
    }

    /// `self^(p - 2)`, the inverse (zero for zero) by multiplications only: what the machine of
    /// the RV64 guest can do, as the safegcd needs signed double-width products
    #[cfg_attr(not(any(target_arch = "riscv64", test)), allow(dead_code))]
    pub(super) fn invert_by_exponentiation_in_place(&mut self) {
        let x1 = *self;

        self.pow2k_in_place(1);
        self.mul_in_place(&x1);
        let x2 = *self;

        self.pow2k_in_place(1);
        self.mul_in_place(&x1);
        let x3 = *self;

        self.pow2k_in_place(3);
        self.mul_in_place(&x3);

        self.pow2k_in_place(3);
        self.mul_in_place(&x3);

        self.pow2k_in_place(2);
        self.mul_in_place(&x2);
        let x11 = *self;

        self.pow2k_in_place(11);
        self.mul_in_place(&x11);
        let x22 = *self;

        self.pow2k_in_place(22);
        self.mul_in_place(&x22);
        let x44 = *self;

        self.pow2k_in_place(44);
        self.mul_in_place(&x44);
        let x88 = *self;

        self.pow2k_in_place(88);
        self.mul_in_place(&x88);

        self.pow2k_in_place(44);
        self.mul_in_place(&x44);

        self.pow2k_in_place(3);
        self.mul_in_place(&x3);

        self.pow2k_in_place(23);
        self.mul_in_place(&x22);
        self.pow2k_in_place(5);
        self.mul_in_place(&x1);
        self.pow2k_in_place(3);
        self.mul_in_place(&x2);
        self.pow2k_in_place(2);
        self.mul_in_place(&x1);
    }

    /// The inverse, zero for zero
    #[inline(always)]
    pub(super) fn invert_in_place(&mut self) {
        #[cfg(target_arch = "riscv64")]
        self.invert_by_exponentiation_in_place();

        #[cfg(not(target_arch = "riscv64"))]
        {
            self.0 = crate::field_inverse::inverse_words(
                canonical(&self.0),
                [P0, u64::MAX, u64::MAX, u64::MAX],
            );
        }
    }
}

#[cfg(test)]
impl PartialEq for FieldElement4x64 {
    fn eq(&self, other: &Self) -> bool {
        canonical(&self.0) == canonical(&other.0)
    }
}

/// Any 256-bit integer, with the edges of the representation as likely as the rest: the two
/// forms of zero and of the residues next to it, and the limbs whose carries are rare
#[cfg(test)]
impl proptest::arbitrary::Arbitrary for FieldElement4x64 {
    type Parameters = ();

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::{any, prop_oneof, Just, Strategy};

        prop_oneof![
            4 => any::<[u64; 4]>(),
            1 => (0..2 * C).prop_map(|low| [low, 0, 0, 0]),
            1 => (0..2 * C).prop_map(|below| [u64::MAX - below, u64::MAX, u64::MAX, u64::MAX]),
            1 => (any::<u64>(), any::<[bool; 3]>()).prop_map(|(low, ones)| {
                let limb = |one: bool| if one { u64::MAX } else { 0 };
                [low, limb(ones[0]), limb(ones[1]), limb(ones[2])]
            }),
            1 => Just([P0, u64::MAX, u64::MAX, u64::MAX]),
        ]
        .prop_map(Self)
        .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}

#[cfg(test)]
mod tests {
    use super::branches::{self, Branch};
    use super::*;
    use num_bigint::BigUint;
    use proptest::{prop_assert, prop_assert_eq, proptest};
    use std::vec::Vec;

    const MAX: u64 = u64::MAX;
    const P: [u64; 4] = [P0, MAX, MAX, MAX];
    const HALF: u64 = 1 << 63;

    fn integer(limbs: &[u64; 4]) -> BigUint {
        limbs.iter().rev().fold(BigUint::ZERO, |sum, limb| {
            (sum << 64) + BigUint::from(*limb)
        })
    }

    fn modulus() -> BigUint {
        integer(&P)
    }

    fn residue(element: &FieldElement4x64) -> BigUint {
        integer(&element.0) % modulus()
    }

    /// How often `f` takes each branch, and its value
    fn counted<R>(f: impl FnOnce() -> R) -> (R, [u64; branches::COUNT]) {
        let before = branches::snapshot();
        let value = f();
        let after = branches::snapshot();
        (value, core::array::from_fn(|i| after[i] - before[i]))
    }

    /// `f` takes exactly the branches `expected` (each once) among `of`
    #[track_caller]
    fn takes<R>(of: &[Branch], expected: &[Branch], f: impl FnOnce() -> R) -> R {
        let (value, counts) = counted(f);
        for branch in of {
            assert_eq!(
                counts[*branch as usize],
                u64::from(expected.contains(branch)),
                "{branch:?}"
            );
        }
        assert!(expected.iter().all(|branch| of.contains(branch)));
        value
    }

    /// The branches of an addition and of the cold carries
    const OF_ADD: &[Branch] = &[
        Branch::AddCarry,
        Branch::AddSecondCarry,
        Branch::CarryIntoThird,
        Branch::CarryIntoFourth,
        Branch::CarryWrap,
        Branch::CarryWrapLow,
    ];
    /// The branches of a subtraction and of the cold borrows
    const OF_SUB: &[Branch] = &[
        Branch::SubBorrow,
        Branch::SubSecondBorrow,
        Branch::BorrowFromThird,
        Branch::BorrowFromFourth,
        Branch::BorrowWrap,
    ];
    /// The branches of the fold of a product and of the cold carries
    const OF_REDUCE: &[Branch] = &[
        Branch::ReduceFoldCarry,
        Branch::ReduceSecondCarry,
        Branch::CarryIntoThird,
        Branch::CarryIntoFourth,
        Branch::CarryWrap,
        Branch::CarryWrapLow,
    ];
    /// The branches of a small multiple and of the cold carries
    const OF_SMALL: &[Branch] = &[
        Branch::SmallFold,
        Branch::SmallSecondCarry,
        Branch::CarryIntoThird,
        Branch::CarryIntoFourth,
        Branch::CarryWrap,
        Branch::CarryWrapLow,
    ];

    /// `a + b` takes exactly the branches `expected`, and is the sum, with the limbs `limbs`
    #[track_caller]
    fn add_takes(a: [u64; 4], b: [u64; 4], expected: &[Branch], limbs: [u64; 4]) {
        let sum = takes(OF_ADD, expected, || add(&a, &b));
        assert_eq!(sum, limbs);
        assert_eq!(
            integer(&sum) % modulus(),
            (integer(&a) + integer(&b)) % modulus()
        );
        // the same through the element, in either order
        let mut r = FieldElement4x64(b);
        takes(OF_ADD, expected, || r.add_in_place(&FieldElement4x64(a)));
        assert_eq!(r.0, limbs);
    }

    #[test]
    fn add_without_a_carry() {
        add_takes([1, 2, 3, 4], [5, 6, 7, 8], &[], [6, 8, 10, 12]);
        // a carry between limbs is not one of 2^256
        add_takes([MAX, MAX, MAX, 0], [1, 0, 0, 0], &[], [0, 0, 0, 1]);
        add_takes(P, [C - 1, 0, 0, 0], &[], [MAX; 4]);
    }

    #[test]
    fn add_with_the_first_carry_only() {
        let first = &[Branch::AddCarry];
        add_takes([0, 0, 0, HALF], [5, 0, 0, HALF], first, [5 + C, 0, 0, 0]);
        add_takes(P, [C, 0, 0, 0], first, [C, 0, 0, 0]);
        // the lowest limb takes C up to its last value
        add_takes(
            [MAX - C, 0, 0, HALF],
            [0, 0, 0, HALF],
            first,
            [MAX, 0, 0, 0],
        );
    }

    #[test]
    fn add_whose_fold_carries_out_of_the_lowest_limb() {
        let second = &[Branch::AddCarry, Branch::AddSecondCarry];
        add_takes([MAX, 0, 0, HALF], [0, 7, 0, HALF], second, [C - 1, 8, 0, 0]);
        // the first value of the lowest limb that carries
        add_takes(
            [MAX - C + 1, 0, 0, HALF],
            [0, 0, 0, HALF],
            second,
            [0, 1, 0, 0],
        );
    }

    #[test]
    fn add_whose_second_carry_ripples_through_one_limb() {
        add_takes(
            [MAX, 0, 0, HALF],
            [0, MAX, 7, HALF],
            &[
                Branch::AddCarry,
                Branch::AddSecondCarry,
                Branch::CarryIntoThird,
            ],
            [C - 1, 0, 8, 0],
        );
    }

    #[test]
    fn add_whose_second_carry_ripples_through_two_limbs() {
        add_takes(
            [MAX, 0, 0, HALF],
            [0, MAX, MAX, HALF + 7],
            &[
                Branch::AddCarry,
                Branch::AddSecondCarry,
                Branch::CarryIntoThird,
                Branch::CarryIntoFourth,
            ],
            [C - 1, 0, 0, 8],
        );
    }

    /// The sum is `2^256 + p` or more: only for two operands above `p`
    #[test]
    fn add_whose_second_carry_wraps_past_two_to_the_256() {
        let wrap = &[
            Branch::AddCarry,
            Branch::AddSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
            Branch::CarryWrap,
        ];
        add_takes([MAX; 4], [MAX; 4], wrap, [2 * C - 2, 0, 0, 0]);
        // the least such sum: (p + 1) + (2^256 - 1) = 2^256 + p
        add_takes([P0 + 1, MAX, MAX, MAX], [MAX; 4], wrap, [C, 0, 0, 0]);
        // the lowest limb is below C after the first fold, so the second cannot carry
        // (`CarryWrapLow`, which the products reach, is not taken)
    }

    #[test]
    fn doubling_and_adding_an_integer_take_the_carries_of_the_addition() {
        let double = |a: [u64; 4], expected: &[Branch], limbs: [u64; 4]| {
            let mut r = FieldElement4x64(a);
            takes(OF_ADD, expected, || r.double_in_place());
            assert_eq!(r.0, limbs);
            assert_eq!(integer(&limbs) % modulus(), integer(&a) * 2u32 % modulus());
        };
        double([1, 2, 3, 4], &[], [2, 4, 6, 8]);
        double([3, 0, 0, HALF], &[Branch::AddCarry], [6 + C, 0, 0, 0]);
        double(
            [MAX, 0, 0, HALF],
            &[Branch::AddCarry, Branch::AddSecondCarry],
            [C - 2, 2, 0, 0],
        );
        double(
            [MAX, MAX, MAX, MAX],
            &[
                Branch::AddCarry,
                Branch::AddSecondCarry,
                Branch::CarryIntoThird,
                Branch::CarryIntoFourth,
                Branch::CarryWrap,
            ],
            [2 * C - 2, 0, 0, 0],
        );

        let add_int = |a: [u64; 4], small: u32, expected: &[Branch], limbs: [u64; 4]| {
            let mut r = FieldElement4x64(a);
            takes(OF_ADD, expected, || r.add_int_in_place(small));
            assert_eq!(r.0, limbs);
            assert_eq!(
                integer(&limbs) % modulus(),
                (integer(&a) + small) % modulus()
            );
        };
        add_int([1, 2, 3, 4], 7, &[], [8, 2, 3, 4]);
        add_int(
            [MAX, MAX, MAX, MAX],
            7,
            &[Branch::AddCarry],
            [6 + C, 0, 0, 0],
        );
        // the sum past 2^256 is below 2^32, so an integer never takes the second carry
        add_int(
            [MAX, MAX, MAX, MAX],
            u32::MAX,
            &[Branch::AddCarry],
            [u32::MAX as u64 - 1 + C, 0, 0, 0],
        );
    }

    /// `a - b` takes exactly the branches `expected`, and is the difference, with the limbs
    /// `limbs`
    #[track_caller]
    fn sub_takes(a: [u64; 4], b: [u64; 4], expected: &[Branch], limbs: [u64; 4]) {
        let p = modulus();
        let difference = takes(OF_SUB, expected, || sub(&a, &b));
        assert_eq!(difference, limbs);
        assert_eq!(
            integer(&difference) % &p,
            (integer(&a) % &p + &p - integer(&b) % &p) % &p
        );
        // the same through the element, and as the subtraction from
        let mut r = FieldElement4x64(a);
        takes(OF_SUB, expected, || r.sub_in_place(&FieldElement4x64(b)));
        assert_eq!(r.0, limbs);
        let mut r = FieldElement4x64(b);
        takes(OF_SUB, expected, || {
            r.sub_and_negate_in_place(&FieldElement4x64(a))
        });
        assert_eq!(r.0, limbs);
    }

    #[test]
    fn sub_without_a_borrow() {
        sub_takes([5, 6, 7, 8], [1, 2, 3, 4], &[], [4, 4, 4, 4]);
        // a borrow between limbs is not one of 2^256
        sub_takes([0, 0, 0, 1], [1, 0, 0, 0], &[], [MAX, MAX, MAX, 0]);
        sub_takes([MAX; 4], [MAX; 4], &[], [0; 4]);
    }

    #[test]
    fn sub_with_the_first_borrow_only() {
        let first = &[Branch::SubBorrow];
        sub_takes([2 * C, 0, 0, 0], [0, 0, 0, 1], first, [C, 0, 0, MAX]);
        // the lowest limb gives C down to its last value
        sub_takes([C, 0, 0, 0], [0, 0, 0, 1], first, [0, 0, 0, MAX]);
        sub_takes([0; 4], [1, 0, 0, 0], first, [P0 - 1, MAX, MAX, MAX]);
    }

    #[test]
    fn sub_whose_fold_borrows_from_the_lowest_limb() {
        let second = &[Branch::SubBorrow, Branch::SubSecondBorrow];
        sub_takes(
            [5, 9, 0, 0],
            [0, 0, 0, 1],
            second,
            [5u64.wrapping_sub(C), 8, 0, MAX],
        );
        // the first value of the lowest limb that borrows
        sub_takes([C - 1, 9, 0, 0], [0, 0, 0, 1], second, [MAX, 8, 0, MAX]);
    }

    #[test]
    fn sub_whose_second_borrow_ripples_through_one_limb() {
        sub_takes(
            [5, 0, 9, 0],
            [0, 0, 0, 1],
            &[
                Branch::SubBorrow,
                Branch::SubSecondBorrow,
                Branch::BorrowFromThird,
            ],
            [5u64.wrapping_sub(C), MAX, 8, MAX],
        );
    }

    #[test]
    fn sub_whose_second_borrow_ripples_through_two_limbs() {
        sub_takes(
            [5, 0, 0, 0],
            [0, 0, 0, 1],
            &[
                Branch::SubBorrow,
                Branch::SubSecondBorrow,
                Branch::BorrowFromThird,
                Branch::BorrowFromFourth,
            ],
            [5u64.wrapping_sub(C), MAX, MAX, MAX - 1],
        );
    }

    /// The difference is below `C - 2^256`: only for a subtrahend above `p`
    #[test]
    fn sub_whose_second_borrow_wraps_past_two_to_the_256() {
        let wrap = &[
            Branch::SubBorrow,
            Branch::SubSecondBorrow,
            Branch::BorrowFromThird,
            Branch::BorrowFromFourth,
            Branch::BorrowWrap,
        ];
        // 0 - (2^256 - 5) = 5 - C
        sub_takes(
            [0; 4],
            [MAX - 4, MAX, MAX, MAX],
            wrap,
            [5u64.wrapping_sub(2 * C), MAX, MAX, MAX],
        );
        // the greatest such difference: (C - 1) - (2^256 - 1) = C - 2^256, which is zero
        sub_takes([C - 2, 0, 0, 0], [MAX; 4], wrap, [P0 - 1, MAX, MAX, MAX]);
        sub_takes(
            [0; 4],
            [P0 + 1, MAX, MAX, MAX],
            wrap,
            [P0 - 1, MAX, MAX, MAX],
        );
    }

    /// The branches of a subtraction of two and of the cold borrows
    const OF_SUB_TWO: &[Branch] = &[
        Branch::SubTwoBorrow,
        Branch::SubTwoBorrows,
        Branch::SubTwoSecondBorrow,
        Branch::BorrowFromThird,
        Branch::BorrowFromFourth,
        Branch::BorrowWrap,
    ];

    /// `a - b - c` takes exactly the branches `expected`, and is the difference, with the limbs
    /// `limbs`, whichever of the two is subtracted first; and so do two subtractions give it
    #[track_caller]
    fn sub_two_takes(a: [u64; 4], b: [u64; 4], c: [u64; 4], expected: &[Branch], limbs: [u64; 4]) {
        let p = modulus();
        let reference = (integer(&a) % &p + &p * 2u32 - integer(&b) % &p - integer(&c) % &p) % &p;
        for (b, c) in [(b, c), (c, b)] {
            let mut r = FieldElement4x64(a);
            takes(OF_SUB_TWO, expected, || {
                r.sub_two_in_place(&FieldElement4x64(b), &FieldElement4x64(c))
            });
            assert_eq!(r.0, limbs);
            assert_eq!(integer(&r.0) % &p, reference);
            let mut twice = FieldElement4x64(a);
            twice.sub_in_place(&FieldElement4x64(b));
            twice.sub_in_place(&FieldElement4x64(c));
            assert_eq!(integer(&twice.0) % &p, reference);
        }
    }

    #[test]
    fn sub_of_two_without_a_borrow() {
        sub_two_takes([9, 9, 9, 9], [1, 2, 3, 4], [5, 4, 3, 2], &[], [3, 3, 3, 3]);
        // borrows between the limbs, two into a limb, are not ones of 2^256
        sub_two_takes(
            [0, 0, 0, 9],
            [1, 0, 0, 0],
            [1, 0, 0, 0],
            &[],
            [MAX - 1, MAX, MAX, 8],
        );
        sub_two_takes(
            [0, 0, 0, 9],
            [MAX, 0, 0, 0],
            [MAX, 0, 0, 0],
            &[],
            [2, MAX - 1, MAX, 8],
        );
        sub_two_takes(
            [0, 0, 0, 9],
            [MAX, MAX, MAX, 0],
            [MAX, MAX, MAX, 0],
            &[],
            [2, 0, 0, 7],
        );
        sub_two_takes([MAX; 4], [MAX; 4], [0; 4], &[], [0; 4]);
    }

    #[test]
    fn sub_of_two_with_one_borrow() {
        let one = &[Branch::SubTwoBorrow];
        sub_two_takes([2 * C, 0, 0, 0], [0, 0, 0, 1], [0; 4], one, [C, 0, 0, MAX]);
        sub_two_takes(
            [2 * C, 0, 0, 1],
            [0, 0, 0, 1],
            [0, 0, 0, 1],
            one,
            [C, 0, 0, MAX],
        );
        // the lowest limb gives C down to its last value
        sub_two_takes(
            [C + 1, 0, 0, 0],
            [1, 0, 0, 0],
            [0, 0, 0, 1],
            one,
            [0, 0, 0, MAX],
        );
        sub_two_takes(
            [0; 4],
            [1, 0, 0, 0],
            [1, 0, 0, 0],
            one,
            [P0 - 2, MAX, MAX, MAX],
        );
    }

    /// The difference is below `-2^256`: for two subtrahends above the half of it together
    #[test]
    fn sub_of_two_with_two_borrows() {
        let two = &[Branch::SubTwoBorrow, Branch::SubTwoBorrows];
        // 3 C - 2^255 - (2^255 + 2^192) = 3 C - 2^192 - 2^256
        sub_two_takes(
            [3 * C, 0, 0, 0],
            [0, 0, 0, HALF],
            [0, 0, 0, HALF + 1],
            two,
            [C, 0, 0, MAX],
        );
        // the lowest limb gives 2 C down to its last value
        sub_two_takes(
            [2 * C, 0, 0, 0],
            [0, 0, 0, HALF],
            [0, 0, 0, HALF + 1],
            two,
            [0, 0, 0, MAX],
        );
        // 2^192 - (2^256 - 1) - (2^256 - 1) = 2^192 + 2 - 2 * 2^256 = 2^192 + 2 - 2 C
        sub_two_takes(
            [0, 0, 0, 1],
            [MAX; 4],
            [MAX; 4],
            &[
                Branch::SubTwoBorrow,
                Branch::SubTwoBorrows,
                Branch::SubTwoSecondBorrow,
                Branch::BorrowFromThird,
                Branch::BorrowFromFourth,
            ],
            [2u64.wrapping_sub(2 * C), MAX, MAX, 0],
        );
    }

    #[test]
    fn sub_of_two_whose_fold_borrows_from_the_lowest_limb() {
        // one borrow of 2^256, and the lowest limb is below C
        sub_two_takes(
            [5, 9, 0, 0],
            [0, 0, 0, 1],
            [0; 4],
            &[Branch::SubTwoBorrow, Branch::SubTwoSecondBorrow],
            [5u64.wrapping_sub(C), 8, 0, MAX],
        );
        // two, and it is below 2 C but not below C: the first value that borrows, and the last
        let second = &[
            Branch::SubTwoBorrow,
            Branch::SubTwoBorrows,
            Branch::SubTwoSecondBorrow,
        ];
        sub_two_takes(
            [2 * C - 1, 9, 0, 0],
            [0, 0, 0, HALF],
            [0, 0, 0, HALF + 1],
            second,
            [MAX, 8, 0, MAX],
        );
        sub_two_takes(
            [0, 9, 0, 0],
            [0, 0, 0, HALF],
            [0, 0, 0, HALF + 1],
            second,
            [(2 * C).wrapping_neg(), 8, 0, MAX],
        );
    }

    #[test]
    fn sub_of_two_whose_second_borrow_ripples() {
        let through_one = &[
            Branch::SubTwoBorrow,
            Branch::SubTwoBorrows,
            Branch::SubTwoSecondBorrow,
            Branch::BorrowFromThird,
        ];
        sub_two_takes(
            [5, 0, 9, 0],
            [0, 0, 0, HALF],
            [0, 0, 0, HALF + 1],
            through_one,
            [5u64.wrapping_sub(2 * C), MAX, 8, MAX],
        );
        let through_two = &[
            Branch::SubTwoBorrow,
            Branch::SubTwoBorrows,
            Branch::SubTwoSecondBorrow,
            Branch::BorrowFromThird,
            Branch::BorrowFromFourth,
        ];
        sub_two_takes(
            [5, 0, 0, 0],
            [0, 0, 0, HALF],
            [0, 0, 0, HALF + 1],
            through_two,
            [5u64.wrapping_sub(2 * C), MAX, MAX, MAX - 1],
        );
    }

    /// The difference is below `2 C - 2 * 2^256` (or `C - 2^256`): only with a subtrahend
    /// above `p`
    #[test]
    fn sub_of_two_whose_second_borrow_wraps_past_two_to_the_256() {
        let wrap = &[
            Branch::SubTwoBorrow,
            Branch::SubTwoBorrows,
            Branch::SubTwoSecondBorrow,
            Branch::BorrowFromThird,
            Branch::BorrowFromFourth,
            Branch::BorrowWrap,
        ];
        // 0 - (2^256 - 1) - (2^256 - 1) = 2 - 2 * 2^256 = 2 - 2 C, and -2 C + C more to wrap
        sub_two_takes(
            [0; 4],
            [MAX; 4],
            [MAX; 4],
            wrap,
            [2u64.wrapping_sub(3 * C), MAX, MAX, MAX],
        );
        // the greatest such difference: 2 C - 1 - 2 * 2^256, which is -1
        sub_two_takes(
            [2 * C - 3, 0, 0, 0],
            [MAX; 4],
            [MAX; 4],
            wrap,
            [P0 - 1, MAX, MAX, MAX],
        );
        // and with one borrow: 0 - (2^256 - 5) - 0 = 5 - C
        sub_two_takes(
            [0; 4],
            [MAX - 4, MAX, MAX, MAX],
            [0; 4],
            &[
                Branch::SubTwoBorrow,
                Branch::SubTwoSecondBorrow,
                Branch::BorrowFromThird,
                Branch::BorrowFromFourth,
                Branch::BorrowWrap,
            ],
            [5u64.wrapping_sub(2 * C), MAX, MAX, MAX],
        );
    }

    #[test]
    fn negation_takes_the_borrows_of_the_subtraction() {
        let negate = |a: [u64; 4], expected: &[Branch], limbs: [u64; 4]| {
            let p = modulus();
            let mut r = FieldElement4x64(a);
            takes(OF_SUB, expected, || r.negate_in_place(1));
            assert_eq!(r.0, limbs);
            assert_eq!(integer(&limbs) % &p, (&p - integer(&a) % &p) % &p);
        };
        negate([0; 4], &[], [0; 4]);
        negate([1, 0, 0, 0], &[Branch::SubBorrow], [P0 - 1, MAX, MAX, MAX]);
        // -p is zero, in its canonical form
        negate(P, &[Branch::SubBorrow], [0; 4]);
        negate(
            [MAX, 0, 0, 0],
            &[Branch::SubBorrow, Branch::SubSecondBorrow],
            [1u64.wrapping_sub(C), MAX - 1, MAX, MAX],
        );
        negate(
            [MAX, MAX, 0, 0],
            &[
                Branch::SubBorrow,
                Branch::SubSecondBorrow,
                Branch::BorrowFromThird,
            ],
            [1u64.wrapping_sub(C), MAX, MAX - 1, MAX],
        );
        negate(
            [MAX, MAX, MAX, 0],
            &[
                Branch::SubBorrow,
                Branch::SubSecondBorrow,
                Branch::BorrowFromThird,
                Branch::BorrowFromFourth,
            ],
            [1u64.wrapping_sub(C), MAX, MAX, MAX - 1],
        );
        // -(p + 1) = p - 1: the operands above p wrap
        negate(
            [P0 + 1, MAX, MAX, MAX],
            &[
                Branch::SubBorrow,
                Branch::SubSecondBorrow,
                Branch::BorrowFromThird,
                Branch::BorrowFromFourth,
                Branch::BorrowWrap,
            ],
            [P0 - 1, MAX, MAX, MAX],
        );
    }

    /// `a * b` takes exactly the branches `expected` in its fold, and is the product; a square
    /// when the operands are equal, by both routines
    #[track_caller]
    fn mul_takes(a: [u64; 4], b: [u64; 4], expected: &[Branch]) -> [u64; 4] {
        let p = modulus();
        let product = integer(&a) * integer(&b) % &p;
        let mut r = FieldElement4x64(a);
        takes(OF_REDUCE, expected, || r.mul_in_place(&FieldElement4x64(b)));
        assert_eq!(integer(&r.0) % &p, product);
        let mut commuted = FieldElement4x64(b);
        takes(OF_REDUCE, expected, || {
            commuted.mul_in_place(&FieldElement4x64(a))
        });
        assert_eq!(commuted.0, r.0);
        if a == b {
            let mut squared = FieldElement4x64(a);
            takes(OF_REDUCE, expected, || squared.square_in_place());
            assert_eq!(squared.0, r.0);
        }
        r.0
    }

    #[test]
    fn mul_whose_fold_carries_nothing() {
        assert_eq!(mul_takes([3, 0, 0, 0], [5, 0, 0, 0], &[]), [15, 0, 0, 0]);
        assert_eq!(mul_takes([0; 4], [MAX; 4], &[]), [0; 4]);
        // 2^128 * 2^128 = C
        assert_eq!(mul_takes([0, 0, 1, 0], [0, 0, 1, 0], &[]), [C, 0, 0, 0]);
    }

    /// For every fourth pair of uniform operands
    #[test]
    fn mul_whose_fold_carries_into_the_second_limb() {
        let fold = &[Branch::ReduceFoldCarry];
        // (p - 1)^2 = 1, in its other form: p + 1
        assert_eq!(
            mul_takes([P0 - 1, MAX, MAX, MAX], [P0 - 1, MAX, MAX, MAX], fold),
            [P0 + 1, MAX, MAX, MAX]
        );
        mul_takes([1, 2, 3, MAX], [1, 2, 3, MAX], fold);
        mul_takes([MAX, 0, 0, MAX], [977, 0, 0, MAX], fold);
    }

    /// The products whose sum `low + C high` has a second limb of `2^64 - 2` or more under a
    /// fold that carries: residues with a second limb of 0 or 1 over a small lowest limb. The
    /// carry goes on into the third limb, which takes it here.
    #[test]
    fn mul_whose_fold_overflows_the_second_limb() {
        let second = &[
            Branch::ReduceFoldCarry,
            Branch::ReduceSecondCarry,
            Branch::CarryIntoThird,
        ];
        mul_takes(
            [
                0x3313_8131_c541_013d,
                0xeb8a_c8ce_8a24_5e6b,
                0x8c5f_e8f8_dc3b_f364,
                0x678a_5aa3_3b6f_e507,
            ],
            [
                0x7398_3c7c_3ec6_5538,
                0xd0ae_efc6_6994_b795,
                0xe0dc_08de_3de2_c4d4,
                0xf6a4_9506_9ddd_1032,
            ],
            second,
        );
        // (p - 2^64)^2 = 2^128, (p - 2^192)^2 = 2^384 = C 2^128
        assert_eq!(
            mul_takes([P0, MAX - 1, MAX, MAX], [P0, MAX - 1, MAX, MAX], second),
            [0, 0, 1, 0]
        );
        assert_eq!(
            mul_takes([P0, MAX, MAX, MAX - 1], [P0, MAX, MAX, MAX - 1], second),
            [0, 0, C, 0]
        );
    }

    #[test]
    fn mul_whose_second_carry_ripples_on_into_the_fourth_limb() {
        let ripple = &[
            Branch::ReduceFoldCarry,
            Branch::ReduceSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
        ];
        assert_eq!(mul_takes([0, 0, 0, 1], [MAX; 4], ripple), [0, 0, 0, C - 1]);
        mul_takes(
            [
                0x25b7_501a_c9c1_ffef,
                0x2001_2170_d418_f7af,
                0x1d5c_4825_5745_0e65,
                0x9660_5d95_9d7c_d4f6,
            ],
            [
                0x2e9d_297c_5343_b8d9,
                0xe9a2_0ae5_9600_6181,
                0x05dc_8b16_13e0_bcfc,
                0x5535_7992_e383_af55,
            ],
            ripple,
        );
    }

    /// The products with a residue from `C` up to the fold of their sum: the sum is less than
    /// that fold below a multiple of `2^256`
    #[test]
    fn mul_whose_second_carry_wraps_past_two_to_the_256() {
        let wrap = &[
            Branch::ReduceFoldCarry,
            Branch::ReduceSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
            Branch::CarryWrap,
        ];
        // a b = C for an arbitrary a
        assert_eq!(
            mul_takes(
                [
                    0x91b7_584a_2265_b1f5,
                    0xcd61_3e30_d8f1_6adf,
                    0x1027_c4d1_c386_bbc4,
                    0x1e2f_eb89_414c_343c,
                ],
                [
                    0x0f29_185e_2385_c192,
                    0x1e69_1cd6_04a4_35e4,
                    0x4184_e57c_0a3d_ef44,
                    0x7a71_3032_511f_ad70,
                ],
                wrap,
            ),
            [C, 0, 0, 0]
        );
        // (p - 2^128)^2 = 2^256 = C, (2^256 - 1)^2 = (C - 1)^2
        assert_eq!(
            mul_takes([P0, MAX, MAX - 1, MAX], [P0, MAX, MAX - 1, MAX], wrap),
            [C, 0, 0, 0]
        );
        assert_eq!(
            mul_takes([MAX; 4], [MAX; 4], wrap),
            [(C - 1).wrapping_mul(C - 1), 1, 0, 0]
        );
    }

    /// And a residue of `2^64 - C` or more besides, which takes a fold of almost `2^64`:
    /// operands within `2^234` of `p`
    #[test]
    fn mul_whose_wrap_carries_out_of_the_lowest_limb() {
        let wrap_low = &[
            Branch::ReduceFoldCarry,
            Branch::ReduceSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
            Branch::CarryWrap,
            Branch::CarryWrapLow,
        ];
        // (p - 2^32)^2 = 2^64
        let a = [P0 - (1 << 32), MAX, MAX, MAX];
        assert_eq!(mul_takes(a, a, wrap_low), [0, 1, 0, 0]);
        // (p - 2^33) (p - 2^31) = 2^64, (p - 3) (p - (2^64 + 2) / 3) = 2^64 + 2
        mul_takes(
            [P0 - (1 << 33), MAX, MAX, MAX],
            [P0 - (1 << 31), MAX, MAX, MAX],
            wrap_low,
        );
        assert_eq!(
            mul_takes(
                [P0 - 3, MAX, MAX, MAX],
                [P0 - 0x5555_5555_5555_5556, MAX, MAX, MAX],
                wrap_low,
            ),
            [2, 1, 0, 0]
        );
    }

    /// `a * small` takes exactly the branches `expected`, and is the multiple, with the limbs
    /// `limbs`; three times by the tripling too
    #[track_caller]
    fn small_takes(a: [u64; 4], small: u32, expected: &[Branch], limbs: [u64; 4]) {
        let mut r = FieldElement4x64(a);
        takes(OF_SMALL, expected, || r.mul_int_in_place(small));
        assert_eq!(r.0, limbs);
        assert_eq!(integer(&limbs) % modulus(), integer(&a) * small % modulus());
        if small == 3 {
            let mut r = FieldElement4x64(a);
            takes(OF_SMALL, expected, || r.triple_in_place());
            assert_eq!(r.0, limbs);
        }
    }

    #[test]
    fn small_multiples_without_a_top_limb() {
        small_takes([1, 2, 3, 4], 3, &[], [3, 6, 9, 12]);
        small_takes([MAX, MAX, MAX, MAX], 0, &[], [0; 4]);
        small_takes([MAX, MAX, MAX, MAX], 1, &[], [MAX; 4]);
        small_takes([HALF, 0, 0, 0], 2, &[], [0, 1, 0, 0]);
    }

    #[test]
    fn small_multiples_that_fold_their_top_limb() {
        let fold = &[Branch::SmallFold];
        small_takes([0, 0, 0, HALF], 2, fold, [C, 0, 0, 0]);
        small_takes([5, 0, 0, HALF], 3, fold, [15 + C, 0, 0, HALF]);
        // the fold carries into the second limb, which takes it
        small_takes([MAX, 0, 0, MAX], 3, fold, [C * 2 - 3, 3, 0, MAX - 2]);
        // a fold of two limbs
        let mut r = FieldElement4x64([MAX, 0, 0, MAX]);
        takes(OF_SMALL, fold, || r.mul_int_in_place(u32::MAX));
        assert_eq!(
            integer(&r.0) % modulus(),
            integer(&[MAX, 0, 0, MAX]) * u32::MAX % modulus()
        );
    }

    #[test]
    fn small_multiples_whose_fold_overflows_the_second_limb() {
        // the carry goes on into the third limb, which takes it
        let second = &[
            Branch::SmallFold,
            Branch::SmallSecondCarry,
            Branch::CarryIntoThird,
        ];
        small_takes([MAX, MAX, 0, MAX], 3, second, [2 * C - 3, 0, 3, MAX - 2]);
        // or passes it on into the fourth
        let ripple = &[
            Branch::SmallFold,
            Branch::SmallSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
        ];
        small_takes(
            [MAX, MAX, MAX, MAX - 1],
            3,
            ripple,
            [2 * C - 3, 0, 0, MAX - 2],
        );
    }

    #[test]
    fn small_multiples_whose_second_carry_wraps_past_two_to_the_256() {
        let wrap = &[
            Branch::SmallFold,
            Branch::SmallSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
            Branch::CarryWrap,
        ];
        // 3 (2^256 - 1) = 3 (C - 1), and likewise
        small_takes([MAX; 4], 3, wrap, [3 * C - 3, 0, 0, 0]);
        small_takes([MAX; 4], 2, wrap, [2 * C - 2, 0, 0, 0]);
        small_takes(
            [MAX; 4],
            u32::MAX,
            wrap,
            [(u32::MAX as u64).wrapping_mul(C - 1), 1, 0, 0],
        );
        // and with a lowest limb that carries once more: the multiple of C - 1 next above 2^64
        let wrap_low = &[
            Branch::SmallFold,
            Branch::SmallSecondCarry,
            Branch::CarryIntoThird,
            Branch::CarryIntoFourth,
            Branch::CarryWrap,
            Branch::CarryWrapLow,
        ];
        let small = (1u64 << 32) - 975;
        small_takes(
            [MAX; 4],
            small as u32,
            wrap_low,
            [small.wrapping_mul(C - 1), 1, 0, 0],
        );
    }

    /// The candidates for zero and one, by their lowest limb, and the canonical form
    #[test]
    fn comparisons_and_the_canonical_form_take_their_branches() {
        let of = &[Branch::ZeroCandidate, Branch::NotCanonical];
        let zero = |a: [u64; 4], expected: &[Branch]| {
            takes(of, expected, || FieldElement4x64(a).normalizes_to_zero())
        };
        assert!(!zero([7, 0, 0, 0], &[]));
        assert!(zero([0; 4], &[Branch::ZeroCandidate]));
        assert!(zero(P, &[Branch::ZeroCandidate]));
        assert!(!zero([0, 0, 1, 0], &[Branch::ZeroCandidate]));
        assert!(!zero([0, MAX, MAX, MAX], &[Branch::ZeroCandidate]));
        assert!(!zero([P0, MAX, MAX - 1, MAX], &[Branch::ZeroCandidate]));
        assert!(!zero([P0, 0, 0, 0], &[Branch::ZeroCandidate]));

        let one =
            |a: [u64; 4], expected: &[Branch]| takes(of, expected, || FieldElement4x64(a).is_one());
        assert!(!one([7, 0, 0, 0], &[]));
        assert!(one([1, 0, 0, 0], &[Branch::ZeroCandidate]));
        assert!(one([P0 + 1, MAX, MAX, MAX], &[Branch::ZeroCandidate]));
        assert!(!one([1, 1, 0, 0], &[Branch::ZeroCandidate]));
        assert!(!one([P0 + 1, MAX, 0, MAX], &[Branch::ZeroCandidate]));

        let normalize = |a: [u64; 4], expected: &[Branch]| {
            let mut r = FieldElement4x64(a);
            takes(of, expected, || r.normalize_in_place());
            assert_eq!(integer(&r.0), integer(&a) % modulus());
            r.0
        };
        assert_eq!(
            normalize([P0 - 1, MAX, MAX, MAX], &[]),
            [P0 - 1, MAX, MAX, MAX]
        );
        assert_eq!(normalize(P, &[Branch::NotCanonical]), [0; 4]);
        assert_eq!(
            normalize([MAX; 4], &[Branch::NotCanonical]),
            [C - 1, 0, 0, 0]
        );
        assert_eq!(
            normalize([MAX, MAX, MAX - 1, MAX], &[]),
            [MAX, MAX, MAX - 1, MAX]
        );
        // the parity and the bytes are those of the canonical form
        assert!(takes(of, &[Branch::NotCanonical], || {
            FieldElement4x64([P0 + 1, MAX, MAX, MAX]).is_odd()
        }));
        assert!(!takes(of, &[], || FieldElement4x64([
            P0 - 1,
            MAX,
            MAX,
            MAX
        ])
        .is_odd()));
        let mut bytes = [0xffu8; 32];
        takes(of, &[Branch::NotCanonical], || {
            FieldElement4x64([P0 + 5, MAX, MAX, MAX]).write_bytes(&mut bytes)
        });
        let mut five = [0u8; 32];
        five[31] = 5;
        assert_eq!(bytes, five);
    }

    /// Every operation on `a` and `b`, against big integers
    fn check_pair(a: &FieldElement4x64, b: &FieldElement4x64) {
        let p = modulus();
        let (ia, ib) = (residue(a), residue(b));

        let mut r = *a;
        r.mul_in_place(b);
        assert_eq!(residue(&r), &ia * &ib % &p, "{:x?} * {:x?}", a.0, b.0);

        let mut r = *a;
        r.add_in_place(b);
        assert_eq!(residue(&r), (&ia + &ib) % &p, "{:x?} + {:x?}", a.0, b.0);

        let mut r = *a;
        r.sub_in_place(b);
        assert_eq!(
            residue(&r),
            (&ia + &p - &ib) % &p,
            "{:x?} - {:x?}",
            a.0,
            b.0
        );

        let mut r = *a;
        r.sub_and_negate_in_place(b);
        assert_eq!(
            residue(&r),
            (&ib + &p - &ia) % &p,
            "{:x?} - {:x?}",
            b.0,
            a.0
        );

        // a - b - b, a - b - a and a - a - b
        for (first, second) in [(b, b), (b, a), (a, b)] {
            let mut r = *a;
            r.sub_two_in_place(first, second);
            assert_eq!(
                residue(&r),
                (&ia + &p * 2u32 - residue(first) - residue(second)) % &p,
                "{:x?} - {:x?} - {:x?}",
                a.0,
                first.0,
                second.0
            );
        }

        assert_eq!(a == b, ia == ib);
        assert_eq!(a.is_equal(b), ia == ib);
        assert!(a.is_equal(a));
    }

    /// Every operation on `a`, against big integers; the inversions if `inversions`
    fn check_one(a: &FieldElement4x64, inversions: bool) {
        let p = modulus();
        let ia = residue(a);

        let mut r = *a;
        r.square_in_place();
        assert_eq!(residue(&r), &ia * &ia % &p, "{:x?}^2", a.0);

        let mut r = *a;
        r.double_in_place();
        assert_eq!(residue(&r), &ia * 2u32 % &p);

        let mut r = *a;
        r.triple_in_place();
        assert_eq!(residue(&r), &ia * 3u32 % &p);

        for small in [0, 1, 2, 3, 7, 8, 977, 0xffff_fc31, u32::MAX - 1, u32::MAX] {
            let mut r = *a;
            r.mul_int_in_place(small);
            assert_eq!(residue(&r), &ia * small % &p, "{:x?} * {small}", a.0);
            let mut r = *a;
            r.add_int_in_place(small);
            assert_eq!(residue(&r), (&ia + small) % &p, "{:x?} + {small}", a.0);
        }

        let mut r = *a;
        r.negate_in_place(1);
        assert_eq!(residue(&r), (&p - &ia) % &p, "-{:x?}", a.0);

        // the canonical form, and what reads it
        let mut r = *a;
        r.normalize_in_place();
        assert_eq!(integer(&r.0), ia);
        assert_eq!(a.is_odd(), ia.bit(0));
        let mut expected = [0u8; 32];
        let digits = ia.to_bytes_be();
        expected[32 - digits.len()..].copy_from_slice(&digits);
        assert_eq!(&*a.to_bytes(), &expected);
        let mut written = [0xffu8; 32];
        a.write_bytes(&mut written);
        assert_eq!(written, expected);
        assert_eq!(
            FieldElement4x64::from_bytes(&expected).map(|x| x.0),
            Some(r.0)
        );

        // the comparisons are modulo p
        assert_eq!(a.normalizes_to_zero(), ia == BigUint::ZERO);
        assert_eq!(a.is_one(), ia == BigUint::from(1u32));
        assert_eq!(a.is_minus_one(), ia == &p - 1u32);

        if inversions {
            // both inversions, zero for zero
            let expected = ia.modpow(&(&p - 2u32), &p);
            let mut r = *a;
            r.invert_in_place();
            assert_eq!(residue(&r), expected, "1 / {:x?}", a.0);
            let mut r = *a;
            r.invert_by_exponentiation_in_place();
            assert_eq!(residue(&r), expected, "1 / {:x?} by exponentiation", a.0);
        }
    }

    /// The representatives where the carries and the two forms of a residue are: around `0`,
    /// `C`, `2^64`, `p` and `2^256`, and limbs of all ones
    fn edges() -> Vec<FieldElement4x64> {
        let mut edges = Vec::new();
        for low in [0, 1, 2, C - 1, C, C + 1, 2 * C, MAX - C, MAX - 1, MAX] {
            edges.push([low, 0, 0, 0]);
            edges.push([low, 1, 0, 0]);
            edges.push([low, MAX, 0, 0]);
            edges.push([low, 0, 0, HALF]);
            edges.push([low, MAX, MAX, MAX >> 1]);
        }
        // p - C - 1 ..= p + C - 1 = 2^256 - 1, in steps that cover p - 2 ..= p + 2 and the ends
        for below in [
            0,
            1,
            2,
            C - 3,
            C - 2,
            C - 1,
            C,
            C + 1,
            C + 2,
            2 * C - 1,
            2 * C,
            2 * C + 1,
        ] {
            edges.push([MAX - below, MAX, MAX, MAX]);
            edges.push([MAX - below, MAX, MAX, MAX - 1]);
            edges.push([MAX - below, 0, MAX, MAX]);
        }
        edges.push([0, 0, 0, MAX]);
        edges.push([0, MAX, MAX, MAX]);
        edges.push([0xffff_ffff, 0xffff_ffff_0000_0000, HALF, 1]);
        edges.into_iter().map(FieldElement4x64).collect()
    }

    /// The equality of elements: of the same limbs, of the two forms of a residue (the cold
    /// comparison of the canonical forms), and of different residues
    #[test]
    fn equality_is_modulo_p() {
        let of = &[Branch::EqualOtherwise];
        let equal = |a: [u64; 4], b: [u64; 4], expected: &[Branch]| {
            let (a, b) = (FieldElement4x64(a), FieldElement4x64(b));
            let result = takes(of, expected, || a.is_equal(&b));
            assert_eq!(takes(of, expected, || b.is_equal(&a)), result);
            assert_eq!(result, residue(&a) == residue(&b));
            result
        };
        assert!(equal([1, 2, 3, 4], [1, 2, 3, 4], &[]));
        assert!(equal([MAX; 4], [MAX; 4], &[]));
        assert!(equal(P, P, &[]));
        let otherwise = &[Branch::EqualOtherwise];
        // the two forms of zero, of one, and of the greatest residue that has two
        assert!(equal([0; 4], P, otherwise));
        assert!(equal([1, 0, 0, 0], [P0 + 1, MAX, MAX, MAX], otherwise));
        assert!(equal([C - 1, 0, 0, 0], [MAX; 4], otherwise));
        // different residues, in each limb and next to the two forms
        assert!(!equal([1, 2, 3, 4], [2, 2, 3, 4], otherwise));
        assert!(!equal([1, 2, 3, 4], [1, 3, 3, 4], otherwise));
        assert!(!equal([1, 2, 3, 4], [1, 2, 4, 4], otherwise));
        assert!(!equal([1, 2, 3, 4], [1, 2, 3, 5], otherwise));
        assert!(!equal([0; 4], [P0 + 1, MAX, MAX, MAX], otherwise));
        assert!(!equal([C, 0, 0, 0], [MAX; 4], otherwise));
        assert!(!equal([0; 4], [P0 - 1, MAX, MAX, MAX], otherwise));
        for a in edges() {
            for b in edges() {
                assert_eq!(a.is_equal(&b), residue(&a) == residue(&b));
                assert_eq!(
                    FieldElement4x64::from_canonical_limbs(a.0).is_some(),
                    integer(&a.0) < modulus()
                );
                assert_eq!(integer(&a.to_canonical_limbs()), residue(&a));
            }
        }
    }

    #[test]
    fn edge_values_match_big_integers() {
        let edges = edges();
        let (_, counts) = counted(|| {
            for a in &edges {
                check_one(a, true);
                for b in &edges {
                    check_pair(a, b);
                }
            }
        });
        // these values alone take every branch of the additions and the subtractions
        for branch in OF_ADD.iter().chain(OF_SUB) {
            if *branch != Branch::CarryWrapLow {
                assert!(counts[*branch as usize] > 0, "{branch:?}");
            }
        }
    }

    #[test]
    fn random_values_match_big_integers() {
        proptest!(|(a: FieldElement4x64, b: FieldElement4x64)| {
            check_one(&a, true);
            check_pair(&a, &b);
            check_pair(&b, &a);
        })
    }

    /// SplitMix64, with a fixed seed
    struct Random(u64);

    impl Random {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn uniform(&mut self) -> [u64; 4] {
            core::array::from_fn(|_| self.next())
        }

        /// Limbs that are boundary values as often as not, so that their combinations occur
        fn structured(&mut self) -> [u64; 4] {
            const BOUNDARIES: [u64; 10] = [
                0,
                1,
                C - 1,
                C,
                C + 1,
                MAX - C,
                MAX - C + 1,
                MAX - C + 2,
                MAX - 1,
                MAX,
            ];
            core::array::from_fn(|_| {
                let choice = self.next();
                if choice & 1 == 0 {
                    BOUNDARIES[(choice >> 1) as usize % BOUNDARIES.len()]
                } else {
                    self.next()
                }
            })
        }
    }

    /// Operands whose limbs mix the boundary values (`0`, `1`, `C - 1`, `C`, `C + 1`,
    /// `2^64 - C - 1`, `2^64 - C`, `2^64 - C + 1`, `2^64 - 2`, `2^64 - 1`) with uniform ones:
    /// every operation against big integers, and the branches they reach
    #[test]
    fn structured_random_values_match_big_integers() {
        let mut random = Random(0x5eed);
        let (_, counts) = counted(|| {
            for _ in 0..40_000 {
                let a = FieldElement4x64(random.structured());
                let b = FieldElement4x64(random.structured());
                check_one(&a, false);
                check_pair(&a, &b);
            }
        });
        // every branch but the last carry of a wrapped product, whose operands are within
        // 2^234 of p (see `mul_whose_wrap_carries_out_of_the_lowest_limb`)
        for (branch, what) in Branch::ALL {
            let count = counts[*branch as usize];
            std::println!("{count:>10}  {what}");
            assert!(count > 0 || *branch == Branch::CarryWrapLow, "{branch:?}");
        }
    }

    /// The frequencies of the branches on uniform operands: a carry or a borrow of `2^256` for
    /// every other pair, a fold that carries for every fourth product and every third square
    /// (the top of the sum is about `C a b / 2^512`), and none of the cold ones
    #[test]
    fn branch_frequencies_on_uniform_operands() {
        const PAIRS: u64 = 1_000_000;
        let mut random = Random(1);
        let (_, counts) = counted(|| {
            for _ in 0..PAIRS {
                let (a, b) = (random.uniform(), random.uniform());
                core::hint::black_box(add(&a, &b));
                core::hint::black_box(sub(&a, &b));
                core::hint::black_box(sub_two(&a, &b, &random.uniform()));
                let (low, high) = mul_wide(&a, &b);
                core::hint::black_box(reduce_wide(low, high));
                let (low, high) = square_wide(&a);
                core::hint::black_box(reduce_wide(low, high));
                core::hint::black_box(mul_small(&a, 3));
                core::hint::black_box(FieldElement4x64(a).normalizes_to_zero());
                core::hint::black_box(canonical(&a));
            }
        });
        std::println!("{PAIRS} uniform pairs:");
        for (branch, what) in Branch::ALL {
            std::println!("{:>10}  {what}", counts[*branch as usize]);
        }
        let count = |branch: Branch| counts[branch as usize];
        let about = |count: u64, expected: u64| count.abs_diff(expected) < expected / 50;
        assert_eq!(count(Branch::Add), PAIRS);
        assert!(about(count(Branch::AddCarry), PAIRS / 2));
        assert!(about(count(Branch::SubBorrow), PAIRS / 2));
        // a - b - c is not negative for a sixth of the operands, and below -2^256 for a sixth
        assert_eq!(count(Branch::SubTwo), PAIRS);
        assert!(about(count(Branch::SubTwoBorrow), 5 * PAIRS / 6));
        assert!(about(count(Branch::SubTwoBorrows), PAIRS / 6));
        assert_eq!(count(Branch::Reduce), 2 * PAIRS);
        assert!(about(count(Branch::ReduceFoldCarry), PAIRS / 4 + PAIRS / 3));
        // 3 a is a 2^256 or more for two thirds of the operands
        assert!(about(count(Branch::SmallFold), 2 * PAIRS / 3));
        for cold in [
            Branch::AddSecondCarry,
            Branch::SubSecondBorrow,
            Branch::SubTwoSecondBorrow,
            Branch::ReduceSecondCarry,
            Branch::SmallSecondCarry,
            Branch::CarryIntoThird,
            Branch::BorrowFromThird,
            Branch::ZeroCandidate,
            Branch::NotCanonical,
        ] {
            // the second carries have a probability of 2^-31 per operation
            assert!(count(cold) <= 3, "{cold:?}: {}", count(cold));
        }
    }

    /// The frequencies of the branches inside recoveries: the hinted recovery of the 64
    /// signatures of the cycle bench (`binbender/tools/ecrecover-cycles`), replayed from the
    /// recorded hints so that only the querier's arithmetic is counted, as the guest runs it
    #[cfg(all(secp256k1_field_4x64, feature = "secp256k1-static-context"))]
    #[test]
    fn branch_frequencies_in_recoveries() {
        use crate::hints::test_support::TapeOracle;
        use crate::hints::{HintTarget, RecordingHintOracle};
        use crate::k256::ecdsa::SigningKey;
        use crate::precompiles::secp256k1_ecrecover;
        use crate::sha256::Digest;

        const SIGNATURES: u32 = 64;
        fn hash(label: &str, index: u32) -> [u8; 32] {
            crate::sha256::Sha256::new()
                .chain_update(label)
                .chain_update(index.to_le_bytes())
                .finalize()
                .into()
        }

        let mut total = [0u64; branches::COUNT];
        for index in 0..SIGNATURES {
            let key = SigningKey::from_bytes(&hash("ecrecover-cycles key", index).into()).unwrap();
            let digest = hash("ecrecover-cycles digest", index);
            let (signature, recovery) = key.sign_prehash_recoverable(&digest).unwrap();
            let r: [u8; 32] = signature.r().to_bytes().into();
            let s: [u8; 32] = signature.s().to_bytes().into();
            let odd = recovery.is_y_odd();

            let mut recorder = RecordingHintOracle::recording_for(HintTarget::Native);
            let recorded = secp256k1_ecrecover(&digest, &r, &s, odd, Some(&mut recorder)).unwrap();
            let mut tape = TapeOracle::new(recorder.into_tape());
            let (replayed, counts) =
                counted(|| secp256k1_ecrecover(&digest, &r, &s, odd, Some(&mut tape)).unwrap());
            assert_eq!(replayed.0, recorded.0);
            for (total, count) in total.iter_mut().zip(counts) {
                *total += count;
            }
        }
        std::println!("{SIGNATURES} recoveries, per recovery:");
        for (branch, what) in Branch::ALL {
            let count = total[*branch as usize];
            std::println!("{:>10.2}  {what}", count as f64 / f64::from(SIGNATURES));
        }
        let count = |branch: Branch| total[branch as usize];
        let share = |part: Branch, of: Branch| count(part) as f64 / count(of) as f64;
        // the operands of the ladder are as good as uniform: a carry or a borrow of 2^256 for
        // about every other addition or subtraction, so they are folded without a branch
        assert!((0.35..0.65).contains(&share(Branch::AddCarry, Branch::Add)));
        assert!((0.35..0.65).contains(&share(Branch::SubBorrow, Branch::Sub)));
        assert!((0.7..0.95).contains(&share(Branch::SubTwoBorrow, Branch::SubTwo)));
        assert!((0.05..0.3).contains(&share(Branch::SubTwoBorrows, Branch::SubTwo)));
        assert!((0.15..0.35).contains(&share(Branch::ReduceFoldCarry, Branch::Reduce)));
        // and what is treated as rare does not happen
        for cold in [
            Branch::AddSecondCarry,
            Branch::SubSecondBorrow,
            Branch::SubTwoSecondBorrow,
            Branch::ReduceSecondCarry,
            Branch::SmallSecondCarry,
            Branch::EqualOtherwise,
            Branch::NotCanonical,
        ] {
            assert_eq!(count(cold), 0, "{cold:?}");
        }
        // The values that are zero where one is tested: the checks of the square root and of
        // the witnessed key, and the sum that the witness makes the point at infinity. (The
        // quotients are checked by an equality of limbs.)
        assert!(count(Branch::ZeroCandidate) <= 4 * u64::from(SIGNATURES));
    }

    #[test]
    fn non_canonical_bytes_are_rejected() {
        let bytes = |limbs: &[u64; 4]| -> [u8; 32] {
            let mut bytes = [0u8; 32];
            for (chunk, limb) in bytes
                .as_chunks_mut::<8>()
                .0
                .iter_mut()
                .zip(limbs.iter().rev())
            {
                *chunk = limb.to_be_bytes();
            }
            bytes
        };
        assert_eq!(FieldElement4x64::from_bytes(&bytes(&P)), None);
        assert_eq!(
            FieldElement4x64::from_bytes(&bytes(&[P0 + 1, MAX, MAX, MAX])),
            None
        );
        assert_eq!(FieldElement4x64::from_bytes(&[0xff; 32]), None);
        let below = [P0 - 1, MAX, MAX, MAX];
        assert_eq!(
            FieldElement4x64::from_bytes(&bytes(&below)).map(|x| x.0),
            Some(below)
        );
        // the unchecked constructor takes any integer as a representative
        assert!(FieldElement4x64::from_bytes_unchecked(&bytes(&P)).normalizes_to_zero());
    }

    #[test]
    fn storage_is_the_canonical_limbs() {
        proptest!(|(a: FieldElement4x64)| {
            let mut normalized = a;
            normalized.normalize_in_place();
            prop_assert!(!is_at_least_p(&normalized.0));
            prop_assert_eq!(normalized.to_storage().to_field_elem().0, normalized.0);
            // the portable constants repack into the same limbs
            let bytes: [u8; 32] = a.to_bytes().into();
            let portable = super::super::field_5x52::FieldElement5x52::from_bytes_unchecked(&bytes);
            prop_assert_eq!(
                FieldElement4x64::from_storage_5x52(portable.to_storage()).0,
                normalized.0
            );
        })
    }
}
