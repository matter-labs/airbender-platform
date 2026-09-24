#[cfg(feature = "bigint_ops")]
use crate::secp256k1::field::FieldElement;
use crate::secp256k1::field::FieldStorage;

use super::Affine;

#[derive(Debug, Clone, Copy)]

pub struct AffineStorage {
    pub(super) x: FieldStorage,
    pub(super) y: FieldStorage,
}

impl AffineStorage {
    pub(crate) const DEFAULT: Self = Self {
        x: FieldStorage::DEFAULT,
        y: FieldStorage::DEFAULT,
    };

    #[allow(dead_code)]
    pub(crate) fn to_affine(self) -> Affine {
        Affine {
            x: self.x.to_field_elem(),
            y: self.y.to_field_elem(),
            infinity: false,
        }
    }

    /// The coordinates in place, with the delegated field (no conversion, no copy)
    #[cfg(feature = "bigint_ops")]
    #[inline(always)]
    pub(crate) fn coordinates(&self) -> (&FieldElement, &FieldElement) {
        (self.x.as_field_elem(), self.y.as_field_elem())
    }
}
