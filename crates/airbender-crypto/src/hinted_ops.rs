//! Curve operations that take a field inversion from the caller instead of computing it, so
//! that a prover can use a hint (an inverse it checks with one multiplication) where the
//! exponentiation-based inversion would cost hundreds of multiplications.

use ark_ec::short_weierstrass::{Affine, Projective, SWCurveConfig};

use ark_ff::{Field, One, Zero};

/// `p` in affine coordinates, as `CurveGroup::into_affine`, with the inverse of `Z` supplied
/// by `inverse`. `inverse` is only called with a non-zero argument, for which it must return
/// `Some`.
pub fn to_affine_with_inverse<C: SWCurveConfig>(
    p: &Projective<C>,
    inverse: impl FnOnce(&C::BaseField) -> Option<C::BaseField>,
) -> Affine<C> {
    if p.z.is_zero() {
        return Affine::identity();
    }
    if p.z.is_one() {
        return Affine::new_unchecked(p.x, p.y);
    }
    let z_inv = inverse(&p.z).expect("a non-zero field element has an inverse");
    let z_inv_squared = z_inv.square();
    let x = p.x * z_inv_squared;
    let y = p.y * z_inv_squared * z_inv;
    Affine::new_unchecked(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::{CurveGroup, PrimeGroup};

    fn check<C: SWCurveConfig>() {
        // small multiples of the generator (a random point would cost a scalar multiplication on
        // the emulated delegations), with Z != 1 after the first addition
        let generator = Projective::<C>::generator();
        let mut p = generator;
        for _ in 0..6 {
            p += &generator;
            assert!(!p.z.is_one());
            let expected = p.into_affine();
            let hinted = to_affine_with_inverse(&p, |z| z.inverse());
            assert_eq!(expected, hinted);
        }
        let zero = Projective::<C>::zero();
        assert_eq!(
            zero.into_affine(),
            to_affine_with_inverse(&zero, |z| z.inverse())
        );
        assert_eq!(
            generator.into_affine(),
            to_affine_with_inverse(&generator, |_| panic!("Z = 1 needs no inversion"))
        );
    }

    #[test]
    fn bn254_g1_matches_into_affine() {
        check::<crate::bn254::curves::g1::Config>();
    }

    #[test]
    fn bls12_381_g1_matches_into_affine() {
        check::<crate::bls12_381::curves::g1::Config>();
    }
}
