//! The residue witnesses of the pairing checks (`crate::residue_witness`, after
//! <https://eprint.iacr.org/2024/640>), which replace the final exponentiations, and the slopes of
//! the affine `G2` chains of the bn254 pairing check.
//!
//! The witnesses are not checked here: the pairing check fails for any witness of a non-identity
//! product (see `crate::residue_witness`). A wrong hint fails an assertion or is ignored, and never
//! changes a result. Ignored words are `G2` slopes a chain does not use, or all of a chain the
//! prover reports as exceptional (the projective computation then runs instead).

#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use super::answer::{read_hint_answer, HintAnswer};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use super::request::{send_field_hint_query, FieldHintOp};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use airbender_oracle::MemoryOracle;
use ark_ff::Field;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use ark_ff::{One, Zero};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use core::mem::MaybeUninit;

/// A validated pair of the bls12-381 pairing; `repr(C)` fixes the guest layout.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bls12381Pair {
    pub g1: crate::bls12_381::G1Affine,
    pub g2: crate::bls12_381::G2Affine,
}

/// The prover's claim about a pairing product `f`, with the witness it takes to settle it
pub enum PairingClaim<F12, F6> {
    /// `f` is the identity: the residue witness `c`, `d = c^-1` (bls12-381 uses only `d`) and the
    /// scaling factor `s`, settled by the residue check
    Identity { c: F12, d: F12, s: F6 },
    /// `f` is not the identity: `f^-1`, for the exact final exponentiation, which settles the
    /// claim (so a false one cannot change the answer)
    NotIdentity { f_inverse: F12 },
}

/// The claim about the bn254 pairing product over validated `pairs`, for
/// `crate::residue_witness::bn254::check` on the Miller loop started at `d`. `c d = 1` is checked
/// here: the loop multiplies by `c` at negative digits, and the check is only sound for `c = d^-1`.
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub fn bn254_pairing_residue_witness<O: MemoryOracle>(
    oracle: &mut O,
    pairs: &[(crate::bn254::G1Affine, crate::bn254::G2Affine)],
) -> PairingClaim<crate::bn254::Fq12, crate::bn254::Fq6> {
    use crate::bn254::{Fq12, Fq6};
    // SAFETY: bn254 `(G1Affine, G2Affine)` pairs are the operand type of the op
    unsafe { send_field_hint_query(oracle, FieldHintOp::Bn254PairingResidueWitness, pairs) }
        .expect("must send the field hint query");
    let is_identity: bool = read_hint_answer(oracle).expect("the hint answer is well-formed");
    let claim = if is_identity {
        let (c, d, s): (Fq12, Fq12, Fq6) =
            read_hint_answer(oracle).expect("the hint answer is well-formed");
        PairingClaim::Identity { c, d, s }
    } else {
        PairingClaim::NotIdentity {
            f_inverse: read_hint_answer(oracle).expect("the hint answer is well-formed"),
        }
    };
    oracle
        .finish_query()
        .expect("the hint answer has no excess data");
    if let PairingClaim::Identity { c, d, .. } = &claim {
        assert!(
            (*c * d).is_one(),
            "the residue witness hint is not an inverse pair"
        );
    }
    claim
}

/// The hinted slopes of the affine `G2` chains of one point (`crate::bn254::curves::g2_affine`),
/// received as the chains consume them: a flag and the membership-test slopes, then a flag and the
/// line slopes. A cleared flag marks an exceptional chain, whose words are skipped. Each slope is
/// checked against its fraction. The unread rest is skipped on drop and the query ended, so the
/// oracle stays in step on every path.
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub struct G2SlopeHints<'a, O: MemoryOracle> {
    oracle: &'a mut O,
    /// Words of the answer not received yet
    remaining_words: usize,
}

/// Oracle words of one `Fq2` hint
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
const FQ2_HINT_WORDS: usize = 2 * 2 * 4;

/// Oracle words of the whole answer: two flags and the slopes
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
const G2_HINT_WORDS: usize = 2
    + (crate::bn254::curves::g2_affine::SUBGROUP_DIVISIONS
        + crate::bn254::curves::g2_affine::LINE_DIVISIONS)
        * FQ2_HINT_WORDS;

