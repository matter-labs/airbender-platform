pub trait Secp256k1Hooks {
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
