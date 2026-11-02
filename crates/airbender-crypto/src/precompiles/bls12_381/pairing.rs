//! The BLS12-381 pairing check (EIP-2537).

use super::*;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints as curve_hints;
use crate::hints::{Bls12381Pair, PairingClaim};
use crate::{ark_ec::pairing::Pairing, ark_ff::One, bls12_381::curves::Bls12_381};
use alloc::vec::Vec;

#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
use super::super::unavailable as curve_hints;

/// The size of the EIP-2537 encoding of a pair of the pairing check: a G1 point and a G2 point
pub const BLS12_381_PAIR_LEN: usize = G1_SERIALIZATION_LEN + G2_SERIALIZATION_LEN;

/// Whether the product of the pairings of the pairs of `input` (a G1 and a G2 point each) is the
/// identity. Fails unless every point is canonical, on its curve and in the subgroup, checked
/// point by point while parsing.
///
/// With an oracle, the subgroup checks take checked hinted divisions, and the final
/// exponentiation is replaced by a residue witness check or a checked hinted inverse. A wrong
/// hint panics. The pairs go into a vector of `allocator`.
pub fn bls12_381_pairing_check<A: core::alloc::Allocator + Clone, O: MemoryOracle>(
    input: &[[u8; BLS12_381_PAIR_LEN]],
    allocator: A,
    mut oracle: Option<&mut O>,
) -> Result<bool, Bls12Error> {
    let num_pairs = input.len();
    let mut pairs = Vec::with_capacity_in(num_pairs, allocator.clone());

    for pair_encoding in input.iter() {
        let g1 = parse_g1_with_subgroup_check_with_oracle(
            pair_encoding[0..G1_SERIALIZATION_LEN].try_into().unwrap(),
            oracle.as_deref_mut(),
        )?;
        let g2 = parse_g2_with_subgroup_check_with_oracle(
            pair_encoding[G1_SERIALIZATION_LEN..(G1_SERIALIZATION_LEN + G2_SERIALIZATION_LEN)]
                .try_into()
                .unwrap(),
            oracle.as_deref_mut(),
        )?;
        pairs.push(Bls12381Pair { g1, g2 });
    }

    let is_identity = pairing_product_is_identity(&pairs, oracle);
    Ok(is_identity)
}

