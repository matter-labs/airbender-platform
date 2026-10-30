//! Hooks for overriding expensive secp256k1 field operations during EC recovery.
//!
//! See the [Secp256k1 Hooks](https://matter-labs.github.io/airbender-platform/latest/04-crypto-on-guest-and-host.html#secp256k1-hooks)
//! section of the book for usage details and examples.

/// What a prover witnesses for a key recovery instead of letting the verifier compute it
/// (see `recover_witnessed`): the recovered key, and four coefficients `(a, b, c, d)` with
/// `(a + b λ) r ≡ (c + d λ) s (mod n)` for the eigenvalue `λ` of the endomorphism, each of at
/// most `WITNESS_COEFFICIENT_BITS` bits in absolute value (a short vector of a lattice of
/// index `n` in `Z⁴`, so about 64 bits each). The verification is then a multi-scalar
/// multiplication whose variable-base scalars have about 64 bits instead of 128.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryWitness {
    /// The coordinates of the key, big-endian
    pub key_x: [u8; 32],
    pub key_y: [u8; 32],
    /// `a, b, c, d`
    pub coefficients: [i128; 4],
}

pub trait Secp256k1Hooks {
    /// Tells that `fe_invert_and_assign` costs about as much as a few field multiplications
    /// (e.g. it takes the inverse as a hint and checks it), so the algorithms can use inversions
    /// where they otherwise do more multiplications to avoid them.
    const FE_INVERT_IS_CHEAP: bool = false;

    /// Tells that `fe_divide` costs about as much as a field multiplication or two (e.g. it
    /// takes the quotient as a hint and checks it with one multiplication). The scalar
    /// multiplication then works in affine coordinates, where a group operation is a division
    /// (the slope) and two or three multiplications, instead of the 7 to 11 multiplications of
    /// the Jacobian formulas.
    const FE_DIVIDE_IS_CHEAP: bool = false;

    /// The witness of the recovery of the key of the signature `(r, s)` of `message` (reduced
    /// modulo the order) with the point `point` (`x = r`, the `y` of the recovery id), if the
    /// hook provides one: the recovery is then checked against it (`recover_witnessed`), with
    /// about a fifth fewer group operations, instead of computed. Only used with
    /// `FE_DIVIDE_IS_CHEAP`. A witness that does not check out panics, as a wrong hint does, so
    /// that it cannot change the result of the recovery; a hook must answer `None` when it has
    /// none (e.g. for a signature whose key is the point at infinity).
    fn recovery_witness(
        &mut self,
        point: &super::points::Affine,
        r: &super::scalars::Scalar,
        s: &super::scalars::Scalar,
        message: &super::scalars::Scalar,
    ) -> Option<RecoveryWitness> {
        let _ = (point, r, s, message);
        None
    }

    fn fe_sqrt_and_assign(&mut self, fe: &mut super::field::FieldElement) -> bool;
    fn fe_invert_and_assign(&mut self, fe: &mut super::field::FieldElement);
    fn scalar_invert_and_assign(&mut self, scalar: &mut super::scalars::Scalar);

    /// `fraction[0] / fraction[1]` into `quotient`, for a non-zero denominator. The numerator
    /// and the denominator are adjacent in memory, so that an oracle reads them as one operand.
    /// The fraction is scratch space of the hook (e.g. for the check of a hinted quotient): its
    /// value is unspecified afterwards.
    fn fe_divide<'a>(
        &mut self,
        fraction: &mut [super::field::FieldElement; 2],
        quotient: &'a mut core::mem::MaybeUninit<super::field::FieldElement>,
    ) -> &'a mut super::field::FieldElement {
        let quotient = quotient.write(fraction[1]);
        self.fe_invert_and_assign(quotient);
        quotient.mul_in_place(&fraction[0]);
        quotient
    }
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
