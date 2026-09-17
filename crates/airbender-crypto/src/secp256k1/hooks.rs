//! Hooks for overriding expensive secp256k1 field operations during EC recovery.
//!
//! See the [Secp256k1 Hooks](https://matter-labs.github.io/airbender-platform/latest/04-crypto-on-guest-and-host.html#secp256k1-hooks)
//! section of the book for usage details and examples.

pub trait Secp256k1Hooks {
    /// Tells that `fe_invert_and_assign` costs about as much as a few field multiplications
    /// (e.g. it takes the inverse as a hint and checks it), so the algorithms can use inversions
    /// where they otherwise do more multiplications to avoid them.
    const FE_INVERT_IS_CHEAP: bool = false;

    fn fe_sqrt_and_assign(&mut self, fe: &mut super::field::FieldElement) -> bool;
    fn fe_invert_and_assign(&mut self, fe: &mut super::field::FieldElement);
    fn scalar_invert_and_assign(&mut self, scalar: &mut super::scalars::Scalar);
}

pub struct DefaultSecp256k1Hooks;

impl Secp256k1Hooks for DefaultSecp256k1Hooks {
    #[inline(always)]
    fn fe_sqrt_and_assign(&mut self, fe: &mut super::field::FieldElement) -> bool {
        fe.sqrt_in_place()
    }

    #[inline(always)]
    fn fe_invert_and_assign(&mut self, fe: &mut super::field::FieldElement) {
        fe.invert_in_place()
    }

    #[inline(always)]
    fn scalar_invert_and_assign(&mut self, scalar: &mut super::scalars::Scalar) {
        scalar.invert_in_place()
    }
}