/// Asks for the slopes of the affine `G2` chains of `q` (on the twist, not at infinity)
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub fn bn254_g2_pairing_slopes<'a, O: MemoryOracle>(
    oracle: &'a mut O,
    q: &crate::bn254::G2Affine,
) -> G2SlopeHints<'a, O> {
    // SAFETY: a bn254 `G2Affine` is the operand type of the op
    unsafe { send_field_hint_query(oracle, FieldHintOp::Bn254G2PairingSlopes, q) }
        .expect("must send the field hint query");
    G2SlopeHints {
        oracle,
        remaining_words: G2_HINT_WORDS,
    }
}

#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
impl<'a, O: MemoryOracle> G2SlopeHints<'a, O> {
    /// The slopes of the membership test, `None` if the prover reports an exceptional chain
    pub fn subgroup_chain(&mut self) -> Option<HintedChain<'_, 'a, O>> {
        self.chain(crate::bn254::curves::g2_affine::SUBGROUP_DIVISIONS)
    }

    /// The slopes of the line precomputation (after the membership test's), `None` if the prover
    /// reports an exceptional chain
    pub fn line_chain(&mut self) -> Option<HintedChain<'_, 'a, O>> {
        self.chain(crate::bn254::curves::g2_affine::LINE_DIVISIONS)
    }

    fn chain(&mut self, slopes: usize) -> Option<HintedChain<'_, 'a, O>> {
        self.remaining_words -= 1;
        let present: bool =
            read_hint_answer(self.oracle).expect("the hint response has the flag of the chain");
        if !present {
            self.skip(slopes * FQ2_HINT_WORDS);
            return None;
        }
        Some(HintedChain {
            hints: self,
            remaining: slopes,
        })
    }

    fn skip(&mut self, words: usize) {
        self.remaining_words -= words;
        for _ in 0..words {
            self.oracle
                .read_word()
                .expect("the hint response has the words of the chain");
        }
    }
}

#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
impl<O: MemoryOracle> Drop for G2SlopeHints<'_, O> {
    fn drop(&mut self) {
        self.skip(self.remaining_words);
        self.oracle
            .finish_query()
            .expect("the hint answer has no excess data");
    }
}

/// The hints of one chain, in the chain's order
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub struct HintedChain<'c, 'a, O: MemoryOracle> {
    hints: &'c mut G2SlopeHints<'a, O>,
    remaining: usize,
}

#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
impl<O: MemoryOracle> crate::affine_glv::Divider<crate::bn254::Fq2> for HintedChain<'_, '_, O> {
    #[inline(always)]
    fn divide<'q>(
        &mut self,
        fraction: &mut [crate::bn254::Fq2; 2],
        quotient: &'q mut MaybeUninit<crate::bn254::Fq2>,
    ) -> &'q mut crate::bn254::Fq2 {
        assert!(self.remaining > 0, "the chain is longer than its hints");
        self.remaining -= 1;
        self.hints.remaining_words -= FQ2_HINT_WORDS;
        let quotient = <crate::bn254::Fq2 as HintAnswer>::read(self.hints.oracle, quotient)
            .expect("the hint is a canonical field element");
        assert!(
            crate::bn254::curves::g2_affine::verify_quotient(fraction, quotient),
            "the field division hint is wrong"
        );
        quotient
    }
}

#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
impl<O: MemoryOracle> Drop for HintedChain<'_, '_, O> {
    /// The rest of an abandoned chain is skipped
    fn drop(&mut self) {
        self.hints.skip(self.remaining * FQ2_HINT_WORDS);
    }
}

/// The claim about `e(p1, G2) e(p2, tau G2)` for `points = [p1, p2]` (the KZG proof check), for
/// `crate::residue_witness::bls12_381::check` on the Miller loop started at `d` (`c` is zero)
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub fn bls12_381_kzg_residue_witness<O: MemoryOracle>(
    oracle: &mut O,
    points: &[crate::bls12_381::G1Affine; 2],
) -> PairingClaim<crate::bls12_381::Fq12, crate::bls12_381::Fq6> {
    use crate::bls12_381::{Fq12, Fq6};
    // SAFETY: `[bls12_381::G1Affine; 2]` is the operand type of the op
    unsafe { send_field_hint_query(oracle, FieldHintOp::Bls12381KzgResidueWitness, points) }
        .expect("must send the field hint query");
    let is_identity: bool = read_hint_answer(oracle).expect("the hint answer is well-formed");
    let claim = if is_identity {
        let (d, s): (Fq12, Fq6) = read_hint_answer(oracle).expect("the hint answer is well-formed");
        PairingClaim::Identity {
            c: Fq12::zero(),
            d,
            s,
        }
    } else {
        PairingClaim::NotIdentity {
            f_inverse: read_hint_answer(oracle).expect("the hint answer is well-formed"),
        }
    };
    oracle
        .finish_query()
        .expect("the hint answer has no excess data");
    claim
}

