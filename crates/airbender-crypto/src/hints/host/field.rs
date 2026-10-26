// Ported from zksync-os e855bca2:callable_oracles/src/field_hints/mod.rs
// Ported from zksync-os e855bca2:callable_oracles/src/field_hints/impls.rs
//! The answers to the field hints (square roots, inverses and quotients of the secp256k1, bn254
//! and bls12-381 fields).
//!
//! The hints are served with the memory-based protocol: the input word is the address of the
//! request, which points to the operand where the querier keeps it. The field elements of the
//! operands and of the answers are in the representation of the target of the querier (see
//! `FieldHintOp`, `HintAnswer` and `Secp256k1Element`), which differs between the RISC-V guest and
//! native builds. The responses asked for tell which the querier is, and whether the guest is
//! answered too, when a native run records the prover input (see `answer_query`).

use super::read_querier_u32_words;
use crate::hints::{
    guest_layout, read_field_hint_request, FieldHintOp, HintAnswer, HintEncoding, HintTarget,
    QuerierMemory, Secp256k1Element, FIELD_OPS_ADVISE_QUERY_ID,
};
use crate::secp256k1::field::FieldElement;
use crate::secp256k1::scalars::Scalar;
use alloc::vec::Vec;
use ark_ec::short_weierstrass::{Affine, SWCurveConfig};
use ark_ff::{Field, One, PrimeField, Zero};

/// The op and the operand of the hint request at `input_word` in the memory of `querier`
///
/// # Safety
///
/// For a querier in this process (`HintTarget::Native`), `memory` must be the memory of this
/// process, and `input_word` the address of a live request whose operand references live values
/// of the operand types of its op (see `Operand::new`).
unsafe fn read_request<'m>(
    memory: &'m dyn QuerierMemory,
    input_word: usize,
    querier: HintTarget,
) -> (FieldHintOp, Operand<'m>) {
    let (op, address, len_u32_words) =
        read_field_hint_request(memory, input_word).expect("must read the field hint request");
    let Some(op) = FieldHintOp::parse_u32(op) else {
        panic!("Unknown field hint op {op}");
    };
    // SAFETY: guaranteed by the caller: the operand of a querier in this process is live values
    // of the operand types of `op`
    (op, unsafe {
        Operand::new(memory, querier, address, len_u32_words)
    })
}

/// The responses of a processor to a hint query: the answer as a querier in this process reads
/// it (a native run), and as the RISC-V guest reads it, for those of the runs it is produced for
struct Responses<'a> {
    native_run: Option<&'a mut Vec<u32>>,
    guest_run: Option<&'a mut Vec<u32>>,
}

impl Responses<'_> {
    fn write<T: HintAnswer>(&mut self, answer: &T) {
        if let Some(response) = self.native_run.as_deref_mut() {
            answer.write(HintTarget::Native, response);
        }
        if let Some(response) = self.guest_run.as_deref_mut() {
            answer.write(HintTarget::Guest, response);
        }
    }
}

/// The answer to the hint `op` on `operand`
fn answer(op: FieldHintOp, operand: &Operand, responses: &mut Responses) {
    match op {
        FieldHintOp::Secp256k1BaseFieldSqrt => responses.write(&secp256k1_base_field_sqrt(operand)),
        FieldHintOp::Secp256k1BaseFieldInverse => {
            responses.write(&secp256k1_base_field_inverse(operand))
        }
        FieldHintOp::Secp256k1ScalarFieldInverse => {
            responses.write(&secp256k1_scalar_field_inverse(operand))
        }
        FieldHintOp::Secp256k1BaseFieldDivision => {
            responses.write(&secp256k1_base_field_division(operand))
        }
        FieldHintOp::Bn254BaseFieldInverse => {
            responses.write(&inverse::<crate::bn254::Fq>(operand))
        }
        FieldHintOp::Bn254BaseFieldDivision => {
            responses.write(&division::<crate::bn254::Fq>(operand))
        }
        FieldHintOp::Bn254Fq12Inverse => responses.write(&inverse::<crate::bn254::Fq12>(operand)),
        FieldHintOp::Bls12381BaseFieldSqrt => responses.write(&bls12_381_base_field_sqrt(operand)),
        FieldHintOp::Bls12381BaseFieldInverse => {
            responses.write(&inverse::<crate::bls12_381::Fq>(operand))
        }
        FieldHintOp::Bls12381BaseFieldDivision => {
            responses.write(&division::<crate::bls12_381::Fq>(operand))
        }
        FieldHintOp::Bls12381Fq12Inverse => {
            responses.write(&inverse::<crate::bls12_381::Fq12>(operand))
        }
        FieldHintOp::Bn254PairingResidueWitness => {
            bn254_pairing_residue_witness(operand, false, responses)
        }
        FieldHintOp::Bn254G2PairingSlopes => bn254_g2_pairing_slopes(operand, responses),
        FieldHintOp::Bls12381KzgResidueWitness => {
            bls12_381_kzg_residue_witness(operand, false, responses)
        }
        FieldHintOp::Bls12381PairingResidueWitness => {
            bls12_381_pairing_residue_witness(operand, false, responses)
        }
    }
}

