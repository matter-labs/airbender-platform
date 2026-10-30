//! The bn254 pairing check (EIP-197), on the EVM encoding of the pairs.

use super::bigint_from_be;
use crate::ark_ff::Zero;
use crate::ark_serialize::Valid;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints as curve_hints;
use airbender_oracle::MemoryOracle;
use alloc::vec::Vec;
use core::alloc::Allocator;

#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
use super::super::unavailable as curve_hints;

/// Whether the product of the pairings of the first `num_pairs` pairs of `src` is the identity.
/// A pair is 192 bytes, big-endian: G1 `x, y`, then G2 `x, y` with the imaginary part first; all
/// zeroes is the point at infinity, and `src` is read as if zero-padded. Fails unless every
/// coordinate is canonical, every point is on its curve and every G2 point is in the subgroup.
///
/// With an oracle, the G2 lines come from checked hinted slopes (asked pair by pair while
/// parsing), the G1 points are normalized with hinted divisions, and the final exponentiation is
/// replaced by a residue witness check or a checked hinted inverse. A wrong hint panics or is
/// ignored (unused slope words, or every slope of a chain the prover marks exceptional); it never
/// changes the result. The pairs, and with an oracle the 16 KB G2 lines and the G1 points, go
/// into vectors of `allocator`.
// The caller maps every failure to its invalid-point error
#[allow(clippy::result_unit_err)]
pub fn bn254_pairing_check<A: Allocator + Clone, O: MemoryOracle>(
    num_pairs: usize,
    src: &[u8],
    allocator: A,
    oracle: Option<&mut O>,
) -> Result<bool, ()> {
    use crate::ark_ec::pairing::Pairing;
    use crate::ark_ff::{One, PrimeField};
    use crate::bn254::curves::{
        g2_affine, Bn254, G1Affine, G1Evaluation, G2Affine, G2PreparedNoAlloc,
    };
    use crate::bn254::fields::{Fq, Fq2};

    if num_pairs == 0 {
        return Ok(true);
    }

    let mut oracle = oracle;
    let mut pairs = Vec::with_capacity_in(num_pairs, allocator.clone());
    // with an oracle: the affine G2 lines from hinted slopes, and the normalized G1 points
    let with_advice = if oracle.is_some() { num_pairs } else { 0 };
    let mut prepared = Vec::with_capacity_in(with_advice, allocator.clone());
    let mut evaluations = Vec::with_capacity_in(with_advice, allocator.clone());
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
        // the slopes of the point's two affine chains; unread slopes are skipped on drop
        let mut hints = oracle
            .as_deref_mut()
            .map(|oracle| curve_hints::bn254_g2_pairing_slopes(oracle, &g2_point));
        let is_in_subgroup = hints
            .as_mut()
            .and_then(|hints| hints.subgroup_chain())
            .and_then(|mut chain| g2_affine::membership(&g2_point, &mut chain))
            // no hints, or an exceptional chain: the projective test
            .unwrap_or_else(|| g2_point.is_in_correct_subgroup_assuming_on_curve());
        if !is_in_subgroup {
            return Err(());
        }
        if g1_is_zero {
            // e(O, Q) = 1
            continue;
        }
        if let Some(hints) = hints.as_mut() {
            // built in place in the next slot: 16 KB, not to be moved
            prepared.reserve(1);
            let slot = &mut prepared.spare_capacity_mut()[0];
            let affine = hints
                .line_chain()
                .and_then(|mut chain| g2_affine::prepare_into(&g2_point, &mut chain, slot));
            if affine.is_none() {
                // an exceptional chain: the projective lines
                slot.write(G2PreparedNoAlloc::from(g2_point));
            }
            // SAFETY: the slot was just initialized
            unsafe { prepared.set_len(prepared.len() + 1) };
        }
        drop(hints);
        if let Some(oracle) = oracle.as_deref_mut() {
            evaluations.push(G1Evaluation::new(
                &g1_point,
                &mut curve_hints::Bn254FqDivider::new(oracle),
            ));
        }

        pairs.push((g1_point, g2_point));
    }

    if pairs.is_empty() {
        return Ok(true);
    }

    let g1_iter = || pairs.iter().map(|(g1, _)| g1);
    let g2_iter = || pairs.iter().map(|(_, g2)| g2);
    let Some(oracle) = oracle else {
        let miller_loop = Bn254::multi_miller_loop(g1_iter(), g2_iter());
        let result =
            Bn254::final_exponentiation(miller_loop).expect("the Miller loop output is invertible");
        return Ok(result.0.is_one());
    };
    debug_assert!(prepared.len() == pairs.len() && evaluations.len() == pairs.len());
    let normalized = || evaluations.iter().zip(prepared.iter());
    // Residue witness check in place of the final exponentiation (eprint 2024/640, soundness in
    // `crate::residue_witness`). The prover cannot choose the result: a claimed identity must pass
    // the check or this panics, and a claimed non-identity takes the exact final exponentiation.
    match curve_hints::bn254_pairing_residue_witness(oracle, &pairs) {
        curve_hints::PairingClaim::Identity { c, d, s } => {
            let l = Bn254::multi_miller_loop_normalized(Some((&d, &c)), normalized());
            assert!(
                crate::residue_witness::bn254::check(&l, &d, &s),
                "the residue witness of the pairing claimed to be the identity is wrong"
            );
            Ok(true)
        }
        curve_hints::PairingClaim::NotIdentity { f_inverse } => {
            let miller_loop = Bn254::multi_miller_loop_normalized(None, normalized());
            let result = Bn254::final_exponentiation_with_inverse(&miller_loop, |f| {
                curve_hints::checked_inverse(f, f_inverse)
            })
            .expect("the Miller loop output is invertible");
            Ok(result.is_one())
        }
    }
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
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
        // and a point outside the subgroup, rejected while parsing
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

    /// The hints of each pair are asked while it is parsed, so a rejected pair comes after the
    /// hints of the pairs before it, and no residue witness is asked
    #[test]
    fn hints_are_asked_while_parsing() {
        let first = encode(&[(g1(6), g2(1))]);
        let rejected = encode(&[(g1(2), outside())]);
        let check = |src: &[u8]| {
            record_hints(|oracle| bn254_pairing_check(src.len() / 192, src, Global, Some(oracle)))
        };
        let (_, first_tape) = check(&first);
        let (_, rejected_tape) = check(&rejected);
        let (result, tape) = check(&[first, rejected].concat());
        assert_eq!(result, Err(()));
        assert!(!rejected_tape.is_empty());
        // the slopes and G1 divisions of the first pair, then the slopes of the second
        let (parsed, rest) = tape.split_at(tape.len() - rejected_tape.len());
        assert_eq!(rest, rejected_tape);
        assert!(!parsed.is_empty() && parsed.len() < first_tape.len());
        assert_eq!(parsed, &first_tape[..parsed.len()]);
    }
}