/// The claim about the bls12-381 pairing product over validated `pairs` (`c` is zero)
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub fn bls12_381_pairing_residue_witness<O: MemoryOracle>(
    oracle: &mut O,
    pairs: &[Bls12381Pair],
) -> PairingClaim<crate::bls12_381::Fq12, crate::bls12_381::Fq6> {
    use crate::bls12_381::{Fq12, Fq6};
    // SAFETY: `Bls12381Pair`s are the operand type of the op
    unsafe { send_field_hint_query(oracle, FieldHintOp::Bls12381PairingResidueWitness, pairs) }
        .expect("must send the field hint query");
    let identity: bool = read_hint_answer(oracle).expect("the hint answer is well-formed");
    let claim = if identity {
        let (d, s): (Fq12, Fq6) = read_hint_answer(oracle).expect("the hint answer is well-formed");
        PairingClaim::Identity {
            c: Fq12::zero(),
            d,
            s,
        }
    } else {
        PairingClaim::NotIdentity {
            f_inverse: read_hint_answer(oracle).expect("the hint answer is well-formed"),
        }
    };
    oracle
        .finish_query()
        .expect("the hint answer has no excess data");
    claim
}

/// The hinted inverse of the Miller loop output `f`, checked
pub fn checked_inverse<F: Field>(f: &F, f_inverse: F) -> Option<F> {
    assert!(
        (*f * f_inverse).is_one(),
        "the Miller loop inverse hint is wrong"
    );
    Some(f_inverse)
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::affine_glv::{Divider, InvertingDivider};
    use crate::bn254::curves::g2_affine;
    use crate::hints::host::{record_hints, RecordingHintOracle};
    use crate::hints::test_support::{
        lying_oracle, lying_oracle_last_word, non_identity_claiming_oracle, TapeOracle,
    };
    use crate::{bls12_381, bn254};
    use ark_ec::pairing::Pairing;
    use ark_ec::{AffineRepr, CurveGroup};

    fn bn254_g1(k: u64) -> bn254::G1Affine {
        (bn254::G1Affine::generator() * bn254::Fr::from(k)).into_affine()
    }

    fn bn254_g2(k: u64) -> bn254::G2Affine {
        (bn254::G2Affine::generator() * bn254::Fr::from(k)).into_affine()
    }

    /// `e(6 G1, G2) e(-2 G1, 3 G2)`, the identity
    fn bn254_identity() -> [(bn254::G1Affine, bn254::G2Affine); 2] {
        [(bn254_g1(6), bn254_g2(1)), (-bn254_g1(2), bn254_g2(3))]
    }

    /// The Miller loop of `pairs` as the verifier computes it, started at `initial`
    fn bn254_miller_loop(
        initial: Option<(&bn254::Fq12, &bn254::Fq12)>,
        pairs: &[(bn254::G1Affine, bn254::G2Affine)],
    ) -> bn254::Fq12 {
        use crate::bn254::curves::{Bn254, G1Evaluation, G2PreparedNoAlloc};
        let prepared: Vec<G2PreparedNoAlloc> = pairs
            .iter()
            .map(|(_, q)| g2_affine::prepare_as_verifier(q))
            .collect();
        let evaluations: Vec<G1Evaluation> = pairs
            .iter()
            .map(|(p, _)| G1Evaluation::new(p, &mut InvertingDivider))
            .collect();
        Bn254::multi_miller_loop_normalized(initial, evaluations.iter().zip(prepared.iter()))
    }

    #[test]
    fn bn254_claims_are_settled_by_their_witnesses() {
        let identity = bn254_identity();
        let other = [(bn254_g1(6), bn254_g2(1))];
        let mut oracle = RecordingHintOracle::new();
        let PairingClaim::Identity { c, d, s } =
            bn254_pairing_residue_witness(&mut oracle, &identity)
        else {
            panic!("the product is the identity");
        };
        let l = bn254_miller_loop(Some((&d, &c)), &identity);
        assert!(crate::residue_witness::bn254::check(&l, &d, &s));
        let PairingClaim::NotIdentity { f_inverse } =
            bn254_pairing_residue_witness(&mut oracle, &other)
        else {
            panic!("the product is not the identity");
        };
        let f = bn254_miller_loop(None, &other);
        assert_eq!(checked_inverse(&f, f_inverse), Some(f_inverse));
        // a false non-identity claim comes with the inverse the exact path checks
        let PairingClaim::NotIdentity { f_inverse } =
            bn254_pairing_residue_witness(&mut non_identity_claiming_oracle(), &identity)
        else {
            panic!("the oracle claims a non-identity");
        };
        let f = bn254_miller_loop(None, &identity);
        assert_eq!(checked_inverse(&f, f_inverse), Some(f_inverse));
    }

    #[test]
    #[should_panic(expected = "the residue witness hint is not an inverse pair")]
    fn wrong_bn254_residue_witness_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Bn254PairingResidueWitness]);
        let _ = bn254_pairing_residue_witness(&mut oracle, &bn254_identity());
    }

    #[test]
    #[should_panic(expected = "the Miller loop inverse hint is wrong")]
    fn wrong_miller_loop_inverse_is_rejected() {
        let f = bls12_381::Fq12::from(3u64);
        let _ = checked_inverse(&f, f);
    }

    /// The Miller loop of `e(p1, G2) e(p2, tau G2)` as the verifier computes it, started at
    /// `initial`
    fn kzg_miller_loop(
        initial: Option<&bls12_381::Fq12>,
        points: &[bls12_381::G1Affine; 2],
    ) -> bls12_381::Fq12 {
        use crate::bls12_381::consts::{PREPARED_G2_BY_TAU, PREPARED_G2_GENERATOR};
        let g2 = [&PREPARED_G2_GENERATOR, &PREPARED_G2_BY_TAU];
        bls12_381::curves::Bls12_381::multi_miller_loop_shared(initial, points.iter().zip(g2))
    }

    #[test]
    fn kzg_claims_are_settled_by_their_witnesses() {
        let infinity = bls12_381::G1Affine::identity();
        let identity = [infinity, infinity];
        let other = [bls12_381::G1Affine::generator(), infinity];
        let mut oracle = RecordingHintOracle::new();
        let PairingClaim::Identity { c, d, s } =
            bls12_381_kzg_residue_witness(&mut oracle, &identity)
        else {
            panic!("the product is the identity");
        };
        assert!(c.is_zero());
        let l = kzg_miller_loop(Some(&d), &identity);
        assert!(crate::residue_witness::bls12_381::check(&l, &d, &s));
        // the inverses are of the conjugated loop outputs
        let PairingClaim::NotIdentity { f_inverse } =
            bls12_381_kzg_residue_witness(&mut oracle, &other)
        else {
            panic!("the product is not the identity");
        };
        let mut f = kzg_miller_loop(None, &other);
        f.conjugate_in_place();
        assert_eq!(checked_inverse(&f, f_inverse), Some(f_inverse));
    }

    #[test]
    fn bls12_381_claims_are_settled_by_their_witnesses() {
        use crate::bls12_381::curves::Bls12_381;
        let p = bls12_381::G1Affine::generator();
        let q = (bls12_381::G2Affine::generator() * bls12_381::Fr::from(5u64)).into_affine();
        let identity = [
            Bls12381Pair { g1: p, g2: q },
            Bls12381Pair { g1: -p, g2: q },
        ];
        let other = [Bls12381Pair { g1: p, g2: q }];
        let g1 = |pairs: &[Bls12381Pair]| pairs.iter().map(|pair| pair.g1).collect::<Vec<_>>();
        let g2 = |pairs: &[Bls12381Pair]| pairs.iter().map(|pair| pair.g2).collect::<Vec<_>>();
        let mut oracle = RecordingHintOracle::new();
        let PairingClaim::Identity { c, d, s } =
            bls12_381_pairing_residue_witness(&mut oracle, &identity)
        else {
            panic!("the product is the identity");
        };
        assert!(c.is_zero());
        let l = Bls12_381::multi_miller_loop_with_initial_owned(&d, g1(&identity), g2(&identity));
        assert!(crate::residue_witness::bls12_381::check(&l, &d, &s));
        let PairingClaim::NotIdentity { f_inverse } =
            bls12_381_pairing_residue_witness(&mut oracle, &other)
        else {
            panic!("the product is not the identity");
        };
        let f = Bls12_381::multi_miller_loop(g1(&other), g2(&other)).0;
        assert_eq!(checked_inverse(&f, f_inverse), Some(f_inverse));
    }

    /// Divides with the first `hinted` quotients of `chain`, and with inversions after them
    struct FirstHinted<D> {
        chain: D,
        hinted: usize,
    }

    impl<D: Divider<bn254::Fq2>> Divider<bn254::Fq2> for FirstHinted<D> {
        fn divide<'q>(
            &mut self,
            fraction: &mut [bn254::Fq2; 2],
            quotient: &'q mut MaybeUninit<bn254::Fq2>,
        ) -> &'q mut bn254::Fq2 {
            if self.hinted == 0 {
                return InvertingDivider.divide(fraction, quotient);
            }
            self.hinted -= 1;
            self.chain.divide(fraction, quotient)
        }
    }

    /// How many quotients of the membership test chain, then of the line chain, a querier takes
    /// (`None`: it does not take the chain)
    type Way = (Option<usize>, Option<usize>);

    const ALL: Way = (
        Some(g2_affine::SUBGROUP_DIVISIONS),
        Some(g2_affine::LINE_DIVISIONS),
    );

    /// Takes the slopes of `q` in the way `(subgroup, line)`, and checks the chains' results
    fn take_slopes<O: MemoryOracle>(oracle: &mut O, q: &bn254::G2Affine, (subgroup, line): Way) {
        let mut hints = bn254_g2_pairing_slopes(oracle, q);
        if let Some(hinted) = subgroup {
            let chain = hints
                .subgroup_chain()
                .expect("the chain of a subgroup point");
            let is_member = g2_affine::membership(q, &mut FirstHinted { chain, hinted });
            assert_eq!(is_member, Some(true));
        }
        if let Some(hinted) = line {
            let chain = hints.line_chain().expect("the chain of a subgroup point");
            let prepared = g2_affine::prepare(q, &mut FirstHinted { chain, hinted })
                .expect("the chain of a subgroup point");
            assert_eq!(
                prepared.ell_coeffs,
                g2_affine::prepare_as_verifier(q).ell_coeffs
            );
        }
        // dropping the chains and `hints` skips the rest of the answer
    }

    #[test]
    fn the_whole_slope_answer_is_taken_however_much_is_used() {
        const NEXT_ANSWER: u32 = 0x5eed_5eed;
        let q = bn254_g2(7);
        for way in [(None, None), (Some(5), Some(3)), ALL] {
            let (_, tape) = record_hints(|recorder| {
                take_slopes(recorder, &q, way);
                // nothing of the answer of this process is left unread
                assert_eq!(recorder.finish_query(), Ok(()));
            });
            assert_eq!(tape.len(), G2_HINT_WORDS, "{way:?}");
            // the guest reads the whole answer off the tape, and no more
            let mut guest = TapeOracle::new(tape.into_iter().chain([NEXT_ANSWER]));
            take_slopes(&mut guest, &q, way);
            assert_eq!(guest.words, [NEXT_ANSWER], "{way:?}");
            assert_eq!(guest.queries.len(), 1);
        }
    }

    #[test]
    #[should_panic(expected = "the field division hint is wrong")]
    fn wrong_slope_is_rejected() {
        // the last slope of the line chain
        let mut oracle = lying_oracle_last_word(&[FieldHintOp::Bn254G2PairingSlopes]);
        take_slopes(&mut oracle, &bn254_g2(7), ALL);
    }

    #[test]
    fn a_cleared_flag_skips_the_chain() {
        // the membership test flag is cleared: its words are skipped, then the line chain is read
        let mut oracle = lying_oracle(&[FieldHintOp::Bn254G2PairingSlopes]);
        let q = bn254_g2(7);
        let mut hints = bn254_g2_pairing_slopes(&mut oracle, &q);
        assert!(hints.subgroup_chain().is_none());
        let chain = hints
            .line_chain()
            .expect("the flag of the line chain is set");
        let prepared = g2_affine::prepare(
            &q,
            &mut FirstHinted {
                chain,
                hinted: g2_affine::LINE_DIVISIONS,
            },
        )
        .expect("the chain of a subgroup point");
        assert_eq!(
            prepared.ell_coeffs,
            g2_affine::prepare_as_verifier(&q).ell_coeffs
        );
        // ends the query, which the oracle checks was read in full
        drop(hints);
    }
}
