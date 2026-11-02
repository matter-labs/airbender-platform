//! The bn254 pairing check (EIP-197), on the EVM encoding of the pairs.

use super::bigint_from_be;
use crate::ark_ec::pairing::Pairing;
use crate::ark_ff::{One, PrimeField, Zero};
use crate::ark_serialize::Valid;
use crate::bn254::curves::{g2_affine, Bn254, G1Affine, G1Evaluation, G2Affine, G2PreparedNoAlloc};
use crate::bn254::fields::{Fq, Fq12, Fq2};
#[cfg(any(feature = "hinted_operations", test))]
use crate::hints as curve_hints;
use airbender_oracle::MemoryOracle;
use alloc::vec::Vec;
use core::alloc::Allocator;
use core::mem::MaybeUninit;

#[cfg(not(any(feature = "hinted_operations", test)))]
use super::super::unavailable as curve_hints;

/// The pairs of the hinted path whose Miller loops run with one shared accumulator: up to this
/// many prepared `G2` points (16 KB each, the affine lines of hinted slopes) and normalized `G1`
/// points are held on the stack at a time; every further chunk costs one more loop's squarings
/// and one multiplication. Guests have 16-32 MB stacks, the host the thread stacks of its
/// embedding.
#[cfg(target_arch = "riscv32")]
const MILLER_LOOP_CHUNK: usize = 4;
#[cfg(not(target_arch = "riscv32"))]
const MILLER_LOOP_CHUNK: usize = 2;

/// Whether the product of the pairings of the first `num_pairs` pairs of `src` is the identity.
/// A pair is 192 bytes, big-endian: G1 `x, y`, then G2 `x, y` with the imaginary part first; all
/// zeroes is the point at infinity, and `src` is read as if zero-padded. Fails unless every
/// coordinate is canonical, every point is on its curve and every G2 point is in the subgroup.
///
/// With an oracle, the residue witness of the product is asked once the pairs are parsed, and
/// then, a chunk of pairs at a time, the G2 lines come from checked hinted slopes (which also
/// check the subgroup) and the G1 points are normalized with hinted divisions; the final
/// exponentiation is replaced by the residue witness check or a checked hinted inverse. A wrong
/// hint panics or is ignored (unused slope words, or every slope of a chain the prover marks
/// exceptional); it never changes the result. Only the pairs go into a vector of `allocator`:
/// the 16 KB G2 lines and the G1 points of a chunk live on the stack.
// The caller maps every failure to its invalid-point error
#[allow(clippy::result_unit_err)]
pub fn bn254_pairing_check<A: Allocator + Clone, O: MemoryOracle>(
    num_pairs: usize,
    src: &[u8],
    allocator: A,
    oracle: Option<&mut O>,
) -> Result<bool, ()> {
    if num_pairs == 0 {
        return Ok(true);
    }

    let mut oracle = oracle;
    let mut pairs = Vec::with_capacity_in(num_pairs, allocator);
    let mut src_iter = src.iter();

    for _ in 0..num_pairs {
        let mut buffer = [0u8; 192];
        for (dst, src) in buffer.iter_mut().zip(&mut src_iter) {
            *dst = *src;
        }
        // EIP-197 puts the imaginary part of each G2 coordinate first
        let ([g1_x, g1_y, g2_x_c1, g2_x_c0, g2_y_c1, g2_y_c0], []) = buffer.as_chunks::<32>()
        else {
            unreachable!("192 bytes are 6 chunks of 32")
        };
        let parse = |bytes: &[u8; 32]| Fq::from_bigint(bigint_from_be(bytes)).ok_or(());
        let g1_x = parse(g1_x)?;
        let g1_y = parse(g1_y)?;

        let g1_is_zero = g1_x.is_zero() && g1_y.is_zero();
        let g1_point = G1Affine::new_unchecked(g1_x, g1_y);
        if !g1_is_zero {
            g1_point.check().map_err(|_| ())?;
        }

        let g2_x = Fq2::new(parse(g2_x_c0)?, parse(g2_x_c1)?);
        let g2_y = Fq2::new(parse(g2_y_c0)?, parse(g2_y_c1)?);

        let g2_is_zero = g2_x.is_zero() && g2_y.is_zero();
        let g2_point = G2Affine::new_unchecked(g2_x, g2_y);
        if g2_is_zero {
            // e(P, O) = 1; the G1 point was validated above
            continue;
        }
        if !g2_point.is_on_curve() {
            return Err(());
        }
        if g1_is_zero {
            // e(O, Q) = 1, but Q must be in the subgroup all the same: with an oracle, the
            // membership test runs on hinted slopes (the rest of the answer is skipped)
            let is_in_subgroup = match oracle.as_deref_mut() {
                Some(oracle) => {
                    let mut hints = curve_hints::bn254_g2_pairing_slopes(oracle, &g2_point);
                    membership_with_hints(&g2_point, hints.subgroup_chain())
                }
                None => g2_point.is_in_correct_subgroup_assuming_on_curve(),
            };
            if !is_in_subgroup {
                return Err(());
            }
            continue;
        }

        pairs.push((g1_point, g2_point));
    }

    if pairs.is_empty() {
        return Ok(true);
    }

    let Some(oracle) = oracle else {
        if pairs
            .iter()
            .any(|(_, g2)| !g2.is_in_correct_subgroup_assuming_on_curve())
        {
            return Err(());
        }
        // each G2 point is prepared projectively, one at a time
        let miller_loop = Bn254::multi_miller_loop(
            pairs.iter().map(|(g1, _)| g1),
            pairs.iter().map(|(_, g2)| g2),
        );
        let result =
            Bn254::final_exponentiation(miller_loop).expect("the Miller loop output is invertible");
        return Ok(result.0.is_one());
    };
    hinted_check(&pairs, oracle)
}

