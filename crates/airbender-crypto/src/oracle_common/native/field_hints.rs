//! The dispatch of the field hint queries to the oracle implementations of the primitives.
//! Operands and answers are in the representation of the querier's target (see `FieldHintOp`,
//! `HintAnswer` and `Secp256k1Element`); the responses asked for tell which target the querier is
//! (see `answer_query`).

use super::{Operand, Responses};
use crate::bls12_381::oracle_implementations as bls12_381;
use crate::bn254::oracle_implementations as bn254;
use crate::hints::{
    read_field_hint_request, FieldHintOp, HintEncoding, HintError, HintTarget,
    FIELD_OPS_ADVISE_QUERY_ID,
};
use crate::secp256k1::oracle_implementations as secp256k1;
use airbender_oracle::{offset_address, QuerierMemory};
use alloc::vec::Vec;
use ark_ff::PrimeField;

/// Reads `num_words` consecutive `u32` words at `address` of the querier.
pub fn read_querier_u32_words<M: QuerierMemory + ?Sized>(
    memory: &M,
    address: usize,
    num_words: usize,
) -> Result<Vec<u32>, HintError> {
    (0..num_words)
        .map(|i| {
            let offset = i
                .checked_mul(size_of::<u32>())
                .ok_or(HintError("querier address overflows"))?;
            Ok(memory.read_u32(offset_address(address, offset)?)?)
        })
        .collect()
}

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

/// The answer to the hint `op` on `operand`
fn answer(op: FieldHintOp, operand: &Operand, responses: &mut Responses) {
    match op {
        FieldHintOp::Secp256k1BaseFieldSqrt => {
            responses.write(&secp256k1::base_field_sqrt(operand))
        }
        FieldHintOp::Secp256k1BaseFieldInverse => {
            responses.write(&secp256k1::base_field_inverse(operand))
        }
        FieldHintOp::Secp256k1ScalarFieldInverse => {
            responses.write(&secp256k1::scalar_field_inverse(operand))
        }
        FieldHintOp::Secp256k1BaseFieldDivision => {
            responses.write(&secp256k1::base_field_division(operand))
        }
        FieldHintOp::Secp256k1RecoveryWitness => match secp256k1::recovery_witness(operand) {
            Some(witness) => responses.write(&(true, witness)),
            None => responses.write(&false),
        },
        FieldHintOp::Bn254BaseFieldInverse => {
            responses.write(&inverse::<crate::bn254::Fq>(operand))
        }
        FieldHintOp::Bn254BaseFieldDivision => {
            responses.write(&division::<crate::bn254::Fq>(operand))
        }
        FieldHintOp::Bn254Fq12Inverse => responses.write(&inverse::<crate::bn254::Fq12>(operand)),
        FieldHintOp::Bls12381BaseFieldSqrt => responses.write(&bls12_381::base_field_sqrt(operand)),
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
            bn254::pairing_residue_witness(operand, false, responses)
        }
        FieldHintOp::Bn254G2PairingSlopes => bn254::g2_pairing_slopes(operand, responses),
        FieldHintOp::Bls12381KzgResidueWitness => {
            bls12_381::kzg_residue_witness(operand, false, responses)
        }
        FieldHintOp::Bls12381PairingResidueWitness => {
            bls12_381::pairing_residue_witness(operand, false, responses)
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
            bn254::pairing_residue_witness(&operand, true, &mut responses)
        }
        (FieldHintOp::Bls12381KzgResidueWitness, operand) => {
            bls12_381::kzg_residue_witness(&operand, true, &mut responses)
        }
        (FieldHintOp::Bls12381PairingResidueWitness, operand) => {
            bls12_381::pairing_residue_witness(&operand, true, &mut responses)
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

/// The inverse of a non-zero field element
pub(crate) fn inverse<F: HintEncoding + Copy>(operand: &Operand) -> F {
    let el: F = operand.element();
    crate::field_inverse::inverse(&el).expect("the operand is non-zero")
}

/// The quotient of the numerator and the (non-zero) denominator, two adjacent elements of a
/// base prime field
pub(crate) fn division<F: HintEncoding + PrimeField>(operand: &Operand) -> F {
    let [numerator, denominator]: [F; 2] = match operand.querier() {
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

#[cfg(test)]
mod native_query_tests {
    use super::*;
    use crate::hints::test_support::{GuestMemory, TestMemorySource};
    use crate::hints::{
        guest_layout, FieldOpsHint64, HintAnswer, RecoveryRequest, Secp256k1Element,
    };
    use crate::oracle_common::{answer_query, NativeQuerierMemory};
    use crate::secp256k1::field::FieldElement;
    use crate::secp256k1::scalars::Scalar;
    use ark_ec::AffineRepr;
    use ark_ff::{BigInteger, Field};

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

    /// The request of the witness of the recovery of the signature by `key` of the digest
    /// `digest` (not reduced), with its `r` below the base field modulus
    fn recovery_request(key: [u8; 32], digest: [u8; 32]) -> RecoveryRequest {
        use crate::secp256k1::hooks::DefaultSecp256k1Hooks;
        use crate::secp256k1::Affine;

        let key = crate::k256::ecdsa::SigningKey::from_bytes((&key).into()).unwrap();
        let (signature, recovery_id) = key.sign_prehash_recoverable(&digest).unwrap();
        assert!(!recovery_id.is_x_reduced());
        let bytes: [u8; 64] = signature.to_bytes().into();
        let r = <Scalar as Secp256k1Element>::from_be_bytes(bytes[..32].try_into().unwrap());
        let s = <Scalar as Secp256k1Element>::from_be_bytes(bytes[32..].try_into().unwrap());
        let x = FieldElement::from_bytes(bytes[..32].try_into().unwrap()).unwrap();
        let point =
            Affine::lift_x_with_hooks(&x, recovery_id.is_y_odd(), &mut DefaultSecp256k1Hooks)
                .unwrap();
        RecoveryRequest::new(
            &point,
            &r.unwrap(),
            &s.unwrap(),
            &Scalar::from_be_bytes_reduced(&digest),
        )
    }

    #[test]
    fn the_recovery_witness_is_the_same_words_for_every_target() {
        let operand = recovery_request([5; 32], [9; 32]);
        // the guest's request: the five elements as their Montgomery words, back to back
        let mut guest = Vec::new();
        for words in [
            operand.r.to_montgomery_words(),
            operand.s.to_montgomery_words(),
            operand.message.to_montgomery_words(),
            operand.x.to_montgomery_words(),
            operand.y.to_montgomery_words(),
        ] {
            guest.extend(words);
        }
        assert_same_guest_answers(FieldHintOp::Secp256k1RecoveryWitness, &operand, &guest);

        let (native, guest) = process(
            &request(FieldHintOp::Secp256k1RecoveryWitness, &operand),
            true,
            true,
        );
        let witness = crate::secp256k1::oracle_implementations::witness::recovery_witness(
            &operand.point(),
            &operand.r,
            &operand.s,
            &operand.message,
        )
        .unwrap();
        let mut expected = Vec::new();
        (true, witness).write(HintTarget::Guest, &mut expected);
        assert_eq!(expected.len(), 1 + 8 + 8 + 16);
        assert_eq!(native, expected);
        assert_eq!(guest, expected);
    }
}
