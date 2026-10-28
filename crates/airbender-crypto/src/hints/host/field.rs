//! The answers to the field hints. Operands and answers are in the representation of the
//! querier's target (see `FieldHintOp`, `HintAnswer` and `Secp256k1Element`); the responses asked
//! for tell which target the querier is (see `answer_query`).

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

/// The op and the operand of the hint request at `input_word`
///
/// # Safety
///
/// For a native querier, as for `Operand::new`: `input_word` is a live request in this process.
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
    // SAFETY: guaranteed by the caller
    (op, unsafe {
        Operand::new(memory, querier, address, len_u32_words)
    })
}

/// The responses asked for: the answer as a native querier reads it, and as the guest does
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

/// A native answer to the pairing request at `input_word` that claims a non-identity whatever the
/// product, with the inverse the exact path needs: for tests of that path.
///
/// # Panics
///
/// If the request is not a pairing op (`Bn254PairingResidueWitness`, `Bls12381KzgResidueWitness`
/// or `Bls12381PairingResidueWitness`), or is malformed.
///
/// # Safety
///
/// As for [`answer_query`](super::answer_query) with a querier in this process.
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

/// Serves a field hint query (see `answer_query`)
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
    // the querier is in this process exactly when the native response is asked for
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
    // SAFETY: a native querier's request is live (the caller's obligation)
    let (op, operand) = unsafe { read_request(memory, input_word, querier) };
    answer(op, &operand, &mut responses);
}

/// The operand of a hint query, in the querier's memory and representation
pub(crate) struct Operand<'m> {
    memory: &'m dyn QuerierMemory,
    querier: HintTarget,
    address: usize,
    len_u32_words: u32,
}

impl<'m> Operand<'m> {
    /// # Safety
    ///
    /// For a native querier, `address` must point to `len_u32_words` words of live values of the
    /// op's operand types, alive and unchanged while the operand is used: they are read in place.
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
        // SAFETY: the operand is live values of `T` (see `Operand::new`); size and alignment are
        // checked above
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

/// The base field element of its guest Montgomery limbs (`R = 2^(32 words)`), any representative
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

/// `(candidate, is_quadratic_non_residue)`: `candidate² == input` for a square, `-input` otherwise
pub(crate) fn secp256k1_base_field_sqrt(operand: &Operand) -> (FieldElement, bool) {
    let el: FieldElement = secp256k1_element(operand);
    assert!(!el.is_zero());
    let mut candidate = el;
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

/// As `secp256k1_base_field_sqrt` (`p = 3 mod 4`, so exactly one of `±input` is a square)
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

/// The claim about the bn254 pairing product (see `hints::PairingClaim`). `claim_not_identity`
/// forces a non-identity claim (for tests of the exact path).
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
    // the witness equation holds for the verifier's Miller loop output only
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

/// The claim about `e(p1, G2) e(p2, tau G2)`, as `bn254_pairing_residue_witness`
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
    // the witness equation holds for the verifier's Miller loop output only
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
        // the inverse of the conjugated loop output, which the exact path computes
        let mut conjugated = f;
        conjugated.conjugate_in_place();
        responses.write(&(false, conjugated.inverse().expect("non-zero")));
    }
}

/// The slopes of the two affine `G2` chains of the operand (see `hints::G2SlopeHints`); an
/// exceptional chain is a cleared flag and zeros
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