/// The membership test of `q` on the hinted chain, or projectively when the prover marks the
/// chain exceptional
fn membership_with_hints<C: crate::affine_glv::Divider<Fq2>>(
    q: &G2Affine,
    chain: Option<C>,
) -> bool {
    chain
        .and_then(|mut chain| g2_affine::membership(q, &mut chain))
        .unwrap_or_else(|| q.is_in_correct_subgroup_assuming_on_curve())
}

/// The hinted check of parsed pairs (on the curves, no point at infinity): the residue witness
/// claim, then the Miller loop a chunk of pairs at a time (the subgroup checks on the way), with
/// the first chunk's accumulator started at the witness. Not inlined: the chunk buffers live in
/// this frame only while the loop runs.
#[inline(never)]
fn hinted_check<O: MemoryOracle>(
    pairs: &[(G1Affine, G2Affine)],
    oracle: &mut O,
) -> Result<bool, ()> {
    // Residue witness check in place of the final exponentiation (eprint 2024/640, soundness in
    // `crate::oracle_common::residue_witness`). The prover cannot choose the result: a claimed
    // identity must pass the check or this panics, and a claimed non-identity takes the exact
    // final exponentiation. The claim is asked before the subgroup checks of the pairs: an
    // invalid pair fails the check whatever the claim, which is then dropped.
    let claim = curve_hints::bn254_pairing_residue_witness(oracle, pairs);
    let initial = match &claim {
        curve_hints::PairingClaim::Identity { c, d, .. } => Some((d, c)),
        curve_hints::PairingClaim::NotIdentity { .. } => None,
    };

    let mut prepared: [MaybeUninit<G2PreparedNoAlloc>; MILLER_LOOP_CHUNK] =
        [const { MaybeUninit::uninit() }; MILLER_LOOP_CHUNK];
    let mut evaluations: [MaybeUninit<G1Evaluation>; MILLER_LOOP_CHUNK] =
        [const { MaybeUninit::uninit() }; MILLER_LOOP_CHUNK];
    let mut f = Fq12::one();
    let mut started = false;
    for chunk in pairs.chunks(MILLER_LOOP_CHUNK) {
        for (i, (g1, g2)) in chunk.iter().enumerate() {
            // the slopes of the point's two affine chains; unread slopes are skipped on drop
            let mut hints = curve_hints::bn254_g2_pairing_slopes(oracle, g2);
            if !membership_with_hints(g2, hints.subgroup_chain()) {
                return Err(());
            }
            // built in place in its slot: 16 KB, not to be moved
            let slot = &mut prepared[i];
            let affine = hints
                .line_chain()
                .and_then(|mut chain| g2_affine::prepare_into(g2, &mut chain, slot));
            if affine.is_none() {
                // an exceptional chain: the projective lines
                slot.write(G2PreparedNoAlloc::from(*g2));
            }
            drop(hints);
            evaluations[i].write(G1Evaluation::new(
                g1,
                &mut curve_hints::Bn254FqDivider::new(oracle),
            ));
        }
        // SAFETY: the first `chunk.len()` slots of both arrays were initialized above
        let (prepared, evaluations) = unsafe {
            (
                core::slice::from_raw_parts(
                    prepared.as_ptr().cast::<G2PreparedNoAlloc>(),
                    chunk.len(),
                ),
                core::slice::from_raw_parts(
                    evaluations.as_ptr().cast::<G1Evaluation>(),
                    chunk.len(),
                ),
            )
        };
        let normalized = || evaluations.iter().zip(prepared.iter());
        // the Miller loop is multiplicative in the pairs: the product of the loops of the chunks,
        // the first one started at the witness, is the loop of all the pairs started there
        if started {
            f *= Bn254::multi_miller_loop_normalized(None, normalized());
        } else {
            f = Bn254::multi_miller_loop_normalized(initial, normalized());
            started = true;
        }
    }

    match claim {
        curve_hints::PairingClaim::Identity { d, s, .. } => {
            assert!(
                crate::oracle_common::residue_witness::bn254::check(&f, &d, &s),
                "the residue witness of the pairing claimed to be the identity is wrong"
            );
            Ok(true)
        }
        curve_hints::PairingClaim::NotIdentity { f_inverse } => {
            let result = Bn254::final_exponentiation_with_inverse(&f, |f| {
                Some(curve_hints::checked_inverse(f, f_inverse))
            })
            .expect("the Miller loop output is invertible");
            Ok(result.is_one())
        }
    }
}