/// The answer, as a querier in this process reads it, to its pairing request (the bn254 pairing,
/// the KZG proof pairing, or the BLS12-381 pairing: `Bn254PairingResidueWitness`,
/// `Bls12381KzgResidueWitness` or `Bls12381PairingResidueWitness`) at `input_word` that claims a
/// non-identity, with the inverse the exact path needs, whatever the product is: for tests of
/// that path
///
/// # Panics
///
/// Panics if the request is not one of those three ops, or if it is malformed (as
/// [`answer_query`](super::answer_query) does).
///
/// # Safety
///
/// `memory` must be the memory of this process, and `input_word` the address of a live request
/// whose operand references live values of the operand types of its op, for the duration of the
/// call (the contract of [`answer_query`](super::answer_query) for a querier in this process): the
/// operand is read as values of those types, in place.
pub unsafe fn not_identity_claim(memory: &dyn QuerierMemory, input_word: usize) -> Vec<u32> {
    let mut response = Vec::new();
    let mut responses = Responses {
        native_run: Some(&mut response),
        guest_run: None,
    };
    // SAFETY: guaranteed by the caller
    match unsafe { read_request(memory, input_word, HintTarget::Native) } {
        (FieldHintOp::Bn254PairingResidueWitness, operand) => {
            bn254_pairing_residue_witness(&operand, true, &mut responses)
        }
        (FieldHintOp::Bls12381KzgResidueWitness, operand) => {
            bls12_381_kzg_residue_witness(&operand, true, &mut responses)
        }
        (FieldHintOp::Bls12381PairingResidueWitness, operand) => {
            bls12_381_pairing_residue_witness(&operand, true, &mut responses)
        }
        (op, _) => panic!("not a pairing request: {op:?}"),
    }
    response
}

/// Serves a hint query for the runs whose responses are asked for (see `answer_query`)
///
/// # Safety
///
/// As for `answer_query`.
pub(super) unsafe fn process_field_hint_query(
    query_id: u32,
    input_word: usize,
    memory: &dyn QuerierMemory,
    native_run_responses: Option<&mut Vec<u32>>,
    guest_run_responses: Option<&mut Vec<u32>>,
) {
    assert_eq!(query_id, FIELD_OPS_ADVISE_QUERY_ID);
    // the querier, whose memory holds the request and the operand, in its representation: as in
    // the run modes of zksync-os, the runs whose responses are asked for tell it, a native run
    // (only, or saving the responses for the RISC-V guest) answering a querier in this process
    let (querier, word_size) = if native_run_responses.is_some() {
        (HintTarget::Native, size_of::<usize>())
    } else {
        (HintTarget::Guest, size_of::<u32>())
    };
    assert_eq!(
        memory.word_size(),
        word_size,
        "not the querier of the run mode (a {querier:?} querier)"
    );
    let mut responses = Responses {
        native_run: native_run_responses,
        guest_run: guest_run_responses,
    };
    // SAFETY: the querier is in this process exactly when the native responses are asked for,
    // and then its memory and its request are as `read_request` needs (the caller's obligation)
    let (op, operand) = unsafe { read_request(memory, input_word, querier) };
    answer(op, &operand, &mut responses);
}

/// The operand of a hint query where the querier keeps it: its address and size in the memory of
/// the querier, in the representation of the target of the querier (see `FieldHintOp`)
pub(crate) struct Operand<'m> {
    memory: &'m dyn QuerierMemory,
    querier: HintTarget,
    address: usize,
    len_u32_words: u32,
}

impl<'m> Operand<'m> {
    /// # Safety
    ///
    /// For a querier in this process (`HintTarget::Native`), `address` must be the address of
    /// `len_u32_words` words of live values of the operand types of the op of the request, which
    /// stay alive and unchanged while the operand is used: they are read as values of those types,
    /// in place (see `native_values`).
    pub(crate) unsafe fn new(
        memory: &'m dyn QuerierMemory,
        querier: HintTarget,
        address: usize,
        len_u32_words: u32,
    ) -> Self {
        assert!(address != 0, "the operand is not at the null address");
        Self {
            memory,
            querier,
            address,
            len_u32_words,
        }
    }

    /// The size of the operand in bytes
    fn size(&self) -> usize {
        self.len_u32_words as usize * size_of::<u32>()
    }

