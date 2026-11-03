//! The oracle side of the bn254 hints: the residue witness of the pairing check and the slopes
//! of the affine `G2` chains, computed natively. The inverses and divisions of the base field are
//! generic over the field (`oracle_common`).

use super::curves::{g2_affine, Bn254, G1Evaluation, G2PreparedNoAlloc};
use super::{Fq2, G1Affine, G2Affine};
use crate::hints::guest_layout::{
    BN254_G1_AFFINE, BN254_G2_AFFINE, BN254_PAIR_G1, BN254_PAIR_G2, BN254_PAIR_SIZE,
};
use crate::hints::HintTarget;
use crate::oracle_common::{Operand, Responses};
use alloc::vec::Vec;
use ark_ec::pairing::{MillerLoopOutput, Pairing};
use ark_ff::{AdditiveGroup, Field, One};

/// The claim about the bn254 pairing product (see `hints::PairingClaim`). `claim_not_identity`
/// forces a non-identity claim (for tests of the exact path).
pub fn pairing_residue_witness(
    operand: &Operand,
    claim_not_identity: bool,
    responses: &mut Responses,
) {
    let pairs: Vec<(G1Affine, G2Affine)> = match operand.querier() {
        HintTarget::Native => operand.native_values(),
        HintTarget::Guest => {
            let size = BN254_PAIR_SIZE;
            assert!(
                operand.size().is_multiple_of(size),
                "the operand is affine pairs"
            );
            (0..operand.size() / size)
                .map(|k| {
                    let pair = k * size;
                    (
                        operand.guest_affine(pair + BN254_PAIR_G1, BN254_G1_AFFINE),
                        operand.guest_affine(pair + BN254_PAIR_G2, BN254_G2_AFFINE),
                    )
                })
                .collect()
        }
    };
    assert!(!pairs.is_empty(), "the operand has pairs");
    // the witness equation holds for the verifier's Miller loop output only
    let prepared: Vec<G2PreparedNoAlloc> = pairs
        .iter()
        .map(|(_, g2)| g2_affine::prepare_as_verifier(g2))
        .collect();
    let evaluations: Vec<G1Evaluation> = pairs
        .iter()
        .map(|(g1, _)| G1Evaluation::new(g1, &mut crate::affine_glv::InvertingDivider))
        .collect();
    let f = Bn254::multi_miller_loop_normalized(None, evaluations.iter().zip(prepared.iter()));
    let is_identity = !claim_not_identity
        && Bn254::final_exponentiation(MillerLoopOutput(f))
            .expect("non-zero")
            .0
            .is_one();
    if is_identity {
        let witness = crate::oracle_common::residue_witness::bn254::witness(&f)
            .expect("the final exponentiation found an identity, which has a witness");
        responses.write(&(true, witness));
    } else {
        responses.write(&(false, f.inverse().expect("non-zero")));
    }
}

/// The slopes of the two affine `G2` chains of the operand (see `hints::G2SlopeHints`); an
/// exceptional chain is a cleared flag and zeros
pub fn g2_pairing_slopes(operand: &Operand, responses: &mut Responses) {
    let q: G2Affine = match operand.querier() {
        HintTarget::Native => operand.native_value(),
        HintTarget::Guest => {
            assert_eq!(
                operand.size(),
                BN254_G2_AFFINE.size,
                "the operand is a point"
            );
            operand.guest_affine(0, BN254_G2_AFFINE)
        }
    };
    fn chain<const N: usize>(slopes: Option<[Fq2; N]>, responses: &mut Responses) {
        match slopes {
            Some(slopes) => responses.write(&(true, slopes)),
            None => responses.write(&(false, [Fq2::ZERO; N])),
        }
    }
    chain(g2_affine::subgroup_slopes(&q), responses);
    chain(g2_affine::line_slopes(&q), responses);
}