// The tests use the host side (`oracle_common`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::ark_ec::{AffineRepr, CurveGroup};
    use crate::bn254::{Fq, Fq2, Fr, G1Affine, G2Affine};
    use crate::hints::test_support::{lying_oracle, lying_oracle_last_word};
    use crate::hints::{record_hints, FieldHintOp, NoHints};
    use crate::precompiles::HintedCases;
    use hex_literal::hex;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};
    use std::alloc::Global;
    use std::vec::Vec;

    /// Two pairs whose product is the identity
    const SRC: [u8; 384] = hex!(
        "1c76476f4def4bb94541d57ebba1193381ffa7aa76ada664dd31c16024c43f59"
        "3034dd2920f673e204fee2811c678745fc819b55d3e9d294e45c9b03a76aef41"
        "209dd15ebff5d46c4bd888e51a93cf99a7329636c63514396b4a452003a35bf7"
        "04bf11ca01483bfa8b34b43561848d28905960114c8ac04049af4b6315a41678"
        "2bb8324af6cfc93537a2ad1a445cfd0ca2a71acd7ac41fadbf933c2a51be344d"
        "120a2a4cf30c1bf9845f20c6fe39e07ea2cce61f0c9bb048165fe5e4de877550"
        "111e129f1cf1097710d41c4ac70fcdfa5ba2023c6ff1cbeac322de49d1b6df7c"
        "2032c61a830e3c17286de9462bf242fca2883585b93870a73853face6a6bf411"
        "198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2"
        "1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed"
        "090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b"
        "12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa"
    );

    /// The EIP-197 encoding of `pairs` (the point at infinity has zero coordinates)
    fn encode(pairs: &[(G1Affine, G2Affine)]) -> Vec<u8> {
        use crate::ark_ff::{BigInteger, PrimeField};
        let elements = |(p, q): &(G1Affine, G2Affine)| [p.x, p.y, q.x.c1, q.x.c0, q.y.c1, q.y.c0];
        let bytes = |element: Fq| element.into_bigint().to_bytes_be();
        pairs.iter().flat_map(elements).flat_map(bytes).collect()
    }

    fn g1(k: u64) -> G1Affine {
        (G1Affine::generator() * Fr::from(k)).into_affine()
    }

    fn g2(k: u64) -> G2Affine {
        (G2Affine::generator() * Fr::from(k)).into_affine()
    }

    /// The first point of the twist outside the subgroup from `x = 1` up
    fn outside() -> G2Affine {
        use crate::ark_ec::short_weierstrass::SWCurveConfig;
        use crate::ark_ff::Field;
        let mut x = Fq2::new(Fq::from(1u64), Fq::zero());
        loop {
            let y2 = x * x * x + crate::bn254::g2::Config::COEFF_B;
            if let Some(y) = y2.sqrt() {
                let q = G2Affine::new_unchecked(x, y);
                if !q.is_in_correct_subgroup_assuming_on_curve() {
                    return q;
                }
            }
            x.c0 += Fq::from(1u64);
        }
    }

    /// The identity, not the identity, only points at infinity (an empty product), and a G2 point
    /// outside the subgroup paired with the point at infinity (still rejected)
    #[test]
    fn known_results() {
        let not_identity = encode(&[(g1(6), g2(1)), (g1(2), g2(3))]);
        let infinities = encode(&[(G1Affine::identity(), g2(1)), (g1(3), G2Affine::identity())]);
        let outside = encode(&[(G1Affine::identity(), outside())]);
        for (src, expected) in [
            (&SRC[..], Ok(true)),
            (&not_identity, Ok(false)),
            (&infinities, Ok(true)),
            (&outside, Err(())),
        ] {
            let n = src.len() / 192;
            let (hinted, _) =
                record_hints(|oracle| bn254_pairing_check(n, src, Global, Some(oracle)));
            assert_eq!(hinted, expected);
            assert_eq!(
                bn254_pairing_check(n, src, Global, None::<&mut NoHints>),
                expected
            );
        }
    }

    /// More pairs than a chunk holds, identities and not, with and without hints
    #[test]
    fn products_over_several_chunks() {
        for n in [
            MILLER_LOOP_CHUNK,
            MILLER_LOOP_CHUNK + 1,
            2 * MILLER_LOOP_CHUNK + 1,
        ] {
            let mut pairs: Vec<(G1Affine, G2Affine)> =
                (1..=n as u64).map(|k| (g1(k), g2(k + 1))).collect();
            let not_identity = encode(&pairs);
            // e(G1, G2)^k, cancelled by a last pair
            let k = (1..=n as u64).fold(Fr::from(0u64), |k, i| k + Fr::from(i) * Fr::from(i + 1));
            pairs.push((
                (G1Affine::generator() * -k).into_affine(),
                G2Affine::generator(),
            ));
            let identity = encode(&pairs);
            for (src, expected) in [(&identity, Ok(true)), (&not_identity, Ok(false))] {
                let n = src.len() / 192;
                let result = assert_tape_replays!(|oracle| bn254_pairing_check(
                    n,
                    src,
                    Global,
                    Some(oracle)
                ));
                assert_eq!(result, expected);
                assert_eq!(
                    bn254_pairing_check(n, src, Global, None::<&mut NoHints>),
                    expected
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "not an inverse pair")]
    fn wrong_identity_witness_inverse_panics() {
        let mut lying = lying_oracle(&[FieldHintOp::Bn254PairingResidueWitness]);
        let _ = bn254_pairing_check(2, &SRC, Global, Some(&mut lying));
    }

    #[test]
    #[should_panic(expected = "claimed to be the identity is wrong")]
    fn wrong_identity_witness_scaling_factor_panics() {
        let mut lying = lying_oracle_last_word(&[FieldHintOp::Bn254PairingResidueWitness]);
        let _ = bn254_pairing_check(2, &SRC, Global, Some(&mut lying));
    }

    #[test]
    #[should_panic(expected = "division hint is wrong")]
    fn wrong_slope_panics() {
        let src = encode(&[(G1Affine::generator(), G2Affine::generator())]);
        let mut lying = lying_oracle_last_word(&[FieldHintOp::Bn254G2PairingSlopes]);
        let _ = bn254_pairing_check(1, &src, Global, Some(&mut lying));
    }

    /// Products that are and are not the identity, points at infinity, G2 points outside the
    /// subgroup, and encodings that are mostly not of points of the curves
    #[test]
    fn advice_matches_no_advice() {
        let outside = outside();
        let hinted_cases = HintedCases::default();
        proptest!(ProptestConfig::with_cases(64), |(ks: [(u64, u64); 3], len in 1usize..=3, mode in 0u8..6, at: usize, flip: [u8; 3])| {
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
                4 => pairs[at].1 = outside,
                _ => {}
            }
            let mut src = encode(&pairs);
            if flip[0] < 32 {
                // mostly not a point of the curve, or a coordinate above the modulus
                src[at * 192 + usize::from(flip[1]) % 192] ^= flip[2] | 1;
            }
            let n = pairs.len();
            let hinted = hinted_cases.record(|oracle| bn254_pairing_check(n, &src, Global, Some(oracle)));
            prop_assert_eq!(hinted, bn254_pairing_check(n, &src, Global, None::<&mut NoHints>));
        });
        hinted_cases.check();
    }

    #[test]
    fn recorded_tape_replays() {
        // the identity (the residue witness), not the identity (the exact final exponentiation),
        // and a point outside the subgroup, rejected at its chunk
        let not_identity = encode(&[(g1(6), g2(1)), (g1(2), g2(3))]);
        let rejected = encode(&[(g1(6), g2(1)), (g1(2), outside())]);
        for (src, expected) in [
            (&SRC[..], Ok(true)),
            (&not_identity, Ok(false)),
            (&rejected, Err(())),
        ] {
            let result =
                assert_tape_replays!(|oracle| bn254_pairing_check(2, src, Global, Some(oracle)));
            assert_eq!(result, expected);
        }
    }

    /// The residue witness claim comes first, then the hints of each pair in order: a rejected
    /// pair comes after the hints of the pairs before it, and stops the loop
    #[test]
    fn hints_are_asked_after_the_claim_pair_by_pair() {
        // the words of a non-identity claim: a flag and an `Fq12` inverse
        const CLAIM_WORDS: usize = 1 + 12 * 8;
        let first = encode(&[(g1(6), g2(1))]);
        let rejected = encode(&[(g1(2), outside())]);
        let check = |src: &[u8]| {
            record_hints(|oracle| bn254_pairing_check(src.len() / 192, src, Global, Some(oracle)))
        };
        let (_, first_tape) = check(&first);
        let (rejected_result, rejected_tape) = check(&rejected);
        assert_eq!(rejected_result, Err(()));
        let (result, tape) = check(&[first, rejected].concat());
        assert_eq!(result, Err(()));
        // the claim of the pairs together, the slopes and G1 divisions of the first pair, then
        // the slopes of the second, whose subgroup check fails
        assert_eq!(
            tape.len(),
            first_tape.len() + rejected_tape.len() - CLAIM_WORDS
        );
        assert_eq!(
            &tape[CLAIM_WORDS..first_tape.len()],
            &first_tape[CLAIM_WORDS..]
        );
        assert_eq!(&tape[first_tape.len()..], &rejected_tape[CLAIM_WORDS..]);
    }
}