    /// The operand of a querier in this process, values of `T`
    fn native_values<T: Copy>(&self) -> Vec<T> {
        assert_eq!(self.querier, HintTarget::Native);
        assert!(
            self.size().is_multiple_of(size_of::<T>()),
            "the operand is values of the type"
        );
        assert!(
            self.address.is_multiple_of(align_of::<T>()),
            "the operand is aligned for the type"
        );
        let values = core::ptr::with_exposed_provenance::<T>(self.address);
        // SAFETY: the querier, code of this process, exposes its operand, live values of the
        // operand type of the op, which is `T` here, while it sends the query, i.e. during this
        // call (the obligation of the creator of the operand, see `Operand::new`); the size and
        // the alignment are checked above
        unsafe { core::slice::from_raw_parts(values, self.size() / size_of::<T>()) }.to_vec()
    }

    /// The operand of a querier in this process, a `T`
    fn native_value<T: Copy>(&self) -> T {
        assert_eq!(
            self.size(),
            size_of::<T>(),
            "the operand is a value of the type"
        );
        self.native_values::<T>()[0]
    }

    /// `len` words at `offset` bytes into the operand of the RISC-V guest
    fn guest_words(&self, offset: usize, len: usize) -> Vec<u32> {
        assert_eq!(self.querier, HintTarget::Guest);
        assert!(offset + 4 * len <= self.size(), "within the operand");
        read_querier_u32_words(self.memory, self.address + offset, len)
            .expect("must read the operand")
    }

    /// The byte at `offset` into the operand of the RISC-V guest
    fn guest_byte(&self, offset: usize) -> u8 {
        self.guest_words(offset & !3, 1)[0].to_le_bytes()[offset & 3]
    }

    /// The field element at `offset` bytes into the operand of the RISC-V guest (see
    /// `hints::guest_layout`)
    fn guest_element<F: HintEncoding>(&self, offset: usize) -> F {
        let words = self.guest_words(offset, 2 * F::TOTAL_LIMBS);
        let components: Vec<F::BasePrimeField> = words
            .chunks(2 * F::LIMBS)
            .map(from_guest_montgomery)
            .collect();
        F::from_components(&components)
    }

    /// The affine point at `offset` bytes into the operand of the RISC-V guest
    fn guest_affine<P: SWCurveConfig>(
        &self,
        offset: usize,
        layout: guest_layout::AffinePoint,
    ) -> Affine<P>
    where
        P::BaseField: HintEncoding,
    {
        if self.guest_byte(offset + layout.infinity) != 0 {
            return Affine::identity();
        }
        Affine::new_unchecked(
            self.guest_element(offset + layout.x),
            self.guest_element(offset + layout.y),
        )
    }
}

/// The base field element of its Montgomery limbs on the RISC-V guest (`R = 2^(32 words)`), any
/// representative
fn from_guest_montgomery<F: PrimeField>(words: &[u32]) -> F {
    let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let r = F::from(2u64).pow([32 * words.len() as u64]);
    F::from_le_bytes_mod_order(&bytes) * r.inverse().expect("a power of two is invertible")
}

/// A field element operand
fn element<F: HintEncoding + Copy>(operand: &Operand) -> F {
    match operand.querier {
        HintTarget::Native => operand.native_value(),
        HintTarget::Guest => {
            assert_eq!(
                operand.size(),
                8 * F::TOTAL_LIMBS,
                "the operand is an element"
            );
            operand.guest_element(0)
        }
    }
}

/// A secp256k1 element operand (see `Secp256k1Element`)
fn secp256k1_element<T: Secp256k1Element>(operand: &Operand) -> T {
    match operand.querier {
        HintTarget::Native => operand.native_value(),
        HintTarget::Guest => {
            assert_eq!(operand.size(), 32, "the operand is an element");
            let words: [u32; 8] = operand.guest_words(0, 8).try_into().expect("8 words");
            T::from_guest_operand_words(&words)
        }
    }
}

/// Computes the square root candidate for a secp256k1 base field element.
///
/// Returns `(candidate, is_quadratic_non_residue)` where:
/// - `candidate` is `input^((p+1)/4)` (the square root if one exists)
/// - `is_quadratic_non_residue` is `true` if `input` has no square root in the field
///
/// When `is_quadratic_non_residue` is false: `candidate² == input`
/// When `is_quadratic_non_residue` is true:  `candidate² == -input`
pub(crate) fn secp256k1_base_field_sqrt(operand: &Operand) -> (FieldElement, bool) {
    let el: FieldElement = secp256k1_element(operand);
    assert!(!el.is_zero());
    let mut candidate = el;
    // sqrt_in_place returns true if the input is a quadratic residue (has a square root)
    let is_quadratic_residue = candidate.sqrt_in_place();
    (candidate, !is_quadratic_residue)
}

pub(crate) fn secp256k1_base_field_inverse(operand: &Operand) -> FieldElement {
    let mut el: FieldElement = secp256k1_element(operand);
    assert!(!el.is_zero());
    el.invert_in_place();
    el
}