/// The claim about the bls12-381 pairing product, from the verifier's projective lines
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
    use crate::hints::host::{answer_query, NativeQuerierMemory};
    use crate::hints::test_support::{GuestMemory, TestMemorySource};
    use crate::hints::FieldOpsHint64;
    use ark_ec::AffineRepr;
    use ark_ff::BigInteger;

    /// The native and guest responses of `answer_query` to the field hint request at `input_word`
    fn respond(
        input_word: usize,
        memory: &dyn QuerierMemory,
        native: bool,
        guest: bool,
    ) -> (Vec<u32>, Vec<u32>) {
        let (mut native_run, mut guest_run) = (Vec::new(), Vec::new());
        // SAFETY: a native request is made by `request`, of a live operand, and is alive during
        // the call; the null pointer tests are outside this contract on purpose, as those pointers
        // are rejected before they are read
        let answered = unsafe {
            answer_query(
                FIELD_OPS_ADVISE_QUERY_ID,
                input_word,
                memory,
                native.then_some(&mut native_run),
                guest.then_some(&mut guest_run),
            )
        };
        assert_eq!(answered, Ok(true));
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

    /// The responses to the native request `hint`
    fn process(hint: &FieldOpsHint64, native: bool, guest: bool) -> (Vec<u32>, Vec<u32>) {
        // SAFETY: the request and its operand are alive during the call
        let memory = unsafe { NativeQuerierMemory::new() };
        respond(
            core::ptr::from_ref(hint).expose_provenance(),
            &memory,
            native,
            guest,
        )
    }

    /// The response to a guest request for `op` on the words `operand`: the request (a
    /// `FieldOpsHint`) at 0x100 of its memory, the operand at 0x1000
    fn process_guest(op: FieldHintOp, operand: &[u32]) -> Vec<u32> {
        let mut memory = TestMemorySource::default();
        let request = [op as u32, 0x1000, operand.len() as u32];
        for (address, words) in [(0x100, &request[..]), (0x1000, operand)] {
            for (i, word) in words.iter().enumerate() {
                memory.insert_u32(address + 4 * i as u32, *word);
            }
        }
        respond(0x100, &GuestMemory(&memory), false, true).1
    }

    #[test]
    fn a_native_querier_is_answered_for_the_runs_of_the_mode() {
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        let mut inverse = x;
        inverse.invert_in_place();
        let hint = request(FieldHintOp::Secp256k1BaseFieldInverse, &x);
        let native = inverse.to_canonical_words().to_vec();
        assert_eq!(process(&hint, true, false), (native.clone(), vec![]));
        // a recording native run also answers the guest
        let guest = inverse.to_montgomery_words().to_vec();
        assert_eq!(process(&hint, true, true), (native, guest));
    }

    /// The limbs of `x` in guest memory: its Montgomery form with `R = 2^(64 limbs)`, plus `extra`
    /// moduli (another representative)
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

    /// The guest's answer to the operand in its memory is the one a recording native run gives
    fn assert_same_guest_answers<T: ?Sized>(op: FieldHintOp, native: &T, guest: &[u32]) {
        let (_, from_native) = process(&request(op, native), true, true);
        assert_eq!(process_guest(op, guest), from_native);
    }

    #[test]
    fn guest_elements_are_read_in_their_montgomery_form() {
        let x = crate::bn254::Fq::from(0x1234_5678u64).square();
        let y = crate::bls12_381::Fq::from(0x1234_5678u64).square();
        for extra in [0, 1] {
            let guest = guest_limbs(x, 4, extra);
            assert_same_guest_answers(FieldHintOp::Bn254BaseFieldInverse, &x, &guest);
            let guest = guest_limbs(y, 8, extra);
            assert_same_guest_answers(FieldHintOp::Bls12381BaseFieldInverse, &y, &guest);
            assert_same_guest_answers(FieldHintOp::Bls12381BaseFieldSqrt, &y, &guest);
        }
        let f = crate::bn254::Fq12::from_base_prime_field_elems(
            (1..=12u64).map(|i| x * crate::bn254::Fq::from(i)),
        )
        .unwrap();
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

    /// The words of an affine point in guest memory
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

        let g1: bn254::G1Affine = (bn254::G1Affine::generator() * bn254::Fr::from(5u64)).into();
        let guest_g1 = guest_point(
            BN254_G1_AFFINE,
            guest_limbs(g1.x, 4, 0),
            guest_limbs(g1.y, 4, 1),
            false,
        );
        let mut guest_pair = vec![0u32; BN254_PAIR_SIZE / 4];
        guest_pair[BN254_PAIR_G1 / 4..][..guest_g1.len()].copy_from_slice(&guest_g1);
        guest_pair[BN254_PAIR_G2 / 4..][..guest_g2.len()].copy_from_slice(&guest_g2);
        assert_same_guest_answers(
            FieldHintOp::Bn254PairingResidueWitness,
            &[(g1, g2), (g1, g2)][..],
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
        let guest_infinity = guest_point(BLS12_381_G1_AFFINE, vec![], vec![], true);
        assert_same_guest_answers(
            FieldHintOp::Bls12381KzgResidueWitness,
            &[p, bls12_381::G1Affine::identity()],
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

    /// Every secp256k1 op reads a guest operand as Montgomery words, also above the modulus
    #[test]
    fn secp256k1_guest_operands_are_read_as_montgomery_words() {
        // small Montgomery words, whose other representative (plus the modulus) is below `2^256`
        let small = |word: u32| [word, 0, 0, 0, 0, 0, 0, 0];
        let (xm, ym, sm) = (small(3), small(5), small(7));
        let x = FieldElement::from_montgomery_words(&xm);
        let y = FieldElement::from_montgomery_words(&ym);
        let s = Scalar::from_montgomery_words(&sm);
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
    }

    #[test]
    #[should_panic(expected = "not the querier of the run mode")]
    fn a_querier_in_this_process_is_not_answered_in_a_risc_v_run() {
        let x = FieldElement::ONE;
        let _ = process(
            &request(FieldHintOp::Secp256k1BaseFieldInverse, &x),
            false,
            true,
        );
    }

    #[test]
    #[should_panic(expected = "must read the field hint request")]
    fn native_field_ops_query_rejects_null_query_pointer() {
        // SAFETY: nothing is read at a null address
        let memory = unsafe { NativeQuerierMemory::new() };
        let _ = respond(0, &memory, true, false);
    }

    #[test]
    #[should_panic(expected = "the operand is not at the null address")]
    fn native_field_ops_query_rejects_null_operand_pointer() {
        let hint = FieldOpsHint64 {
            op: FieldHintOp::Secp256k1BaseFieldInverse as u32,
            src_ptr: 0,
            src_len_u32_words: 8,
        };
        let _ = process(&hint, true, false);
    }
}
