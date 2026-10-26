// Ported from zksync-os e855bca2:basic_system/src/system_functions/bn254_pairing_check.rs
//! The bn254 pairing check (EIP-197), with the EVM encoding of the pairs.

use super::bigint_from_be;
use crate::ark_ff::Zero;
use crate::ark_serialize::Valid;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints as curve_hints;
use crate::hints::HintOracle;
use alloc::vec::Vec;
use core::alloc::Allocator;

#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
use super::super::unavailable as curve_hints;

/// The bn254 pairing check (EIP-197) of the first `num_pairs` pairs of `src`, 192 bytes
/// each: the coordinates of a G1 point, then those of a G2 point with the imaginary parts
/// first, big-endian, and all zeroes for a point at infinity (`src` is read as if padded with
/// zeroes). Whether the product of the pairings is the identity; fails unless every coordinate
/// is canonical, every point is on its curve, and every G2 point is in the subgroup. The
/// wrapper keeps the gas, the size of the input (a multiple of 192), the empty input (which is
/// the identity), and the encoding of the result.
///
/// With an oracle, the lines of the G2 points are precomputed in affine coordinates from
/// checked hinted slopes (asked pair by pair, while the pairs are parsed), the G1 points are
/// normalized with hinted divisions, and the final exponentiation is replaced by a residue
/// witness check, or, for a product the prover claims not to be the identity, takes a checked
/// hinted inverse. A wrong hint either panics or is ignored (the slope words that a chain does
/// not use, or all of them for a chain the prover reports as exceptional), and it never changes
/// the result.
///
/// The pairs go into a vector of `allocator`; with an oracle, so do the lines of the G2 points
/// (16 KB each, built in place) and the normalized G1 points.
// zksync-os's signature: the wrapper maps any failure to its invalid point error
#[allow(clippy::result_unit_err)]
pub fn bn254_pairing_check<A: Allocator + Clone, O: HintOracle>(
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
    // with an oracle: the lines of the G2 points, precomputed in affine coordinates from hinted
    // slopes (less than half the multiplications), and the G1 points as the lines are
    // evaluated at them
    let with_advice = if oracle.is_some() { num_pairs } else { 0 };
    let mut prepared = Vec::with_capacity_in(with_advice, allocator.clone());
    let mut evaluations = Vec::with_capacity_in(with_advice, allocator.clone());
    let mut src_iter = src.iter();

    for _ in 0..num_pairs {
        let mut buffer = [0u8; 192];
        for (dst, src) in buffer.iter_mut().zip(&mut src_iter) {
            *dst = *src;
        }
        // NOTE: Ethereum serialization is strange: the imaginary parts of the G2 coordinates
        // come first
        let ([g1_x, g1_y, g2_x_c1, g2_x_c0, g2_y_c1, g2_y_c0], []) = buffer.as_chunks::<32>()
        else {
            unreachable!("192 bytes are 6 chunks of 32")
        };
        // an element of the base field from its big-endian encoding, which must be canonical
        let parse = |bytes: &[u8; 32]| Fq::from_bigint(bigint_from_be(bytes)).ok_or(());
        let g1_x = parse(g1_x)?;
        let g1_y = parse(g1_y)?;

        // the point at infinity is encoded as (0, 0); every other encoding must be a
        // point of the curve (and, for G2, of the subgroup)
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
            // e(P, O) = 1: the pair does not change the product; the G1 point was
            // validated above
            continue;
        }
        if !g2_point.is_on_curve() {
            return Err(());
        }
        // the slopes of the two affine chains of the point; whatever is not consumed of
        // them is skipped when they go out of scope
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
            // the lines of the point, built in the vector's next slot (they are 16 KB,
            // not to be moved), for the Miller loop that starts at the residue witness
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
        // the empty product is the identity of the target group
        return Ok(true);
    }

    let g1_iter = || pairs.iter().map(|(g1, _)| g1);
    let g2_iter = || pairs.iter().map(|(_, g2)| g2);
    let Some(oracle) = oracle else {
        let miller_loop = Bn254::multi_miller_loop(g1_iter(), g2_iter());
        // the Miller loop of curve points never evaluates to zero
        let result =
            Bn254::final_exponentiation(miller_loop).expect("the Miller loop output is invertible");
        return Ok(result.0.is_one());
    };
    debug_assert!(prepared.len() == pairs.len() && evaluations.len() == pairs.len());
    let normalized = || evaluations.iter().zip(prepared.iter());
    // The residue witness check in place of the final exponentiation (Novakovic, Eagen, "On
    // Proving Pairings", https://eprint.iacr.org/2024/640; soundness in the documentation of
    // `crate::residue_witness`): the Miller loop started at `d` gives `d^(6x+2) f`, the rest
    // is a few Frobenius maps. The prover claims the outcome; it cannot choose it: a claimed
    // identity must come with a witness that passes the check (a failed check is a broken
    // prover and panics), and a claimed non-identity is settled by the exact final
    // exponentiation, which finds an identity all the same.
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
    use crate::bn254::{Fr, G1Affine, G2Affine};
    use crate::hints::test_support::{lying_oracle, lying_oracle_last_word};
    use crate::hints::{FieldHintOp, NoHints, RecordingHintOracle};
    use crate::precompiles::HintedCases;
    use hex_literal::hex;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};
    use std::alloc::Global;
    use std::string::String;
    use std::vec;
    use std::vec::Vec;

    // from zksync-os e855bca2:basic_system/src/system_functions/bn254_pairing_check.rs

    /// The input of zksync-os's vector tests: two pairs, whose product is the identity
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

    #[test]
    fn test_pairing_inner() {
        let allocator = Global;

        let src = SRC;

        assert!(bn254_pairing_check::<_, NoHints>(2, src.as_slice(), allocator, None).unwrap());
        let mut oracle = native_oracle();
        assert!(bn254_pairing_check(2, src.as_slice(), allocator, Some(&mut oracle)).unwrap());
    }

    #[test]
    #[should_panic(expected = "claimed to be the identity is wrong")]
    fn test_pairing_wrong_identity_witness_panics() {
        let src = SRC;
        // a corrupted `c` with the identity claim fails the inverse pair check
        let mut lying = lying_oracle(&[FieldHintOp::Bn254PairingResidueWitness]);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            bn254_pairing_check(2, src.as_slice(), Global, Some(&mut lying))
        }));
        let message = result.unwrap_err();
        let message = message
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| message.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("");
        assert!(message.contains("not an inverse pair"), "{message}");
        // a corrupted scaling factor with the identity claim fails the residue check
        let mut lying = lying_oracle_last_word(&[FieldHintOp::Bn254PairingResidueWitness]);
        let _ = bn254_pairing_check(2, src.as_slice(), Global, Some(&mut lying));
    }

    fn native_oracle() -> RecordingHintOracle {
        RecordingHintOracle::new()
    }

    fn encode(pairs: &[(crate::bn254::G1Affine, crate::bn254::G2Affine)]) -> Vec<u8> {
        use crate::ark_ff::{BigInteger, PrimeField};
        let mut encoded = vec![];
        for (p, q) in pairs {
            let elements = if p.infinity {
                [crate::bn254::Fq::zero(); 2]
            } else {
                [p.x, p.y]
            }
            .into_iter()
            .chain(if q.infinity {
                [crate::bn254::Fq::zero(); 4]
            } else {
                [q.x.c1, q.x.c0, q.y.c1, q.y.c0]
            });
            for element in elements {
                encoded.extend(element.into_bigint().to_bytes_be());
            }
        }
        encoded
    }

    /// With the advice of the oracle (the affine chains with hinted slopes, the Miller loop
    /// over all the pairs with the residue witness) the results are the ones without it: for
    /// products that are the identity and that are not, with points at infinity, and for
    /// points of the twist outside the subgroup
    #[test]
    fn test_advice_matches_no_advice() {
        use crate::ark_ec::{AffineRepr, CurveGroup};
        use crate::bn254::{Fq, Fq2, Fr, G1Affine, G2Affine};
        let g1 = |k: u64| (G1Affine::generator() * Fr::from(k)).into_affine();
        let g2 = |k: u64| (G2Affine::generator() * Fr::from(k)).into_affine();
        // a point of the twist outside the subgroup: x = 1 has a y on the twist
        let outside = {
            use crate::ark_ec::short_weierstrass::SWCurveConfig;
            use crate::ark_ff::Field;
            let mut x = Fq2::new(Fq::from(1u64), Fq::zero());
            loop {
                let y2 = x * x * x + crate::bn254::g2::Config::COEFF_B;
                if let Some(y) = y2.sqrt() {
                    let q = G2Affine::new_unchecked(x, y);
                    if !q.is_in_correct_subgroup_assuming_on_curve() {
                        break q;
                    }
                }
                x.c0 += Fq::from(1u64);
            }
        };
        let inputs: Vec<Vec<(G1Affine, G2Affine)>> = vec![
            vec![(g1(6), g2(1)), ((-g1(2)), g2(3))],
            vec![
                (g1(6), g2(1)),
                ((-g1(2)), g2(3)),
                (g1(5), g2(7)),
                (g1(7), -g2(5)),
            ],
            vec![(g1(6), g2(1))],
            vec![(g1(6), g2(1)), (g1(2), g2(3))],
            vec![(G1Affine::identity(), g2(1)), (g1(3), G2Affine::identity())],
            vec![
                (g1(6), g2(1)),
                ((-g1(2)), g2(3)),
                (G1Affine::identity(), g2(9)),
            ],
            vec![(g1(6), outside)],
            vec![(G1Affine::identity(), outside)],
        ];
        for pairs in inputs {
            let src = encode(&pairs);
            let expected = bn254_pairing_check::<_, NoHints>(pairs.len(), &src, Global, None);
            let hinted = bn254_pairing_check(pairs.len(), &src, Global, Some(&mut native_oracle()));
            assert_eq!(hinted, expected);
            let is_outside = pairs.iter().any(|(_, q)| *q == outside);
            assert_eq!(expected.is_err(), is_outside);
        }
    }

    #[test]
    #[should_panic(expected = "division hint is wrong")]
    fn test_pairing_wrong_slope_panics() {
        use crate::ark_ec::AffineRepr;
        let src = encode(&[(
            crate::bn254::G1Affine::generator(),
            crate::bn254::G2Affine::generator(),
        )]);
        let mut lying = lying_oracle_last_word(&[FieldHintOp::Bn254G2PairingSlopes]);
        let _ = bn254_pairing_check(1, &src, Global, Some(&mut lying));
    }

    /// The wrapper's EVM half (zksync-os `bn254_pairing_check.rs:47-78`, without the gas and the
    /// cycle marker): the size of the input, the empty input, and the encoding of the result
    fn execute<O: HintOracle>(src: &[u8], oracle: Option<&mut O>) -> Result<Vec<u8>, ()> {
        let num_pairs = src.len() / 192;
        if !src.len().is_multiple_of(192) {
            return Err(());
        }
        let success = if src.is_empty() {
            true
        } else {
            bn254_pairing_check::<_, O>(num_pairs, src, Global, oracle)?
        };
        Ok(core::iter::repeat_n(0, 31)
            .chain(core::iter::once(success as u8))
            .collect())
    }

    #[test]
    fn test_pairing_external() {
        let src: &[u8] = &SRC;

        let expected = hex!("0000000000000000000000000000000000000000000000000000000000000001");

        let dst = execute::<NoHints>(src, None).unwrap();

        assert_eq!(expected, dst.as_slice());

        let mut oracle = native_oracle();
        let dst = execute(src, Some(&mut oracle)).unwrap();

        assert_eq!(expected, dst.as_slice());
    }

    // new tests

    fn g1(k: u64) -> G1Affine {
        (G1Affine::generator() * Fr::from(k)).into_affine()
    }

    fn g2(k: u64) -> G2Affine {
        (G2Affine::generator() * Fr::from(k)).into_affine()
    }

    /// The first point of the twist outside the subgroup from `x = 1` up (as in
    /// `test_advice_matches_no_advice`)
    fn outside() -> G2Affine {
        use crate::ark_ec::short_weierstrass::SWCurveConfig;
        use crate::ark_ff::Field;
        use crate::bn254::{Fq, Fq2};
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

    /// With and without the hints of an honest oracle the results are the same: for products
    /// that are the identity and that are not, with points at infinity, points of the twist
    /// outside the subgroup, and encodings that are mostly not of points of the curves
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

    /// The guest, which reads its answers off the recorded tape, gets the same result and takes
    /// the whole tape and no more. The bn254 elements of test builds have the representation of
    /// the guest, so the tape is also the answers this process reads.
    #[test]
    fn recorded_tape_replays() {
        // the identity, which the residue witness settles
        let identity =
            assert_tape_replays!(|oracle| bn254_pairing_check(2, &SRC, Global, Some(oracle)));
        assert_eq!(identity, Ok(true));
        // not the identity, which the exact final exponentiation settles
        let src = encode(&[(g1(6), g2(1)), (g1(2), g2(3))]);
        let other =
            assert_tape_replays!(|oracle| bn254_pairing_check(2, &src, Global, Some(oracle)));
        assert_eq!(other, Ok(false));
        // a point outside the subgroup, rejected while the pairs are parsed, after the hints of
        // the pair before it and its own slopes
        let src = encode(&[(g1(6), g2(1)), (g1(2), outside())]);
        let rejected =
            assert_tape_replays!(|oracle| bn254_pairing_check(2, &src, Global, Some(oracle)));
        assert_eq!(rejected, Err(()));
    }

    /// The hints of a pair are asked while the pairs are parsed, pair by pair: a pair that is
    /// rejected comes after the hints of the pairs before it, and there is no product to ask a
    /// residue witness for
    #[test]
    fn hints_are_asked_while_parsing() {
        let first = encode(&[(g1(6), g2(1))]);
        let rejected = encode(&[(g1(2), outside())]);
        let check = |src: &[u8]| {
            crate::hints::record_hints(|oracle| {
                bn254_pairing_check(src.len() / 192, src, Global, Some(oracle))
            })
        };
        let (_, first_tape) = check(&first);
        let (result, rejected_tape) = check(&rejected);
        assert_eq!(result, Err(()));
        assert!(!rejected_tape.is_empty());
        let (result, tape) = check(&[first, rejected].concat());
        assert_eq!(result, Err(()));
        // the slopes and the G1 divisions of the first pair, then the slopes of the second
        let (parsed, rest) = tape.split_at(tape.len() - rejected_tape.len());
        assert_eq!(rest, rejected_tape);
        assert!(!parsed.is_empty() && parsed.len() < first_tape.len());
        assert_eq!(parsed, &first_tape[..parsed.len()]);
    }
}
