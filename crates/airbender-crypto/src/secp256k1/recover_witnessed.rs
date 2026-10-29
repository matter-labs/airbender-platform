//! A key recovery checked against a witness instead of computed. The recovery of the key `Q`
//! of the signature `(r, s)` of `z` with the point `R` (`x = r`) is `Q = (s R - z G) / r`, a
//! double-scalar multiplication whose variable base `R` carries a scalar of 128 bits after the
//! endomorphism split: about 128 doublings. A prover that knows `Q` can instead witness it and
//! have the verifier check the relation `s R - z G - r Q = O` multiplied by a scalar of its
//! choice: with `λ` the eigenvalue of the endomorphism `φ` (`φ(P) = λ P`, `φ(x, y) = (β x, y)`)
//! and a vector `(a, b, c, d)` of the lattice `{(a + b λ) r ≡ (c + d λ) s (mod n)}`, which has
//! index `n` in `Z⁴` and so a non-zero vector of coordinates below `n^(1/4) ≈ 2^64`, the
//! relation times `v = (a + b λ) / s` reads
//!
//! `a R + b φ(R) - c Q - d φ(Q) - ((a + b λ) z / s) G = O`:
//!
//! four variable-base scalars of about 64 bits (about 64 doublings) and a full scalar on the
//! generator, which costs additions only over the four tables of `2^(64 i) G` (Antipa et al.
//! 2005 for the half-length rewriting, El Housni and Eagen for its composition with the
//! endomorphism). The verifier checks that `Q` is a point of the curve, that the coefficients
//! are within `WITNESS_COEFFICIENT_BITS` bits (the digits of the multiplication cover exactly
//! that many, so a larger coefficient would be computed with, but checked without, its high
//! bits), that `c + d λ` is not zero modulo `n` and that `(a + b λ) r ≡ (c + d λ) s`. Then,
//! for the true key `Q*`, the left side equals `(c + d λ)(Q* - Q)`, which is the point at
//! infinity only for `Q = Q*` (the group has prime order), so the check is sound; and
//! `a + b λ ≠ 0` follows from `c + d λ ≠ 0`, so an honest witness passes.
//!
//! Cost: about 64 doublings, 4 × 11 additions of the variable streams (wNAF-5), 2 × 8
//! operations for the tables and 4 × 6 additions of the generator streams (wNAF-10): about
//! 150 group operations instead of about 190, plus three scalar multiplications and the
//! on-curve check.

use super::context::ECMultContext;
use super::field::FieldElement;
use super::hooks::{RecoveryWitness, Secp256k1Hooks};
use super::points::Affine;
use super::recover_affine::ecmult_affine_4d;
use super::scalars::Scalar;
use super::Secp256k1Err;

/// The bits a witnessed coefficient may have in absolute value: a short vector has about 64,
/// the lattice reduction of the host guarantees fewer than this
pub const WITNESS_COEFFICIENT_BITS: usize = 80;

/// A signed integer as a scalar
fn signed_scalar(c: i128) -> Scalar {
    let scalar = Scalar::from_u128(c.unsigned_abs());
    if c < 0 {
        -scalar
    } else {
        scalar
    }
}