/// The hint can save the final exponentiation, but cannot choose the result.
fn pairing_product_is_identity<O: MemoryOracle>(
    pairs: &[Bls12381Pair],
    oracle: Option<&mut O>,
) -> bool {
    let Some(oracle) = oracle else {
        return Bls12_381::multi_pairing(
            pairs.iter().map(|pair| pair.g1),
            pairs.iter().map(|pair| pair.g2),
        )
        .0
        .is_one();
    };
    // the owned Miller loop prepares the G2 lines a chunk at a time
    let g1 = || pairs.iter().map(|pair| pair.g1);
    let g2 = || pairs.iter().map(|pair| pair.g2);
    match curve_hints::bls12_381_pairing_residue_witness(oracle, pairs) {
        PairingClaim::Identity { d, s, .. } => {
            let l = Bls12_381::multi_miller_loop_with_initial_owned(&d, g1(), g2());
            assert!(
                crate::residue_witness::bls12_381::check(&l, &d, &s),
                "the residue witness of the pairing claimed to be the identity is wrong"
            );
            true
        }
        PairingClaim::NotIdentity { f_inverse } => {
            let f = Bls12_381::multi_miller_loop(g1(), g2()).0;
            Bls12_381::final_exponentiation_with_inverse(&f, |f| {
                curve_hints::checked_inverse(f, f_inverse)
            })
            .expect("the Miller loop output is invertible")
            .is_one()
        }
    }
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::super::tests as bls_tests;
    use super::*;
    use crate::hints::test_support::{
        lying_oracle, non_identity_claiming_oracle, InProcessMemoryOracle,
    };
    use crate::hints::{
        answer_query, read_field_hint_request, record_hints, FieldHintOp, HintAnswer, HintTarget,
        NoHints, QuerierMemory,
    };
    use std::alloc::Global;
    use std::vec;
    use std::vec::Vec;

    /// The EIP-2537 encoding of `pairs`
    fn encode(pairs: &[(G1Affine, G2Affine)]) -> Vec<u8> {
        pairs
            .iter()
            .flat_map(|&(p, q)| {
                [bls_tests::write_g1(p).as_slice(), &bls_tests::write_g2(q)].concat()
            })
            .collect()
    }

    fn check<O: MemoryOracle>(pairs: &[(G1Affine, G2Affine)], oracle: Option<&mut O>) -> bool {
        let input = encode(pairs);
        bls12_381_pairing_check(input.as_chunks().0, Global, oracle).unwrap()
    }

    /// An oracle that is honest but for a false identity claim, with the witness `(1, 1)`
    fn false_identity_oracle(
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        InProcessMemoryOracle::new(|id: u32, word: usize, memory: &dyn QuerierMemory| {
            let (op, _, _) = read_field_hint_request(memory, word).unwrap();
            let mut answer = Vec::new();
            if op == FieldHintOp::Bls12381PairingResidueWitness as u32 {
                (true, (Fq12::one(), Fq6::one())).write(HintTarget::Native, &mut answer);
            } else {
                // SAFETY: the handler runs during the query, in this process, while the request
                // and its operand are live
                assert!(
                    unsafe { answer_query(id, word, memory, Some(&mut answer), None) }.unwrap()
                );
            }
            answer
        })
    }

    /// An oracle that claims every product not to be the identity cannot change the result
    #[test]
    fn non_identity_claims_take_the_exact_pairing() {
        let (p, q) = (G1Affine::generator(), G2Affine::generator());
        for (pairs, expected) in [(vec![(p, q)], false), (vec![(p, q), (-p, q)], true)] {
            assert_eq!(
                check(&pairs, Some(&mut non_identity_claiming_oracle())),
                expected
            );
        }
    }

    #[test]
    #[should_panic(expected = "residue witness")]
    fn corrupt_identity_witness_is_rejected() {
        let (p, q) = (G1Affine::generator(), G2Affine::generator());
        let mut lying = lying_oracle(&[FieldHintOp::Bls12381PairingResidueWitness]);
        check(&[(p, q), (-p, q)], Some(&mut lying));
    }

    #[test]
    #[should_panic(expected = "inverse hint is wrong")]
    fn corrupt_nonidentity_inverse_is_rejected() {
        let (p, q) = (G1Affine::generator(), G2Affine::generator());
        let mut lying = lying_oracle(&[FieldHintOp::Bls12381PairingResidueWitness]);
        check(&[(p, q)], Some(&mut lying));
    }

    #[test]
    #[should_panic(expected = "residue witness")]
    fn a_false_identity_claim_cannot_accept_a_nonidentity_pairing() {
        let (p, q) = (G1Affine::generator(), G2Affine::generator());
        check(&[(p, q)], Some(&mut false_identity_oracle()));
    }

    /// Products that are and are not the identity, points at infinity, points off the subgroups,
    /// and encodings that are mostly not of points of the curves
    #[test]
    fn advice_matches_no_advice() {
        use crate::precompiles::{HintedCases, HINT_HEAVY_CASES};
        use proptest::test_runner::Config as ProptestConfig;
        use proptest::{prop_assert_eq, proptest};

        let hinted_cases = HintedCases::default();
        proptest!(ProptestConfig::with_cases(HINT_HEAVY_CASES), |(ks: [(u64, u64); 2], len in 1usize..=2, mode in 0u8..8, at: usize, bytes: [u8; 96])| {
            let g1 = |k: u64| (G1Affine::generator() * Fr::from(k)).into_affine();
            let g2 = |k: u64| (G2Affine::generator() * Fr::from(k)).into_affine();
            let mut pairs: Vec<(G1Affine, G2Affine)> =
                ks[..len].iter().map(|&(a, b)| (g1(a), g2(b))).collect();
            let at = at % len;
            match mode {
                // the product is `e(G1, G2)^k`, which a last pair `(-k G1, G2)` cancels
                0 | 1 => {
                    let k = ks[..len]
                        .iter()
                        .fold(Fr::from(0u64), |k, &(a, b)| k + Fr::from(a) * Fr::from(b));
                    pairs.push(((G1Affine::generator() * -k).into_affine(), G2Affine::generator()));
                }
                2 => pairs[at].0 = G1Affine::identity(),
                3 => pairs[at].1 = G2Affine::identity(),
                4 => pairs[at].0 = bls_tests::g1_off_subgroup(&bytes[..48]),
                5 => pairs[at].1 = bls_tests::g2_off_subgroup(&bytes),
                _ => {}
            }
            let mut input = encode(&pairs);
            if mode == 6 {
                // mostly not a point of the curve
                input[at * BLS12_381_PAIR_LEN + usize::from(bytes[0]) % BLS12_381_PAIR_LEN] ^= 1;
            }
            let (input, []) = input.as_chunks::<BLS12_381_PAIR_LEN>() else {
                unreachable!("whole pairs")
            };
            let hinted = hinted_cases.record(|oracle| bls12_381_pairing_check(input, Global, Some(oracle)));
            prop_assert_eq!(hinted, bls12_381_pairing_check(input, Global, None::<&mut NoHints>));
        });
        hinted_cases.check();
    }

    #[test]
    fn recorded_tape_replays() {
        let (p, q) = (G1Affine::generator(), G2Affine::generator());
        for (pairs, expected) in [
            // the identity, which the residue witness settles
            (vec![(p, q), (-p, q)], Ok(true)),
            // not the identity, which the exact final exponentiation settles
            (vec![(p, q)], Ok(false)),
            // a point off the subgroup, rejected while the pairs are parsed
            (
                vec![(p, q), (bls_tests::g1_off_subgroup(&[7; 48]), q)],
                Err(Bls12Error::NotInSubgroup),
            ),
        ] {
            let input = encode(&pairs);
            let input = input.as_chunks::<BLS12_381_PAIR_LEN>().0;
            let result =
                assert_tape_replays!(|oracle| bls12_381_pairing_check(input, Global, Some(oracle)));
            assert_eq!(result, expected);
            assert_eq!(
                result,
                bls12_381_pairing_check(input, Global, None::<&mut NoHints>)
            );
        }
    }

    /// The subgroup checks run point by point while parsing, so a point off the subgroup is
    /// rejected after the hints of the points before it, and no residue witness is asked
    #[test]
    fn subgroup_hints_are_asked_while_parsing() {
        let (p, q) = (G1Affine::generator(), G2Affine::generator());
        let off_subgroup = bls_tests::g1_off_subgroup(&[7; 48]);
        let input = encode(&[(p, q), (off_subgroup, q)]);
        let (result, tape) = record_hints(|oracle| {
            bls12_381_pairing_check(input.as_chunks().0, Global, Some(oracle))
        });
        assert_eq!(result, Err(Bls12Error::NotInSubgroup));
        let g1_tape = |point: G1Affine| {
            record_hints(|oracle| {
                parse_g1_with_subgroup_check_with_oracle(&bls_tests::write_g1(point), Some(oracle))
            })
            .1
        };
        let (_, q_tape) = record_hints(|oracle| {
            parse_g2_with_subgroup_check_with_oracle(&bls_tests::write_g2(q), Some(oracle))
        });
        let parsed = [g1_tape(p), q_tape, g1_tape(off_subgroup)];
        assert!(parsed.iter().all(|tape| !tape.is_empty()));
        assert_eq!(tape, parsed.concat());
    }
}