/// The quotient of the numerator and the (non-zero) denominator, two adjacent elements
pub(crate) fn secp256k1_base_field_division(operand: &Operand) -> FieldElement {
    let [numerator, denominator]: [FieldElement; 2] = match operand.querier {
        HintTarget::Native => operand.native_value(),
        HintTarget::Guest => {
            assert_eq!(operand.size(), 64, "the operand is two elements");
            core::array::from_fn(|i| {
                let words: [u32; 8] = operand.guest_words(32 * i, 8).try_into().expect("8 words");
                FieldElement::from_guest_operand_words(&words)
            })
        }
    };
    assert!(!denominator.is_zero());
    let mut quotient = denominator;
    quotient.invert_in_place();
    quotient.mul_in_place(&numerator);
    quotient
}

pub(crate) fn secp256k1_scalar_field_inverse(operand: &Operand) -> Scalar {
    let el: Scalar = secp256k1_element(operand);
    assert!(!el.is_zero());
    el.invert_vartime()
}

/// The inverse of a non-zero field element
pub(crate) fn inverse<F: HintEncoding + Copy>(operand: &Operand) -> F {
    let el: F = element(operand);
    crate::field_inverse::inverse(&el).expect("the operand is non-zero")
}

/// The quotient of the numerator and the (non-zero) denominator, two adjacent elements of a
/// base prime field
pub(crate) fn division<F: HintEncoding + PrimeField>(operand: &Operand) -> F {
    let [numerator, denominator]: [F; 2] = match operand.querier {
        HintTarget::Native => operand.native_value(),
        HintTarget::Guest => {
            assert_eq!(operand.size(), 16 * F::LIMBS, "the operand is two elements");
            // Montgomery limbs, any representatives: their factor `R` cancels in the quotient
            let words = operand.guest_words(0, 4 * F::LIMBS);
            let mut halves = words.chunks(2 * F::LIMBS).map(|words| {
                let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
                F::from_le_bytes_mod_order(&bytes)
            });
            [
                halves.next().expect("two elements"),
                halves.next().expect("two elements"),
            ]
        }
    };
    numerator * crate::field_inverse::inverse(&denominator).expect("the denominator is non-zero")
}

/// The square root candidate of a bls12-381 base field element, as `secp256k1_base_field_sqrt`:
/// `(candidate, is_quadratic_non_residue)` with `candidate² == input` for a square and
/// `candidate² == -input` otherwise (`p = 3 mod 4`, so exactly one of them is a square)
pub(crate) fn bls12_381_base_field_sqrt(operand: &Operand) -> (crate::bls12_381::Fq, bool) {
    let el: crate::bls12_381::Fq = element(operand);
    assert!(!el.is_zero());
    match el.sqrt() {
        Some(root) => (root, false),
        None => {
            let root = (-el).sqrt().expect("-1 is not a square, so -el is one");
            (root, true)
        }
    }
}

