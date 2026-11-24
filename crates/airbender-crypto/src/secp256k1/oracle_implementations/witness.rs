//! The prover's side of the witnessed recovery (`recover_witnessed`): the key by the plain
//! recovery, and the coefficients by lattice reduction (`oracle_common::short_vector_4d`). Host
//! only.

use super::super::context::ECRECOVER_CONTEXT;
use super::super::hooks::{DefaultSecp256k1Hooks, RecoveryWitness};
use super::super::points::Affine;
use super::super::recover::compute_from_point;
use super::super::recover_witnessed::WITNESS_COEFFICIENT_BITS;
use super::super::scalars::Scalar;
use crate::oracle_common::short_vector_4d;
use num_bigint::{BigInt, Sign};
use num_traits::One;

/// The witness for the signature `(r, s)` of `message` (reduced) with the point `point`, as the
/// `Secp256k1Hooks::recovery_witness` hook of a prover answers: `None` when the recovered key is
/// the point at infinity (the recovery fails then, computed), or when no short enough vector
/// was found (it always is: the bound is well above the Minkowski bound)
pub fn recovery_witness(
    point: &Affine,
    r: &Scalar,
    s: &Scalar,
    message: &Scalar,
) -> Option<RecoveryWitness> {
    let key = compute_from_point(
        point,
        *r,
        *s,
        *message,
        &ECRECOVER_CONTEXT,
        &mut DefaultSecp256k1Hooks,
    )
    .ok()?;
    let mut key_x = [0u8; 32];
    let mut key_y = [0u8; 32];
    key.write_coordinates(&mut key_x, &mut key_y);
    Some(RecoveryWitness {
        key_x,
        key_y,
        coefficients: decompose_4d(r, s)?,
    })
}

fn big(scalar: &Scalar) -> BigInt {
    let bytes: [u8; 32] = scalar.to_repr().into();
    BigInt::from_bytes_be(Sign::Plus, &bytes)
}

/// A short non-zero vector `(a, b, c, d)` with `(a + b λ) r ≡ (c + d λ) s (mod n)`, every
/// coordinate within `WITNESS_COEFFICIENT_BITS` bits
pub fn decompose_4d(r: &Scalar, s: &Scalar) -> Option<[i128; 4]> {
    // the order: minus one, plus one
    let n = big(&-Scalar::ONE) + BigInt::one();
    let lambda = big(&-Scalar::MINUS_LAMBDA);
    // t = r / s
    let mut t = *r;
    t *= s.invert_vartime();
    short_vector_4d(&big(&t), &lambda, &n, WITNESS_COEFFICIENT_BITS)
}
