#![no_std]
#![no_main]

use airbender::crypto::k256::{
    ecdsa::{hazmat::bits2field, RecoveryId, Signature},
    elliptic_curve::ops::Reduce,
    Scalar, Secp256k1, U256,
};
use airbender::crypto::secp256k1::field::FieldElement;
use airbender::crypto::secp256k1::hooks::Secp256k1Hooks;
use airbender::crypto::secp256k1::recover_with_hooks;
use airbender::crypto::secp256k1::scalars;
use airbender::guest::read;

/// Performs ecrecover using host-provided hints for the expensive operations.
///
/// The host precomputes the results of field sqrt, field inverse, and scalar
/// inverse, and passes them as inputs. The guest checks each hint with a
/// single cheap operation (one multiply) instead of recomputing from scratch.
#[airbender::main]
fn main() -> u32 {
    let digest: [u8; 32] = read().expect("digest");
    let r: [u8; 32] = read().expect("r");
    let s: [u8; 32] = read().expect("s");
    let rec_id: u8 = read().expect("recovery_id");

    // Hints precomputed by the host.
    let sqrt_hint: [u8; 32] = read().expect("sqrt_hint");
    let scalar_inv_hint: [u8; 32] = read().expect("scalar_inv_hint");
    let fe_inv_hint: [u8; 32] = read().expect("fe_inv_hint");

    let signature = Signature::from_scalars(r, s).expect("invalid signature");
    let recovery_id = RecoveryId::try_from(rec_id).expect("invalid recovery id");
    let message =
        <Scalar as Reduce<U256>>::reduce_bytes(&bits2field::<Secp256k1>(&digest).expect("bits"));

    let mut hooks = PrecomputedHintHooks::new(sqrt_hint, scalar_inv_hint, fe_inv_hint);
    let pk = recover_with_hooks(&message, &signature, &recovery_id, &mut hooks)
        .expect("recovery failed");
    let compressed = pk.to_bytes();

    u32::from_be_bytes([compressed[1], compressed[2], compressed[3], compressed[4]])
}

/// Hooks backed by precomputed hints.
///
/// Each method parses a hint, verifies it with one cheap check, and assigns it.
/// If any hint is invalid, the verification assert will fire.
struct PrecomputedHintHooks {
    sqrt_hint: [u8; 32],
    scalar_inv_hint: [u8; 32],
    fe_inv_hint: [u8; 32],
}

impl PrecomputedHintHooks {
    fn new(sqrt_hint: [u8; 32], scalar_inv_hint: [u8; 32], fe_inv_hint: [u8; 32]) -> Self {
        Self {
            sqrt_hint,
            scalar_inv_hint,
            fe_inv_hint,
        }
    }
}

impl Secp256k1Hooks for PrecomputedHintHooks {
    fn fe_sqrt_and_assign(&mut self, fe: &mut FieldElement) -> bool {
        let candidate = FieldElement::from_bytes(&self.sqrt_hint).expect("invalid sqrt hint bytes");

        // Verify: candidate² == *fe.
        let mut check = candidate;
        check.square_in_place();
        check.sub_in_place(fe);
        let is_sqrt = check.normalizes_to_zero();

        *fe = candidate;
        is_sqrt
    }

    fn fe_invert_and_assign(&mut self, fe: &mut FieldElement) {
        let candidate =
            FieldElement::from_bytes(&self.fe_inv_hint).expect("invalid fe_inv hint bytes");

        // Verify: candidate * *fe == 1.
        let mut check = candidate;
        check.mul_in_place(fe);
        check.sub_in_place(&FieldElement::ONE);
        assert!(check.normalizes_to_zero(), "bad field inverse hint");

        *fe = candidate;
    }

    fn scalar_invert_and_assign(&mut self, scalar: &mut scalars::Scalar) {
        use airbender::crypto::k256::elliptic_curve::scalar::FromUintUnchecked;

        let val = U256::from_be_slice(&self.scalar_inv_hint);
        let candidate = scalars::Scalar::from_k256_scalar(Scalar::from_uint_unchecked(val));

        // Verify: candidate * *scalar == 1.
        let mut check = candidate;
        check *= *scalar;
        check = check - scalars::Scalar::ONE;
        assert!(check.is_zero(), "bad scalar inverse hint");

        *scalar = candidate;
    }
}
