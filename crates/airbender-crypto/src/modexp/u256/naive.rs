//! The `U256` without the bigint delegation: four `u64` limbs, low first, and straight-line
//! carry chains. On RV64 a limb product is a `mul` and a `mulhu`, and a carry a `sltu`.

use core::cmp::Ordering;
use core::mem::MaybeUninit;

#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct U256([u64; 4]);

impl Clone for U256 {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self(self.0)
    }

    #[inline(always)]
    fn clone_from(&mut self, source: &Self) {
        self.0 = source.0;
    }
}

impl core::fmt::Debug for U256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(self, f)
    }
}

impl core::fmt::LowerHex for U256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for word in self.as_limbs().iter().rev() {
            write!(f, "{word:016x}")?;
        }

        core::fmt::Result::Ok(())
    }
}

/// From the top limb down
impl Ord for U256 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        for (x, y) in self.0.iter().zip(&other.0).rev() {
            if x != y {
                return if x < y {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for U256 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `(low, high)` of `a * b + c + d`, which fits two limbs
#[inline(always)]
fn mac(a: u64, b: u64, c: u64, d: u64) -> (u64, u64) {
    let t = u128::from(a) * u128::from(b) + u128::from(c) + u128::from(d);
    (t as u64, (t >> 64) as u64)
}

/// `a + b + carry` and its carry
#[inline(always)]
fn adc(a: u64, b: u64, carry: bool) -> (u64, bool) {
    let (sum, c1) = a.overflowing_add(b);
    let (sum, c2) = sum.overflowing_add(u64::from(carry));
    (sum, c1 | c2)
}

/// `a - b - borrow` and its borrow
#[inline(always)]
fn sbb(a: u64, b: u64, borrow: bool) -> (u64, bool) {
    let (difference, b1) = a.overflowing_sub(b);
    let (difference, b2) = difference.overflowing_sub(u64::from(borrow));
    (difference, b1 | b2)
}

/// The eight limbs of `a * b`: four rows of four limb products
#[inline(always)]
fn widening_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 8] {
    let mut out = [0u64; 8];
    for j in 0..4 {
        let mut carry = 0;
        for i in 0..4 {
            (out[i + j], carry) = mac(a[i], b[j], out[i + j], carry);
        }
        out[j + 4] = carry;
    }
    out
}

/// The eight limbs of `a * a`: the six products `a_i a_j` (`i < j`) once and doubled, and the
/// four squares
#[inline(always)]
fn widening_square(a: &[u64; 4]) -> [u64; 8] {
    let mut out = [0u64; 8];
    for i in 0..3 {
        let mut carry = 0;
        for j in i + 1..4 {
            (out[i + j], carry) = mac(a[i], a[j], out[i + j], carry);
        }
        out[i + 4] = carry;
    }
    let (mut shifted_out, mut carry) = (0, false);
    for i in 0..4 {
        let low = (out[2 * i] << 1) | shifted_out;
        let high = (out[2 * i + 1] << 1) | (out[2 * i] >> 63);
        shifted_out = out[2 * i + 1] >> 63;
        // a_i^2 + low + carry fits two limbs
        let (low, square_high) = mac(a[i], a[i], low, u64::from(carry));
        out[2 * i] = low;
        (out[2 * i + 1], carry) = high.overflowing_add(square_high);
    }
    debug_assert!(!carry && shifted_out == 0);
    out
}

impl U256 {
    pub const ZERO: Self = Self([0; 4]);
    pub const ONE: Self = Self([1, 0, 0, 0]);

    pub const fn from_limbs(limbs: [u64; 4]) -> Self {
        Self(limbs)
    }

    /// Writes `source` to `dst`; unlike the delegated one, any source location works.
    ///
    /// # Safety
    /// `dst` must be valid for a write of `Self`.
    #[inline(always)]
    pub unsafe fn write_into_ptr_unchecked(dst: *mut Self, source: &Self) {
        // SAFETY: guaranteed by the caller
        unsafe {
            dst.write(Self(source.0));
        }
    }

    #[inline(always)]
    pub fn zero() -> Self {
        Self::ZERO
    }

    #[inline(always)]
    pub fn one() -> Self {
        Self::ONE
    }

    #[inline(always)]
    pub const fn as_limbs(&self) -> &[u64; 4] {
        &self.0
    }

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        (self.0[0] | self.0[1] | self.0[2] | self.0[3]) == 0
    }

    #[inline(always)]
    pub fn is_one(&self) -> bool {
        ((self.0[0] ^ 1) | self.0[1] | self.0[2] | self.0[3]) == 0
    }

    #[inline(always)]
    pub fn overflowing_add_assign(&mut self, rhs: &Self) -> bool {
        let mut carry = false;
        for (limb, other) in self.0.iter_mut().zip(&rhs.0) {
            (*limb, carry) = adc(*limb, *other, carry);
        }
        carry
    }

    #[inline(always)]
    pub fn overflowing_sub_assign(&mut self, rhs: &Self) -> bool {
        self.overflowing_sub_assign_with_borrow_propagation(rhs, false)
    }

    #[inline(always)]
    pub fn overflowing_sub_assign_with_borrow_propagation(
        &mut self,
        rhs: &Self,
        mut borrow: bool,
    ) -> bool {
        for (limb, other) in self.0.iter_mut().zip(&rhs.0) {
            (*limb, borrow) = sbb(*limb, *other, borrow);
        }
        borrow
    }

    /// `(self, high) = self * rhs`. `high` needs no content here (the delegated one reads the
    /// multiplicand from it).
    #[inline(always)]
    pub fn widening_mul_assign_into(&mut self, high: &mut Self, rhs: &Self) {
        let product = widening_mul(&self.0, &rhs.0);
        self.0.copy_from_slice(&product[..4]);
        high.0.copy_from_slice(&product[4..]);
    }

    /// `(self, high) = self * rhs`, the high half into a fresh slot
    #[inline(always)]
    pub fn widening_mul_assign_into_slot<'a>(
        &mut self,
        high: &'a mut MaybeUninit<Self>,
        rhs: &Self,
    ) -> &'a mut Self {
        let product = widening_mul(&self.0, &rhs.0);
        self.0.copy_from_slice(&product[..4]);
        high.write(Self([product[4], product[5], product[6], product[7]]))
    }

    /// `(self, high) = self * self`, the high half into a fresh slot
    #[inline(always)]
    pub fn widening_square_assign_into_slot<'a>(
        &mut self,
        high: &'a mut MaybeUninit<Self>,
    ) -> &'a mut Self {
        let square = widening_square(&self.0);
        self.0.copy_from_slice(&square[..4]);
        high.write(Self([square[4], square[5], square[6], square[7]]))
    }

    pub fn from_be_bytes(input: &[u8; 32]) -> Self {
        let (limbs, _) = input.as_chunks::<8>();
        Self([
            u64::from_be_bytes(limbs[3]),
            u64::from_be_bytes(limbs[2]),
            u64::from_be_bytes(limbs[1]),
            u64::from_be_bytes(limbs[0]),
        ])
    }

    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let (chunks, _) = bytes.as_chunks_mut::<8>();
        for (chunk, limb) in chunks.iter_mut().zip(self.0.iter().rev()) {
            *chunk = limb.to_be_bytes();
        }
        bytes
    }
}

impl From<ruint::aliases::U256> for U256 {
    #[inline(always)]
    fn from(value: ruint::aliases::U256) -> Self {
        Self(*value.as_limbs())
    }
}

impl From<U256> for ruint::aliases::U256 {
    fn from(value: U256) -> Self {
        Self::from_limbs(value.0)
    }
}