/// The key the witness claims for the signature `(r, s)` of `message` with the point `point`,
/// if the witness checks out (see the module documentation)
pub(super) fn verify_witnessed<H: Secp256k1Hooks>(
    point: &Affine,
    r: &Scalar,
    s: &Scalar,
    message: &Scalar,
    witness: &RecoveryWitness,
    context: &ECMultContext,
    hooks: &mut H,
) -> Result<Affine, Secp256k1Err> {
    let fail = || Secp256k1Err::InvalidWitness;

    // the key: canonical coordinates of a point of the curve, y² = x³ + 7
    let x = FieldElement::from_bytes(&witness.key_x).ok_or_else(fail)?;
    let y = FieldElement::from_bytes(&witness.key_y).ok_or_else(fail)?;
    let mut seven = [0u8; 32];
    seven[31] = 7;
    let seven = FieldElement::from_bytes(&seven).expect("below the modulus");
    let mut lhs = y;
    lhs.square_in_place();
    let mut rhs = x;
    rhs.square_in_place();
    rhs.mul_in_place(&x);
    rhs.add_in_place(&seven);
    lhs.sub_in_place(&rhs);
    if !lhs.is_zero() {
        return Err(fail());
    }
    let key = Affine {
        x,
        y,
        infinity: false,
    };

    // the coefficients, bounded
    let bound = 1i128 << WITNESS_COEFFICIENT_BITS;
    if witness
        .coefficients
        .iter()
        .any(|c| !(-bound < *c && *c < bound))
    {
        return Err(fail());
    }
    let [a, b, c, d] = witness.coefficients.map(signed_scalar);
    let [ca, cb, cc, cd] = witness.coefficients;
    let lambda = -Scalar::MINUS_LAMBDA;
    let mut big_a = b;
    big_a *= lambda;
    big_a += a;
    let mut big_c = d;
    big_c *= lambda;
    big_c += c;
    if big_c.is_zero() {
        return Err(fail());
    }

    // the congruence (a + b λ) r ≡ (c + d λ) s, as (a + b λ) (r / s) = c + d λ
    let mut s_inverse = *s;
    hooks.scalar_invert_and_assign(&mut s_inverse);
    let mut check = *r;
    check *= s_inverse;
    check *= big_a;
    check += -big_c;
    if !check.is_zero() {
        return Err(fail());
    }

    // a R + b φ(R) - c Q - d φ(Q) - ((a + b λ) z / s) G = O
    let mut g_scalar = big_a;
    g_scalar *= s_inverse;
    g_scalar *= *message;
    g_scalar.negate_in_place();
    let sum = ecmult_affine_4d(point, &key, &[ca, cb, -cc, -cd], &g_scalar, context, hooks);
    if !sum.is_infinity() {
        return Err(fail());
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::super::context::ECRECOVER_CONTEXT;
    use super::super::hooks::DefaultSecp256k1Hooks;
    use super::super::recover::{compute_from_point, recover_from_bytes_with_hooks};
    use super::super::witness_host::{decompose_4d, recovery_witness};
    use super::*;
    use core::mem::MaybeUninit;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    type SignatureInput = ([u8; 32], [u8; 32], [u8; 32], bool);

    /// Divisions by the host's arithmetic, counted, and a fixed witness
    struct WitnessHooks {
        witness: Option<RecoveryWitness>,
        divisions: usize,
    }

    impl Secp256k1Hooks for WitnessHooks {
        const FE_DIVIDE_IS_CHEAP: bool = true;

        fn recovery_witness(
            &mut self,
            _point: &Affine,
            _r: &Scalar,
            _s: &Scalar,
            _message: &Scalar,
        ) -> Option<RecoveryWitness> {
            self.witness
        }

        fn fe_sqrt_and_assign(&mut self, fe: &mut FieldElement) -> bool {
            DefaultSecp256k1Hooks.fe_sqrt_and_assign(fe)
        }

        fn fe_invert_and_assign(&mut self, fe: &mut FieldElement) {
            DefaultSecp256k1Hooks.fe_invert_and_assign(fe)
        }

        fn scalar_invert_and_assign(&mut self, scalar: &mut Scalar) {
            DefaultSecp256k1Hooks.scalar_invert_and_assign(scalar)
        }

        fn fe_divide<'a>(
            &mut self,
            fraction: &mut [FieldElement; 2],
            quotient: &'a mut MaybeUninit<FieldElement>,
        ) -> &'a mut FieldElement {
            assert!(!fraction[1].is_zero(), "a division by zero");
            self.divisions += 1;
            DefaultSecp256k1Hooks.fe_divide(fraction, quotient)
        }
    }

    fn recover(
        input: &([u8; 32], [u8; 32], [u8; 32], bool),
        hooks: &mut WitnessHooks,
    ) -> Result<Affine, Secp256k1Err> {
        let (digest, r, s, odd) = input;
        recover_from_bytes_with_hooks(digest, r, s, *odd, hooks)
    }

    /// Whether the recovery rejects the witness
    fn rejects(input: &([u8; 32], [u8; 32], [u8; 32], bool), witness: RecoveryWitness) -> bool {
        recover(
            input,
            &mut WitnessHooks {
                witness: Some(witness),
                divisions: 0,
            },
        )
        .is_err()
    }

    /// A signature of a random digest by a random key, as `(digest, r, s, y_is_odd)`
    fn sign(key: &[u8; 32], digest: &[u8; 32]) -> Option<SignatureInput> {
        let key = crate::k256::ecdsa::SigningKey::from_bytes(key.into()).ok()?;
        let (signature, recovery_id) = key.sign_prehash_recoverable(digest).ok()?;
        if recovery_id.is_x_reduced() {
            return None;
        }
        let bytes = signature.to_bytes();
        let (r, s) = bytes.split_at(32);
        Some((
            *digest,
            r.try_into().unwrap(),
            s.try_into().unwrap(),
            recovery_id.is_y_odd(),
        ))
    }

    #[test]
    fn witnessed_recovery_is_the_computed_one_and_cheaper() {
        proptest!(|(key: [u8; 32], digest: [u8; 32])| {
            if let Some(input) = sign(&key, &digest) {
                let mut computed = WitnessHooks { witness: None, divisions: 0 };
                let expected = recover(&input, &mut computed).unwrap();

                // the witness from the point and the scalars, as the hook receives them
                let (_, r, s, odd) = &input;
                let point = Affine::lift_x_with_hooks(
                    &FieldElement::from_bytes(r).unwrap(), *odd, &mut DefaultSecp256k1Hooks,
                ).unwrap();
                let r_scalar = Scalar::from_be_bytes_checked(r).unwrap();
                let s_scalar = Scalar::from_be_bytes_checked(s).unwrap();
                let message = Scalar::from_be_bytes_reduced(&input.0);
                let witness = recovery_witness(&point, &r_scalar, &s_scalar, &message).unwrap();
                prop_assert!(witness.coefficients.iter().all(|c| c.unsigned_abs() < 1 << 70));

                let mut witnessed = WitnessHooks { witness: Some(witness), divisions: 0 };
                let recovered = recover(&input, &mut witnessed).unwrap();
                prop_assert_eq!(recovered, expected);
                prop_assert!(
                    witnessed.divisions < computed.divisions,
                    "{} divisions witnessed, {} computed", witnessed.divisions, computed.divisions
                );

                // a wrong key (the generator, which is on the curve), a key off the curve, zero
                // coefficients, a coefficient beyond the bound, a coefficient off by one
                let mut wrong = witness;
                let mut x = [0u8; 32];
                let mut y = [0u8; 32];
                ECRECOVER_CONTEXT.pre_g[0].to_affine().write_coordinates(&mut x, &mut y);
                wrong.key_x = x;
                wrong.key_y = y;
                prop_assert!(rejects(&input, wrong));
                let mut wrong = witness;
                wrong.key_y[31] ^= 1;
                prop_assert!(rejects(&input, wrong));
                let mut wrong = witness;
                wrong.coefficients = [0; 4];
                prop_assert!(rejects(&input, wrong));
                let mut wrong = witness;
                wrong.coefficients[0] = 1 << WITNESS_COEFFICIENT_BITS;
                prop_assert!(rejects(&input, wrong));
                let mut wrong = witness;
                wrong.coefficients[2] += 1;
                prop_assert!(rejects(&input, wrong));

                // the computed recovery agrees with the reference entry point
                let reference = compute_from_point(
                    &point, r_scalar, s_scalar, message, &ECRECOVER_CONTEXT, &mut DefaultSecp256k1Hooks,
                ).unwrap();
                prop_assert_eq!(expected, reference);
            }
        })
    }

    #[test]
    fn decomposition_vectors_are_short_and_in_the_lattice() {
        proptest!(|(r: Scalar, s: Scalar)| {
            if !r.is_zero() && !s.is_zero() {
                let [a, b, c, d] = decompose_4d(&r, &s).unwrap().map(signed_scalar);
                let lambda = -Scalar::MINUS_LAMBDA;
                let mut lhs = b;
                lhs *= lambda;
                lhs += a;
                lhs *= r;
                let mut rhs = d;
                rhs *= lambda;
                rhs += c;
                rhs *= s;
                prop_assert_eq!(lhs, rhs);
                prop_assert!(!rhs.is_zero());
            }
        })
    }

    /// The canonical bytes of a field element
    fn bytes_of(fe: &FieldElement) -> [u8; 32] {
        let mut fe = *fe;
        fe.normalize_in_place();
        let mut bytes = [0u8; 32];
        fe.write_bytes(&mut bytes);
        bytes
    }

    /// The negation of a field element
    fn negated(fe: &FieldElement) -> FieldElement {
        let mut fe = *fe;
        fe.negate_in_place(1);
        fe
    }

    /// The witness with the key replaced by `(x, y)`
    fn with_key(witness: RecoveryWitness, key_x: [u8; 32], key_y: [u8; 32]) -> RecoveryWitness {
        RecoveryWitness {
            key_x,
            key_y,
            ..witness
        }
    }

    /// The witness with the coefficients replaced
    fn with_coefficients(witness: RecoveryWitness, coefficients: [i128; 4]) -> RecoveryWitness {
        RecoveryWitness {
            coefficients,
            ..witness
        }
    }

    /// The key the recovery accepts with the witness
    fn accepted(
        input: &([u8; 32], [u8; 32], [u8; 32], bool),
        witness: RecoveryWitness,
    ) -> Result<Affine, Secp256k1Err> {
        recover(
            input,
            &mut WitnessHooks {
                witness: Some(witness),
                divisions: 0,
            },
        )
    }

    /// The key is checked as a whole: another point of the curve, in particular the key of the
    /// other parity (`-Q`), the image under the endomorphism, the point of the signature and its
    /// negation (which collide with the first table), the key of the other recovery id and a
    /// correct key against the other recovery id are rejected, as are coordinates that are not
    /// canonical. Sign changes of the whole coefficient vector stay in the lattice and are
    /// accepted.
    #[test]
    fn keys_of_other_points_and_parities_are_rejected() {
        let mut modulus = [0xffu8; 32];
        modulus[27] = 0xfe;
        modulus[30] = 0xfc;
        modulus[31] = 0x2f;

        proptest!(|(key: [u8; 32], digest: [u8; 32])| {
            if let Some(input) = sign(&key, &digest) {
                let (_, r, s, odd) = &input;
                let x_of_r = FieldElement::from_bytes(r).unwrap();
                let point = Affine::lift_x_with_hooks(&x_of_r, *odd, &mut DefaultSecp256k1Hooks).unwrap();
                let r_scalar = Scalar::from_be_bytes_checked(r).unwrap();
                let s_scalar = Scalar::from_be_bytes_checked(s).unwrap();
                let message = Scalar::from_be_bytes_reduced(&input.0);
                let witness = recovery_witness(&point, &r_scalar, &s_scalar, &message).unwrap();
                let expected = accepted(&input, witness).unwrap();
                let (x, y) = (bytes_of(&expected.x), bytes_of(&expected.y));
                prop_assert_eq!((witness.key_x, witness.key_y), (x, y));

                // the other y of the same x: the key of the other parity
                prop_assert!(rejects(&input, with_key(witness, x, bytes_of(&negated(&expected.y)))));
                // the image under the endomorphism, (β x, y)
                let mut image = expected.x;
                image.mul_in_place(&FieldElement::BETA);
                prop_assert!(rejects(&input, with_key(witness, bytes_of(&image), y)));
                // the point of the signature and its negation
                let (point_x, point_y) = (bytes_of(&point.x), bytes_of(&point.y));
                prop_assert!(rejects(&input, with_key(witness, point_x, point_y)));
                prop_assert!(rejects(&input, with_key(witness, point_x, bytes_of(&negated(&point.y)))));
                // coordinates that are not canonical: the modulus, and all ones
                prop_assert!(rejects(&input, with_key(witness, modulus, y)));
                prop_assert!(rejects(&input, with_key(witness, x, modulus)));
                prop_assert!(rejects(&input, with_key(witness, x, [0xff; 32])));

                // the witness of the other recovery id, against this one
                let other_point = Affine::lift_x_with_hooks(&x_of_r, !*odd, &mut DefaultSecp256k1Hooks).unwrap();
                if let Some(other) = recovery_witness(&other_point, &r_scalar, &s_scalar, &message) {
                    prop_assert!(rejects(&input, other));
                    // which is the correct one for the other recovery id
                    let flipped = (input.0, input.1, input.2, !input.3);
                    prop_assert!(accepted(&flipped, other).is_ok());
                    prop_assert!(rejects(&flipped, witness));
                }

                // the whole vector negated, or doubled while within the bound: the same lattice
                let negated_vector = witness.coefficients.map(|c| -c);
                prop_assert_eq!(accepted(&input, with_coefficients(witness, negated_vector)).unwrap(), expected);
                if witness.coefficients.iter().all(|c| c.unsigned_abs() < 1 << (WITNESS_COEFFICIENT_BITS - 1)) {
                    let doubled = witness.coefficients.map(|c| 2 * c);
                    prop_assert_eq!(accepted(&input, with_coefficients(witness, doubled)).unwrap(), expected);
                }
                // one coefficient negated leaves the lattice
                let mut one_negated = witness.coefficients;
                one_negated[1] = -one_negated[1];
                if one_negated[1] != 0 {
                    prop_assert!(rejects(&input, with_coefficients(witness, one_negated)));
                }
            }
        })
    }
}
