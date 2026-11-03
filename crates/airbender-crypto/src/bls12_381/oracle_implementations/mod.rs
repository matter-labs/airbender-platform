//! The oracle side of the bls12-381 hints: the base field square root, and the residue witnesses
//! of the KZG proof check and of the pairing check, computed natively. The inverses and divisions
//! of the base field are generic over the field (`oracle_common`).

use super::curves::Bls12_381;
use super::{Fq, Fq12, G1Affine};
use crate::hints::guest_layout::{
    BLS12_381_G1_AFFINE, BLS12_381_G2_AFFINE, BLS12_381_PAIR_G1, BLS12_381_PAIR_G2,
    BLS12_381_PAIR_SIZE,
};
use crate::hints::{Bls12381Pair, HintTarget};
use crate::oracle_common::{Operand, Responses};
use alloc::vec::Vec;
use ark_ec::pairing::{MillerLoopOutput, Pairing};
use ark_ff::{Field, One, Zero};

/// `(candidate, is_quadratic_non_residue)`: `candidate² == input` for a square, `-input`
/// otherwise (`p = 3 mod 4`, so exactly one of `±input` is a square)
pub fn base_field_sqrt(operand: &Operand) -> (Fq, bool) {
    let el: Fq = operand.element();
    assert!(!el.is_zero());
    match el.sqrt() {
        Some(root) => (root, false),
        None => {
            let root = (-el).sqrt().expect("-1 is not a square, so -el is one");
            (root, true)
        }
    }
}

/// The claim about `e(p1, G2) e(p2, tau G2)` (see `hints::PairingClaim`). `claim_not_identity`
/// forces a non-identity claim (for tests of the exact path).
pub fn kzg_residue_witness(operand: &Operand, claim_not_identity: bool, responses: &mut Responses) {
    let [p1, p2]: [G1Affine; 2] = match operand.querier() {
        HintTarget::Native => operand.native_value(),
        HintTarget::Guest => {
            let size = BLS12_381_G1_AFFINE.size;
            assert_eq!(operand.size(), 2 * size, "the operand is two affine points");
            [
                operand.guest_affine(0, BLS12_381_G1_AFFINE),
                operand.guest_affine(size, BLS12_381_G1_AFFINE),
            ]
        }
    };
    let g2 = [
        &super::consts::PREPARED_G2_GENERATOR,
        &super::consts::PREPARED_G2_BY_TAU,
    ];
    let points = [p1, p2];
    // the witness equation holds for the verifier's Miller loop output only
    let pairs = points.iter().zip(g2);
    let f = Bls12_381::multi_miller_loop_shared(None, pairs.clone());
    let is_identity = !claim_not_identity
        && Bls12_381::final_exponentiation(MillerLoopOutput(f))
            .expect("non-zero")
            .0
            .is_one();
    if is_identity {
        let witness = crate::oracle_common::residue_witness::bls12_381::witness(&f)
            .expect("the final exponentiation found an identity, which has a witness");
        responses.write(&(true, witness));
    } else {
        // the inverse of the conjugated loop output, which the exact path computes
        let mut conjugated = f;
        conjugated.conjugate_in_place();
        responses.write(&(false, conjugated.inverse().expect("non-zero")));
    }
}

/// The claim about the bls12-381 pairing product, from the verifier's projective lines
pub fn pairing_residue_witness(
    operand: &Operand,
    claim_not_identity: bool,
    responses: &mut Responses,
) {
    let pairs: Vec<Bls12381Pair> = match operand.querier() {
        HintTarget::Native => operand.native_values(),
        HintTarget::Guest => {
            assert!(
                operand.size().is_multiple_of(BLS12_381_PAIR_SIZE),
                "the operand is affine pairs"
            );
            (0..operand.size() / BLS12_381_PAIR_SIZE)
                .map(|i| {
                    let offset = i * BLS12_381_PAIR_SIZE;
                    Bls12381Pair {
                        g1: operand.guest_affine(offset + BLS12_381_PAIR_G1, BLS12_381_G1_AFFINE),
                        g2: operand.guest_affine(offset + BLS12_381_PAIR_G2, BLS12_381_G2_AFFINE),
                    }
                })
                .collect()
        }
    };
    assert!(!pairs.is_empty(), "the operand has pairs");
    let f = Bls12_381::multi_miller_loop_with_initial_owned(
        &Fq12::one(),
        pairs.iter().map(|pair| pair.g1),
        pairs.iter().map(|pair| pair.g2),
    );
    let mut conjugated = f;
    conjugated.conjugate_in_place();
    let identity = !claim_not_identity
        && Bls12_381::final_exponentiation(MillerLoopOutput(conjugated))
            .expect("the Miller loop is non-zero")
            .0
            .is_one();
    if identity {
        let witness = crate::oracle_common::residue_witness::bls12_381::witness(&f)
            .expect("an identity has a residue witness");
        responses.write(&(true, witness));
    } else {
        responses.write(&(
            false,
            conjugated.inverse().expect("the Miller loop is non-zero"),
        ));
    }
}