/// The claim about the bn254 pairing product over the affine pairs of the operand, see
/// `hints::PairingClaim`: the identity flag, then `c`, `d = c^-1` and the scaling factor
/// (`crate::residue_witness::bn254`) for an identity, or the inverse of the Miller loop output
/// otherwise. `claim_not_identity` forces the latter (for tests of the exact path).
fn bn254_pairing_residue_witness(
    operand: &Operand,
    claim_not_identity: bool,
    responses: &mut Responses,
) {
    use crate::bn254::curves::{Bn254, G1Evaluation, G2PreparedNoAlloc};
    use crate::bn254::{G1Affine, G2Affine};
    use ark_ec::pairing::{MillerLoopOutput, Pairing};
    use guest_layout::{BN254_G1_AFFINE, BN254_G2_AFFINE, BN254_PAIR_G1, BN254_PAIR_G2};
    let pairs: Vec<(G1Affine, G2Affine)> = match operand.querier {
        HintTarget::Native => operand.native_values(),
        HintTarget::Guest => {
            let size = guest_layout::BN254_PAIR_SIZE;
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
    // the Miller loop as the verifier computes it (the affine lines from hinted slopes, at
    // normalized `G1` points): the witness equation holds for the verifier's output only
    let prepared: Vec<G2PreparedNoAlloc> = pairs
        .iter()
        .map(|(_, g2)| crate::bn254::curves::g2_affine::prepare_as_verifier(g2))
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
        let witness = crate::residue_witness::bn254::witness(&f)
            .expect("the final exponentiation found an identity, which has a witness");
        responses.write(&(true, witness));
    } else {
        responses.write(&(false, f.inverse().expect("non-zero")));
    }
}

/// The claim about `e(p1, G2) e(p2, tau G2)` for the affine points `[p1, p2]` of the operand, as
/// `bn254_pairing_residue_witness`: the identity flag, then `d` and the scaling factor
/// (`crate::residue_witness::bls12_381`) for an identity, or the inverse of the Miller loop
/// output otherwise
fn bls12_381_kzg_residue_witness(
    operand: &Operand,
    claim_not_identity: bool,
    responses: &mut Responses,
) {
    use crate::bls12_381::curves::Bls12_381;
    use crate::bls12_381::G1Affine;
    use ark_ec::pairing::{MillerLoopOutput, Pairing};
    use guest_layout::BLS12_381_G1_AFFINE;
    let [p1, p2]: [G1Affine; 2] = match operand.querier {
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
        &crate::bls12_381::consts::PREPARED_G2_GENERATOR,
        &crate::bls12_381::consts::PREPARED_G2_BY_TAU,
    ];
    let points = [p1, p2];
    // the Miller loop as the verifier computes it: over both pairs at once, with the normalized
    // lines of the fixed points (the witness equation holds for the verifier's output only)
    let pairs = points.iter().zip(g2);
    let f = Bls12_381::multi_miller_loop_shared(None, pairs.clone());
    let is_identity = !claim_not_identity
        && Bls12_381::final_exponentiation(MillerLoopOutput(f))
            .expect("non-zero")
            .0
            .is_one();
    if is_identity {
        let witness = crate::residue_witness::bls12_381::witness(&f)
            .expect("the final exponentiation found an identity, which has a witness");
        responses.write(&(true, witness));
    } else {
        // the inverse of the output of the plain (conjugated) loop, which the exact path runs
        let mut conjugated = f;
        conjugated.conjugate_in_place();
        responses.write(&(false, conjugated.inverse().expect("non-zero")));
    }
}

/// The slopes of the two affine `G2` chains of the pairing input point of the operand, see
/// `hints::bn254_g2_pairing_slopes`: a flag and the subgroup test slopes, a flag and the
/// line precomputation slopes (zeros behind a cleared flag)
fn bn254_g2_pairing_slopes(operand: &Operand, responses: &mut Responses) {
    use crate::bn254::curves::g2_affine::{line_slopes, subgroup_slopes};
    use crate::bn254::{Fq2, G2Affine};
    use ark_ff::AdditiveGroup;
    use guest_layout::BN254_G2_AFFINE;
    let q: G2Affine = match operand.querier {
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
    chain(subgroup_slopes(&q), responses);
    chain(line_slopes(&q), responses);
}

/// Arbitrary pairing claim, using exactly the projective prepared lines of the verifier.
fn bls12_381_pairing_residue_witness(
    operand: &Operand,
    claim_not_identity: bool,
    responses: &mut Responses,
) {
    use crate::bls12_381::curves::Bls12_381;
    use crate::bls12_381::Fq12;
    use crate::hints::Bls12381Pair;
    use ark_ec::pairing::{MillerLoopOutput, Pairing};
    use guest_layout::*;
    let pairs: Vec<Bls12381Pair> = match operand.querier {
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
        let witness = crate::residue_witness::bls12_381::witness(&f)
            .expect("an identity has a residue witness");
        responses.write(&(true, witness));
    } else {
        responses.write(&(
            false,
            conjugated.inverse().expect("the Miller loop is non-zero"),
        ));
    }
}

#[cfg(test)]
mod native_query_tests {
    use super::*;
    use crate::hints::guest_layout;
    use crate::hints::host::{answer_query, NativeQuerierMemory};
    use crate::hints::test_support::{GuestMemory, TestMemorySource};
    use crate::hints::{FieldOpsHint64, Secp256k1Element, FIELD_OPS_ADVISE_QUERY_ID};
    use crate::secp256k1::field::FieldElement;
    use crate::secp256k1::scalars::Scalar;
    use ark_ec::AffineRepr;
    use ark_ff::{AdditiveGroup, BigInteger, Field, PrimeField};

    /// The run an oracle serves, as `oracle_provider::RunMode`: `answer_query` produces the
    /// responses of the runs the mode produces
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RunMode {
        NativeRunOnly,
        NativeRunSavingForRiscV,
        RiscVRun,
    }

    impl RunMode {
        const fn produces_native_run_responses(self) -> bool {
            match self {
                Self::NativeRunOnly | Self::NativeRunSavingForRiscV => true,
                Self::RiscVRun => false,
            }
        }

        const fn produces_guest_run_responses(self) -> bool {
            match self {
                Self::NativeRunOnly => false,
                Self::NativeRunSavingForRiscV | Self::RiscVRun => true,
            }
        }
    }

    /// The responses of the runs of `mode` to a hint query of the querier of `memory`
    fn process_memory_query(
        input_word: usize,
        memory: &dyn QuerierMemory,
        mode: RunMode,
    ) -> (Vec<u32>, Vec<u32>) {
        let (mut native_run, mut guest_run) = (Vec::new(), Vec::new());
        // SAFETY: for a querier in this process, the tests pass the memory of this process and a
        // live request made by `request`, of a live operand of the type of its op. The tests of
        // the null request and operand pointers are outside this contract on purpose: those
        // pointers are rejected before they are read
        let answered = unsafe {
            answer_query(
                FIELD_OPS_ADVISE_QUERY_ID,
                input_word,
                memory,
                mode.produces_native_run_responses()
                    .then_some(&mut native_run),
                mode.produces_guest_run_responses()
                    .then_some(&mut guest_run),
            )
        }
        .expect("must answer the field hint query");
        assert!(answered);
        (native_run, guest_run)
    }

    /// The request for the hint `op` on `operand`, of a querier in this process
    fn request<T: ?Sized>(op: FieldHintOp, operand: &T) -> FieldOpsHint64 {
        FieldOpsHint64 {
            op: op as u32,
            src_ptr: core::ptr::from_ref(operand)
                .cast::<u8>()
                .expose_provenance() as u64,
            src_len_u32_words: (size_of_val(operand) / 4) as u32,
        }
    }

    /// The responses of the native run and of the guest run of `mode` to the hint request `hint` of a
    /// querier in this process
    fn process(mode: RunMode, hint: &FieldOpsHint64) -> (Vec<u32>, Vec<u32>) {
        // SAFETY: the request and the operand it points to are alive during the call
        let memory = unsafe { NativeQuerierMemory::new() };
        process_memory_query(core::ptr::from_ref(hint).expose_provenance(), &memory, mode)
    }

    /// The response of a run on the RISC-V guest to its request for the hint `op` on the words
    /// `operand` (at 0x1000 of its memory)
    fn process_guest(op: FieldHintOp, operand: &[u32]) -> Vec<u32> {
        // guest memory: the request (a `FieldOpsHint`) at 0x100, the operand at 0x1000
        let mut memory = TestMemorySource::default();
        memory.insert_u32(0x100, op as u32);
        memory.insert_u32(0x104, 0x1000);
        memory.insert_u32(0x108, operand.len() as u32);
        for (i, word) in operand.iter().enumerate() {
            memory.insert_u32(0x1000 + 4 * i as u32, *word);
        }
        let (native_run, guest_run) =
            process_memory_query(0x100, &GuestMemory(&memory), RunMode::RiscVRun);
        assert!(native_run.is_empty());
        guest_run
    }

    /// An element and its inverse
    fn element_and_inverse() -> (FieldElement, FieldElement) {
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        let mut inverse = x;
        inverse.invert_in_place();
        (x, inverse)
    }

    #[test]
    fn a_native_querier_is_answered_for_the_runs_of_the_mode() {
        let (x, inverse) = element_and_inverse();
        let hint = request(FieldHintOp::Secp256k1BaseFieldInverse, &x);

        let (native_run, guest_run) = process(RunMode::NativeRunOnly, &hint);
        assert_eq!(native_run, inverse.to_canonical_words());
        assert!(guest_run.is_empty());

        // a native run that records the prover input also answers the guest
        let (native_run, guest_run) = process(RunMode::NativeRunSavingForRiscV, &hint);
        assert_eq!(native_run, inverse.to_canonical_words());
        assert_eq!(guest_run, inverse.to_montgomery_words());
    }

    #[test]
    fn the_guest_is_answered_in_a_risc_v_run() {
        let (x, inverse) = element_and_inverse();
        let guest_run = process_guest(
            FieldHintOp::Secp256k1BaseFieldInverse,
            &x.to_montgomery_words(),
        );
        assert_eq!(guest_run, inverse.to_montgomery_words());
    }

    /// The limbs of `x` in the memory of the RISC-V guest: its Montgomery form with
    /// `R = 2^(64 limbs)`, plus `extra` moduli (another representative)
    fn guest_limbs<F: PrimeField>(x: F, limbs: usize, extra: u64) -> Vec<u32> {
        let r = F::from(2u64).pow([64 * limbs as u64]);
        let mut value = (x * r).into_bigint();
        for _ in 0..extra {
            value.add_with_carry(&F::MODULUS);
        }
        let mut words: Vec<u32> = value
            .as_ref()
            .iter()
            .flat_map(|limb| [*limb as u32, (*limb >> 32) as u32])
            .collect();
        words.resize(2 * limbs, 0);
        words
    }

    /// Both answers to the guest agree: from the operand of a native run, and from the same
    /// operand in the memory of the guest
    fn assert_same_guest_answers<T: ?Sized>(op: FieldHintOp, native: &T, guest: &[u32]) {
        let (_, from_native) = process(RunMode::NativeRunSavingForRiscV, &request(op, native));
        assert_eq!(process_guest(op, guest), from_native);
    }

    #[test]
    fn guest_elements_are_read_in_their_montgomery_form() {
        let x = crate::bn254::Fq::from(0x1234_5678u64).square();
        for extra in [0, 1] {
            let guest = guest_limbs(x, 4, extra);
            assert_same_guest_answers(FieldHintOp::Bn254BaseFieldInverse, &x, &guest);
        }
        let y = crate::bls12_381::Fq::from(0x1234_5678u64).square();
        for extra in [0, 1] {
            let guest = guest_limbs(y, 8, extra);
            assert_same_guest_answers(FieldHintOp::Bls12381BaseFieldInverse, &y, &guest);
            assert_same_guest_answers(FieldHintOp::Bls12381BaseFieldSqrt, &y, &guest);
        }
        let f = crate::bn254::Fq12::new(
            crate::bn254::Fq6::new(
                crate::bn254::Fq2::new(x, x.double()),
                crate::bn254::Fq2::new(x.square(), x + x.square()),
                crate::bn254::Fq2::new(x.inverse().unwrap(), x),
            ),
            crate::bn254::Fq6::new(
                crate::bn254::Fq2::new(x.double().square(), x),
                crate::bn254::Fq2::new(x, x.double()),
                crate::bn254::Fq2::new(x.square().square(), x),
            ),
        );
        // the components in order, `c0` first, recursively
        let mut guest = Vec::new();
        for fq6 in [&f.c0, &f.c1] {
            for fq2 in [&fq6.c0, &fq6.c1, &fq6.c2] {
                guest.extend(guest_limbs(fq2.c0, 4, 0));
                guest.extend(guest_limbs(fq2.c1, 4, 1));
            }
        }
        assert_same_guest_answers(FieldHintOp::Bn254Fq12Inverse, &f, &guest);
    }

    /// The words of an affine point in the memory of the RISC-V guest
    fn guest_point(
        layout: guest_layout::AffinePoint,
        x: Vec<u32>,
        y: Vec<u32>,
        infinity: bool,
    ) -> Vec<u32> {
        let mut words = vec![0u32; layout.size / 4];
        words[layout.x / 4..][..x.len()].copy_from_slice(&x);
        words[layout.y / 4..][..y.len()].copy_from_slice(&y);
        words[layout.infinity / 4] = u32::from(infinity) << (8 * (layout.infinity % 4));
        words
    }

    fn guest_fq2(x: &crate::bn254::Fq2) -> Vec<u32> {
        [guest_limbs(x.c0, 4, 0), guest_limbs(x.c1, 4, 0)].concat()
    }

    #[test]
    fn guest_points_are_read_where_the_layout_puts_them() {
        use crate::{bls12_381, bn254};
        use guest_layout::*;

        let g2 = bn254::G2Affine::generator();
        let guest_g2 = guest_point(BN254_G2_AFFINE, guest_fq2(&g2.x), guest_fq2(&g2.y), false);
        assert_same_guest_answers(FieldHintOp::Bn254G2PairingSlopes, &g2, &guest_g2);

        let g1 = (bn254::G1Affine::generator() * bn254::Fr::from(5u64)).into();
        let g1: bn254::G1Affine = g1;
        let guest_g1 = guest_point(
            BN254_G1_AFFINE,
            guest_limbs(g1.x, 4, 0),
            guest_limbs(g1.y, 4, 1),
            false,
        );
        let mut guest_pair = vec![0u32; BN254_PAIR_SIZE / 4];
        guest_pair[BN254_PAIR_G1 / 4..][..guest_g1.len()].copy_from_slice(&guest_g1);
        guest_pair[BN254_PAIR_G2 / 4..][..guest_g2.len()].copy_from_slice(&guest_g2);
        let pairs = [(g1, g2), (g1, g2)];
        assert_same_guest_answers(
            FieldHintOp::Bn254PairingResidueWitness,
            &pairs[..],
            &[guest_pair.clone(), guest_pair].concat(),
        );

        let p = bls12_381::G1Affine::generator();
        let guest_p = guest_point(
            BLS12_381_G1_AFFINE,
            guest_limbs(p.x, 8, 0),
            guest_limbs(p.y, 8, 0),
            false,
        );
        // the point at infinity is the flag
        let infinity = bls12_381::G1Affine::identity();
        let guest_infinity = guest_point(BLS12_381_G1_AFFINE, vec![], vec![], true);
        assert_same_guest_answers(
            FieldHintOp::Bls12381KzgResidueWitness,
            &[p, infinity],
            &[guest_p, guest_infinity].concat(),
        );
    }

    /// The integer of little-endian words plus `modulus`, which must stay below `2^256`
    fn plus(words: [u32; 8], modulus: [u32; 8]) -> [u32; 8] {
        let mut sum = [0u32; 8];
        let mut carry = 0u64;
        for ((out, word), modulus) in sum.iter_mut().zip(words).zip(modulus) {
            let t = u64::from(word) + u64::from(modulus) + carry;
            *out = t as u32;
            carry = t >> 32;
        }
        assert_eq!(carry, 0, "below 2^256");
        sum
    }

    /// The big-endian bytes of `n`
    fn be_bytes_of(n: u8) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[31] = n;
        bytes
    }

    // Not in zksync-os, whose tests answer the guest for the base field inverse only: every
    // secp256k1 op answers the guest alike from either querier, also for an operand of the guest
    // above the modulus (another 256-bit representative of its Montgomery form).
    #[test]
    fn secp256k1_guest_operands_are_read_as_montgomery_words() {
        // elements of small Montgomery words, which have another representative below `2^256`:
        // the words plus the modulus, as they are below `2^256 - p = 2^32 + 977` (`2^256 - n`)
        let small = |word: u32| [word, 0, 0, 0, 0, 0, 0, 0];
        let (xm, ym, sm) = (small(3), small(5), small(7));
        let x = FieldElement::from_montgomery_words(&xm);
        let y = FieldElement::from_montgomery_words(&ym);
        let s = Scalar::from_montgomery_words(&sm);
        assert_eq!((x.to_montgomery_words(), s.to_montgomery_words()), (xm, sm));
        let p = <FieldElement as Secp256k1Element>::MODULUS;
        let n = <Scalar as Secp256k1Element>::MODULUS;
        for (xw, yw, sw) in [(xm, ym, sm), (plus(xm, p), plus(ym, p), plus(sm, n))] {
            assert_same_guest_answers(FieldHintOp::Secp256k1BaseFieldSqrt, &x, &xw);
            assert_same_guest_answers(FieldHintOp::Secp256k1BaseFieldInverse, &x, &xw);
            assert_same_guest_answers(FieldHintOp::Secp256k1ScalarFieldInverse, &s, &sw);
            assert_same_guest_answers(
                FieldHintOp::Secp256k1BaseFieldDivision,
                &[x, y],
                &[xw, yw].concat(),
            );
        }
        // both answers of the sqrt: 4 is a square, -4 is not (-1 is not, as `p = 3 mod 4`), and
        // its candidate is the same root of 4 (`(p + 1) / 4` is even)
        let four = FieldElement::from_bytes(&be_bytes_of(4)).unwrap();
        let mut minus_four = four;
        minus_four.negate_in_place(1);
        minus_four.normalize_in_place();
        let mut root = four;
        assert!(root.sqrt_in_place());
        for (element, flag) in [(four, 0), (minus_four, 1)] {
            let words = element.to_montgomery_words();
            assert_same_guest_answers(FieldHintOp::Secp256k1BaseFieldSqrt, &element, &words);
            assert_eq!(
                process_guest(FieldHintOp::Secp256k1BaseFieldSqrt, &words),
                [&root.to_montgomery_words()[..], &[flag]].concat()
            );
        }
        // and the guest answers are the Montgomery words of the results
        let mut quotient = y;
        quotient.invert_in_place();
        quotient.mul_in_place(&x);
        assert_eq!(
            process_guest(FieldHintOp::Secp256k1BaseFieldDivision, &[xm, ym].concat()),
            quotient.to_montgomery_words()
        );
        let inverse = s.invert_vartime();
        assert_eq!(
            process_guest(FieldHintOp::Secp256k1ScalarFieldInverse, &sm),
            inverse.to_montgomery_words()
        );
    }

    #[test]
    #[should_panic(expected = "not the querier of the run mode")]
    fn a_querier_in_this_process_is_not_answered_in_a_risc_v_run() {
        let (x, _) = element_and_inverse();
        let _ = process(
            RunMode::RiscVRun,
            &request(FieldHintOp::Secp256k1BaseFieldInverse, &x),
        );
    }

    #[test]
    #[should_panic]
    fn native_field_ops_query_rejects_null_query_pointer() {
        // SAFETY: nothing is read at a null address
        let memory = unsafe { NativeQuerierMemory::new() };
        // SAFETY: outside the contract on purpose: the null request is rejected before it is read
        let _ = unsafe {
            answer_query(
                FIELD_OPS_ADVISE_QUERY_ID,
                0,
                &memory,
                Some(&mut Vec::new()),
                None,
            )
        };
    }

    #[test]
    #[should_panic]
    fn native_field_ops_query_rejects_null_operand_pointer() {
        let hint = FieldOpsHint64 {
            op: FieldHintOp::Secp256k1BaseFieldInverse as u32,
            src_ptr: 0,
            src_len_u32_words: 8,
        };
        let _ = process(RunMode::NativeRunOnly, &hint);
    }
}
