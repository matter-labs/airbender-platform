//! Where the oracle finds an operand's parts in guest memory. A base field element is its
//! `HintEncoding::LIMBS` Montgomery limbs (`R = 2^(64 LIMBS)`, any representative); an extension
//! element its components in `HintEncoding` order; an affine point its coordinates and infinity
//! flag at the offsets below. The offsets are asserted at compile time on the guest.

/// Byte offsets of the coordinates and of the flag of the point at infinity of an affine
/// point, and its size in bytes (with padding)
#[derive(Clone, Copy, Debug)]
pub struct AffinePoint {
    pub x: usize,
    pub y: usize,
    pub infinity: usize,
    pub size: usize,
}

pub const BN254_G1_AFFINE: AffinePoint = AffinePoint {
    x: 0,
    y: 32,
    infinity: 64,
    size: 96,
};
pub const BN254_G2_AFFINE: AffinePoint = AffinePoint {
    x: 0,
    y: 64,
    infinity: 128,
    size: 160,
};
pub const BLS12_381_G1_AFFINE: AffinePoint = AffinePoint {
    x: 0,
    y: 64,
    infinity: 128,
    size: 160,
};

pub const BLS12_381_G2_AFFINE: AffinePoint = AffinePoint {
    x: 0,
    y: 128,
    infinity: 256,
    size: 288,
};
pub const BLS12_381_PAIR_G1: usize = 0;
pub const BLS12_381_PAIR_G2: usize = 160;
pub const BLS12_381_PAIR_SIZE: usize = 448;

/// Byte offsets of the points of a `(G1Affine, G2Affine)` pair of the bn254 pairing input, and
/// its size in bytes
pub const BN254_PAIR_G1: usize = 0;
pub const BN254_PAIR_G2: usize = 96;
pub const BN254_PAIR_SIZE: usize = 256;

/// Byte offsets of the scalar (its 4 Montgomery limbs, as a base field element) and of the
/// coordinates of the point of a `Bn254G1MulRequest`, and its size in bytes
pub const BN254_G1_MUL_SCALAR: usize = 0;
pub const BN254_G1_MUL_X: usize = 32;
pub const BN254_G1_MUL_Y: usize = 64;
pub const BN254_G1_MUL_SIZE: usize = 96;

#[cfg(all(target_arch = "riscv32", bigint_ops))]
const _: () = {
    use super::{Bls12381Pair, HintEncoding};
    use crate::{bls12_381, bn254};
    use core::mem::{offset_of, size_of};

    // base field elements are their Montgomery limbs
    assert!(offset_of!(bn254::Fq, 0) == 0);
    assert!(size_of::<bn254::Fq>() == 8 * <bn254::Fq as HintEncoding>::LIMBS);
    assert!(offset_of!(bls12_381::Fq, 0) == 0);
    assert!(size_of::<bls12_381::Fq>() == 8 * <bls12_381::Fq as HintEncoding>::LIMBS);

    macro_rules! assert_affine {
        ($point:ty, $layout:expr) => {
            assert!(offset_of!($point, x) == $layout.x && offset_of!($point, y) == $layout.y);
            assert!(offset_of!($point, infinity) == $layout.infinity);
            assert!(size_of::<$point>() == $layout.size);
        };
    }
    assert_affine!(bn254::G1Affine, BN254_G1_AFFINE);
    assert_affine!(bn254::G2Affine, BN254_G2_AFFINE);
    assert_affine!(bls12_381::G1Affine, BLS12_381_G1_AFFINE);

    assert_affine!(bls12_381::G2Affine, BLS12_381_G2_AFFINE);
    assert!(offset_of!(Bls12381Pair, g1) == BLS12_381_PAIR_G1);
    assert!(offset_of!(Bls12381Pair, g2) == BLS12_381_PAIR_G2);
    assert!(size_of::<Bls12381Pair>() == BLS12_381_PAIR_SIZE);

    #[cfg(feature = "hinted_operations")]
    {
        use super::Bn254G1MulRequest;
        assert!(offset_of!(bn254::Fr, 0) == 0 && size_of::<bn254::Fr>() == 32);
        assert!(offset_of!(Bn254G1MulRequest, scalar) == BN254_G1_MUL_SCALAR);
        assert!(offset_of!(Bn254G1MulRequest, x) == BN254_G1_MUL_X);
        assert!(offset_of!(Bn254G1MulRequest, y) == BN254_G1_MUL_Y);
        assert!(size_of::<Bn254G1MulRequest>() == BN254_G1_MUL_SIZE);
    }

    type Bn254Pair = (bn254::G1Affine, bn254::G2Affine);
    assert!(offset_of!(Bn254Pair, 0) == BN254_PAIR_G1);
    assert!(offset_of!(Bn254Pair, 1) == BN254_PAIR_G2);
    assert!(size_of::<Bn254Pair>() == BN254_PAIR_SIZE);
};
