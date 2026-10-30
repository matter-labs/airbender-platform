//! The `U256` without the bigint delegation, over `ruint`.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct U256(ruint::aliases::U256);

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

impl U256 {
    pub const ZERO: Self = Self(ruint::aliases::U256::ZERO);
    pub const ONE: Self = Self(ruint::aliases::U256::ONE);

    pub const fn from_limbs(limbs: [u64; 4]) -> Self {
        Self(ruint::aliases::U256::from_limbs(limbs))
    }

    /// Writes `source` to `dst`; unlike the delegated one, any source location works.
    ///
    /// # Safety
    /// `dst` must be valid for a write of `Self`.
    pub unsafe fn write_into_ptr_unchecked(dst: *mut Self, source: &Self) {
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
        self.0.as_limbs()
    }

    pub fn as_limbs_mut(&mut self) -> &mut [u64; 4] {
        unsafe { self.0.as_limbs_mut() }
    }

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[inline(always)]
    pub fn is_one(&self) -> bool {
        self.0 == ruint::aliases::U256::ONE
    }

    #[inline(always)]
    pub fn overflowing_add_assign(&mut self, rhs: &Self) -> bool {
        let (t, of) = self.0.overflowing_add(rhs.0);
        self.0 = t;

        of
    }

    #[inline(always)]
    pub fn overflowing_sub_assign(&mut self, rhs: &Self) -> bool {
        let (t, of) = self.0.overflowing_sub(rhs.0);
        self.0 = t;

        of
    }

    #[inline(always)]
    pub fn widening_mul_assign_into(&mut self, high: &mut Self, rhs: &Self) {
        let t: ruint::aliases::U512 = self.0.widening_mul(rhs.0);
        self.as_limbs_mut().copy_from_slice(&t.as_limbs()[0..4]);
        high.as_limbs_mut().copy_from_slice(&t.as_limbs()[4..8]);
    }

    #[inline(always)]
    pub fn overflowing_sub_assign_with_borrow_propagation(
        &mut self,
        rhs: &Self,
        borrow: bool,
    ) -> bool {
        let (t, of1) = self.0.overflowing_sub(rhs.0);
        let (t, of2) = t.overflowing_sub(ruint::aliases::U256::from(borrow as u64));
        self.0 = t;
        of1 | of2
    }

    pub fn from_be_bytes(input: &[u8; 32]) -> Self {
        Self(ruint::aliases::U256::from_be_bytes::<32>(*input))
    }

    pub fn to_be_bytes(&self) -> [u8; 32] {
        self.0.to_be_bytes()
    }
}

impl From<ruint::aliases::U256> for U256 {
    #[inline(always)]
    fn from(value: ruint::aliases::U256) -> Self {
        Self(value)
    }
}

impl From<U256> for ruint::aliases::U256 {
    fn from(value: U256) -> Self {
        value.0
    }
}
