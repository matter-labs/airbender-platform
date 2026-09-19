//! Word-aligned 32-byte digest.
//!
//! Hash outputs travel through many copies (into caches, keys, comparisons). A plain `[u8; 32]`
//! has byte alignment, so on targets without unaligned word access every such copy is a byte
//! loop or a `memcpy` call. This type is aligned to a machine word, so it is moved word-wise.

use core::ops::{Deref, DerefMut};

#[repr(C, align(8))]
#[derive(Clone, Copy, Default)]
pub struct Bytes32(pub [u8; 32]);

const _: () = const {
    assert!(core::mem::size_of::<Bytes32>() == 32);
    assert!(core::mem::align_of::<Bytes32>() >= core::mem::align_of::<u32>());
};

impl Bytes32 {
    pub const ZERO: Self = Self([0u8; 32]);

    #[inline(always)]
    pub const fn new(inner: [u8; 32]) -> Self {
        Self(inner)
    }

    #[inline(always)]
    pub const fn as_array(&self) -> &[u8; 32] {
        &self.0
    }

    #[inline(always)]
    pub const fn into_array(self) -> [u8; 32] {
        self.0
    }

    #[inline(always)]
    fn as_words(&self) -> &[u32; 8] {
        // Safety: size and alignment are asserted above
        unsafe { &*(self as *const Self).cast::<[u32; 8]>() }
    }
}

impl PartialEq for Bytes32 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        // word-wise, without a `memcmp` call
        let mut diff = 0u32;
        for (a, b) in self.as_words().iter().zip(other.as_words().iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

impl Eq for Bytes32 {}

impl core::hash::Hash for Bytes32 {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // consistent with `PartialEq`: equal bytes hash equally
        self.0.hash(state)
    }
}

impl PartialEq<[u8; 32]> for Bytes32 {
    #[inline(always)]
    fn eq(&self, other: &[u8; 32]) -> bool {
        let mut diff = 0u8;
        for (a, b) in self.0.iter().zip(other.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

impl core::fmt::Debug for Bytes32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("0x")?;
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl Deref for Bytes32 {
    type Target = [u8; 32];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Bytes32 {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl core::borrow::Borrow<[u8; 32]> for Bytes32 {
    #[inline(always)]
    fn borrow(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AsRef<[u8]> for Bytes32 {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8; 32]> for Bytes32 {
    #[inline(always)]
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for Bytes32 {
    #[inline(always)]
    fn from(inner: [u8; 32]) -> Self {
        Self(inner)
    }
}

impl From<Bytes32> for [u8; 32] {
    #[inline(always)]
    fn from(value: Bytes32) -> Self {
        value.0
    }
}
