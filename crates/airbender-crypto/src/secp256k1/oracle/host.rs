//! The host side of the hints (see the [module documentation](super)): serves a hint query from
//! the memory of the querier, in the representation of the querier.
//!
//! A host that runs a guest on the RISC-V simulator reads the request and the operand from the
//! guest's RAM ([`QuerierMemory`] over it, word size 4) and answers in the representation of the
//! guest's build: [`Representation::Montgomery`] for the delegated field, [`Representation::Canonical`]
//! otherwise. A test in the host's own process reads them through exposed pointers (word size
//! `size_of::<usize>()`), in the canonical representation.

extern crate alloc;

use super::{
    be_bytes_of_words, is_below, words_of_be_bytes, FieldHintOp, FieldOpsHint, N_WORDS, P_WORDS,
    RECOVERY_REQUEST_WORDS, R_MOD_N_WORDS, R_MOD_P_WORDS,
};
use crate::secp256k1::field::FieldElement;
use crate::secp256k1::points::Affine;
use crate::secp256k1::scalars::Scalar;
use crate::secp256k1::witness_host::recovery_witness;
use alloc::vec::Vec;
use core::mem::offset_of;

/// Read access of the host to the memory of the querier
pub trait QuerierMemory {
    /// The size in bytes of a word, and of an address, of the querier: 4 for the RISC-V guest,
    /// `size_of::<usize>()` for a querier in this process
    fn word_size(&self) -> usize;

    /// The `u32` at `address`, which is 4-byte aligned
    fn read_u32(&self, address: usize) -> u32;
}

/// How the elements cross the channel (see `MONTGOMERY_WORDS`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Representation {
    /// The canonical integers, little-endian words
    Canonical,
    /// The Montgomery words of the delegated field of the RISC-V guest: an operand is any
    /// 256-bit representative, an answer is below the modulus
    Montgomery,
}

/// `2^-256 mod p` and `2^-256 mod n`, big-endian
const R_INVERSE_MOD_P: [u8; 32] = [
    0xc9, 0xbd, 0x19, 0x05, 0x15, 0x53, 0x83, 0x99, 0x9c, 0x46, 0xc2, 0xc2, 0x95, 0xf2, 0xb7, 0x61,
    0xbc, 0xb2, 0x23, 0xfe, 0xdc, 0x24, 0xa0, 0x59, 0xd8, 0x38, 0x09, 0x1d, 0x08, 0x68, 0x19, 0x2a,
];
const R_INVERSE_MOD_N: [u8; 32] = [
    0xd9, 0xe8, 0x89, 0x0d, 0x64, 0x94, 0xef, 0x93, 0x89, 0x7f, 0x30, 0xc1, 0x27, 0xcf, 0xab, 0x5d,
    0x3b, 0xbb, 0xd4, 0x56, 0x7f, 0xa5, 0x0c, 0x3c, 0x80, 0xfd, 0x22, 0x93, 0x80, 0x97, 0xc0, 0x16,
];

/// The canonical words of a 256-bit integer below twice the modulus (any 256-bit integer, for
/// these moduli above `2^255`)
fn reduce_below(words: &[u32; 8], modulus: &[u32; 8]) -> [u32; 8] {
    if is_below(words, modulus) {
        return *words;
    }
    let mut difference = [0u32; 8];
    let mut borrow = 0u64;
    for ((out, word), modulus) in difference.iter_mut().zip(words).zip(modulus) {
        let t = u64::from(*word)
            .wrapping_sub(u64::from(*modulus))
            .wrapping_sub(borrow);
        *out = t as u32;
        borrow = (t >> 63) & 1;
    }
    assert_eq!(borrow, 0, "the operand is below twice the modulus");
    difference
}

impl Representation {
    /// The element of the words of an operand
    pub fn field_element(self, words: &[u32; 8]) -> FieldElement {
        match self {
            Self::Canonical => FieldElement::from_bytes(&be_bytes_of_words(words))
                .expect("the operand is a field element"),
            Self::Montgomery => {
                let mut element =
                    FieldElement::from_bytes(&be_bytes_of_words(&reduce_below(words, &P_WORDS)))
                        .expect("reduced below the modulus");
                element
                    .mul_in_place(&FieldElement::from_bytes(&R_INVERSE_MOD_P).expect("constant"));
                element
            }
        }
    }

    /// The words of an answer
    pub fn field_element_words(self, element: &FieldElement) -> [u32; 8] {
        let mut element = *element;
        if self == Self::Montgomery {
            element.mul_in_place(
                &FieldElement::from_bytes(&be_bytes_of_words(&R_MOD_P_WORDS)).expect("constant"),
            );
        }
        words_of_be_bytes(&element.to_bytes().into())
    }

    /// The scalar of the words of an operand
    pub fn scalar(self, words: &[u32; 8]) -> Scalar {
        match self {
            Self::Canonical => Scalar::from_be_bytes_checked(&be_bytes_of_words(words))
                .expect("the operand is a scalar"),
            Self::Montgomery => {
                let mut scalar = Scalar::from_be_bytes_checked(&be_bytes_of_words(&reduce_below(
                    words, &N_WORDS,
                )))
                .expect("reduced below the modulus");
                scalar *= &Scalar::from_be_bytes_checked(&R_INVERSE_MOD_N).expect("constant");
                scalar
            }
        }
    }

    /// The words of an answer
    pub fn scalar_words(self, scalar: &Scalar) -> [u32; 8] {
        let mut scalar = *scalar;
        if self == Self::Montgomery {
            scalar *= &Scalar::from_be_bytes_checked(&be_bytes_of_words(&R_MOD_N_WORDS))
                .expect("constant");
        }
        words_of_be_bytes(&scalar.to_repr().into())
    }
}

/// A word of the querier at `address`
fn read_querier_word(memory: &dyn QuerierMemory, address: usize) -> usize {
    let low = memory.read_u32(address);
    match memory.word_size() {
        4 => low as usize,
        8 => {
            let high = memory.read_u32(address + 4);
            usize::try_from(u64::from(low) | (u64::from(high) << 32))
                .expect("a querier word fits into usize")
        }
        size => panic!("unsupported querier word size {size}"),
    }
}

/// The request at `address` in the memory of the querier (the second word of a query): the op
/// and the words of the operand
pub fn read_request(memory: &dyn QuerierMemory, address: usize) -> (FieldHintOp, Vec<u32>) {
    assert!(address != 0, "the request is not at the null address");
    let (op, src_ptr, src_len_u32_words) = match memory.word_size() {
        4 => (
            offset_of!(FieldOpsHint<u32>, op),
            offset_of!(FieldOpsHint<u32>, src_ptr),
            offset_of!(FieldOpsHint<u32>, src_len_u32_words),
        ),
        8 => (
            offset_of!(FieldOpsHint<u64>, op),
            offset_of!(FieldOpsHint<u64>, src_ptr),
            offset_of!(FieldOpsHint<u64>, src_len_u32_words),
        ),
        size => panic!("unsupported querier word size {size}"),
    };
    let op = memory.read_u32(address + op);
    let op = FieldHintOp::parse_u32(op).unwrap_or_else(|| panic!("unknown field hint op {op}"));
    let operand = read_querier_word(memory, address + src_ptr);
    assert!(operand != 0, "the operand is not at the null address");
    let len = memory.read_u32(address + src_len_u32_words) as usize;
    let words = (0..len).map(|i| memory.read_u32(operand + 4 * i)).collect();
    (op, words)
}

fn element_words(operand: &[u32], i: usize) -> &[u32; 8] {
    operand[8 * i..8 * i + 8].try_into().expect("8 words")
}

/// The response to the hint `op` on the words `operand`, in `representation`: the words the
/// querier reads
pub fn answer(op: FieldHintOp, operand: &[u32], representation: Representation) -> Vec<u32> {
    let mut response = Vec::new();
    match op {
        FieldHintOp::Secp256k1BaseFieldSqrt => {
            assert_eq!(operand.len(), 8, "the operand is an element");
            let mut candidate = representation.field_element(element_words(operand, 0));
            assert!(!candidate.is_zero());
            // the candidate is `x^((p + 1) / 4)` either way: a root of `x` or of `-x`
            let is_residue = candidate.sqrt_in_place();
            response.extend(representation.field_element_words(&candidate));
            response.push(u32::from(!is_residue));
        }
        FieldHintOp::Secp256k1BaseFieldInverse => {
            assert_eq!(operand.len(), 8, "the operand is an element");
            let mut inverse = representation.field_element(element_words(operand, 0));
            assert!(!inverse.is_zero());
            inverse.invert_in_place();
            response.extend(representation.field_element_words(&inverse));
        }
        FieldHintOp::Secp256k1ScalarFieldInverse => {
            assert_eq!(operand.len(), 8, "the operand is a scalar");
            let scalar = representation.scalar(element_words(operand, 0));
            assert!(!scalar.is_zero());
            response.extend(representation.scalar_words(&scalar.invert_vartime()));
        }
        FieldHintOp::Secp256k1BaseFieldDivision => {
            assert_eq!(operand.len(), 16, "the operand is two elements");
            let numerator = representation.field_element(element_words(operand, 0));
            let mut quotient = representation.field_element(element_words(operand, 1));
            assert!(!quotient.is_zero());
            quotient.invert_in_place();
            quotient.mul_in_place(&numerator);
            response.extend(representation.field_element_words(&quotient));
        }
        FieldHintOp::Secp256k1RecoveryWitness => {
            assert_eq!(
                operand.len(),
                RECOVERY_REQUEST_WORDS,
                "the operand is a recovery request"
            );
            let r = representation.scalar(element_words(operand, 0));
            let s = representation.scalar(element_words(operand, 1));
            let message = representation.scalar(element_words(operand, 2));
            let point = Affine {
                x: representation.field_element(element_words(operand, 3)),
                y: representation.field_element(element_words(operand, 4)),
                infinity: false,
            };
            match recovery_witness(&point, &r, &s, &message) {
                None => response.push(0),
                Some(witness) => {
                    response.push(1);
                    response.extend(words_of_be_bytes(&witness.key_x));
                    response.extend(words_of_be_bytes(&witness.key_y));
                    for coefficient in witness.coefficients {
                        // 4 little-endian words
                        response.extend(
                            coefficient
                                .to_le_bytes()
                                .as_chunks::<4>()
                                .0
                                .iter()
                                .map(|chunk| u32::from_le_bytes(*chunk)),
                        );
                    }
                }
            }
        }
    }
    response
}

/// Serves the query whose second word is `address`: the words the querier reads
pub fn serve(
    memory: &dyn QuerierMemory,
    representation: Representation,
    address: usize,
) -> Vec<u32> {
    let (op, operand) = read_request(memory, address);
    answer(op, &operand, representation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    #[test]
    fn the_montgomery_words_are_those_of_the_delegated_field() {
        // `R mod m` is the Montgomery form of one
        assert_eq!(
            Representation::Montgomery.field_element_words(&FieldElement::ONE),
            R_MOD_P_WORDS
        );
        assert_eq!(
            Representation::Montgomery.scalar_words(&Scalar::ONE),
            R_MOD_N_WORDS
        );
        assert_eq!(
            Representation::Canonical.field_element_words(&FieldElement::ONE),
            [1, 0, 0, 0, 0, 0, 0, 0]
        );
        proptest!(|(bytes: [u8; 32], extra: bool)| {
            for representation in [Representation::Canonical, Representation::Montgomery] {
                if let Some(x) = FieldElement::from_bytes(&bytes) {
                    let mut words = representation.field_element_words(&x);
                    prop_assert!(is_below(&words, &P_WORDS));
                    prop_assert_eq!(representation.field_element(&words).to_bytes(), x.to_bytes());
                    // an operand of the delegated field may be any representative below 2^256
                    if representation == Representation::Montgomery && extra {
                        let mut carry = 0u64;
                        for (word, modulus) in words.iter_mut().zip(&P_WORDS) {
                            let t = u64::from(*word) + u64::from(*modulus) + carry;
                            *word = t as u32;
                            carry = t >> 32;
                        }
                        if carry == 0 {
                            prop_assert_eq!(representation.field_element(&words).to_bytes(), x.to_bytes());
                        }
                    }
                }
                if let Some(k) = Scalar::from_be_bytes_checked(&bytes) {
                    let words = representation.scalar_words(&k);
                    prop_assert!(is_below(&words, &N_WORDS));
                    prop_assert_eq!(representation.scalar(&words).to_repr(), k.to_repr());
                }
            }
        });
    }

    #[test]
    fn the_request_is_read_at_the_offsets_of_the_querier_word() {
        assert_eq!(offset_of!(FieldOpsHint<u32>, src_ptr), 4);
        assert_eq!(offset_of!(FieldOpsHint<u32>, src_len_u32_words), 8);
        assert_eq!(offset_of!(FieldOpsHint<u64>, src_ptr), 8);
        assert_eq!(offset_of!(FieldOpsHint<u64>, src_len_u32_words), 16);

        /// A guest memory of words
        struct Words(std::collections::BTreeMap<usize, u32>);

        impl QuerierMemory for Words {
            fn word_size(&self) -> usize {
                4
            }

            fn read_u32(&self, address: usize) -> u32 {
                *self.0.get(&address).expect("a written word")
            }
        }

        // the request at 0x100, the operand (an element in Montgomery words) at 0x1000
        let mut memory = Words(Default::default());
        memory
            .0
            .insert(0x100, FieldHintOp::Secp256k1BaseFieldInverse as u32);
        memory.0.insert(0x104, 0x1000);
        memory.0.insert(0x108, 8);
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        for (i, word) in Representation::Montgomery
            .field_element_words(&x)
            .into_iter()
            .enumerate()
        {
            memory.0.insert(0x1000 + 4 * i, word);
        }
        let response = serve(&memory, Representation::Montgomery, 0x100);
        let mut inverse = x;
        inverse.invert_in_place();
        assert_eq!(
            response,
            Representation::Montgomery.field_element_words(&inverse)
        );
    }

    #[test]
    fn the_witness_answer_is_a_flag_then_the_key_and_the_coefficients() {
        let key: [u8; 32] = [5; 32];
        let key = crate::k256::ecdsa::SigningKey::from_bytes((&key).into()).unwrap();
        let digest = [9; 32];
        let (signature, recovery_id) = key.sign_prehash_recoverable(&digest).unwrap();
        assert!(!recovery_id.is_x_reduced());
        let bytes = signature.to_bytes();
        let (r, s) = bytes.split_at(32);
        let r_scalar = Scalar::from_be_bytes_checked(r.try_into().unwrap()).unwrap();
        let s_scalar = Scalar::from_be_bytes_checked(s.try_into().unwrap()).unwrap();
        let message = Scalar::from_be_bytes_reduced(&digest);
        let point = Affine::lift_x_with_hooks(
            &FieldElement::from_bytes(r.try_into().unwrap()).unwrap(),
            recovery_id.is_y_odd(),
            &mut crate::secp256k1::hooks::DefaultSecp256k1Hooks,
        )
        .unwrap();
        let witness = recovery_witness(&point, &r_scalar, &s_scalar, &message).unwrap();

        for representation in [Representation::Canonical, Representation::Montgomery] {
            let mut operand = Vec::new();
            operand.extend(representation.scalar_words(&r_scalar));
            operand.extend(representation.scalar_words(&s_scalar));
            operand.extend(representation.scalar_words(&message));
            operand.extend(representation.field_element_words(&point.x));
            operand.extend(representation.field_element_words(&point.y));
            let response = answer(
                FieldHintOp::Secp256k1RecoveryWitness,
                &operand,
                representation,
            );
            assert_eq!(response.len(), 1 + 8 + 8 + 16);
            assert_eq!(response[0], 1);
            assert_eq!(
                be_bytes_of_words(response[1..9].try_into().unwrap()),
                witness.key_x
            );
            assert_eq!(
                be_bytes_of_words(response[9..17].try_into().unwrap()),
                witness.key_y
            );
            for (i, coefficient) in witness.coefficients.iter().enumerate() {
                let words = &response[17 + 4 * i..21 + 4 * i];
                let mut value = 0u128;
                for (j, word) in words.iter().enumerate() {
                    value |= u128::from(*word) << (32 * j);
                }
                assert_eq!(value as i128, *coefficient);
            }
        }
    }
}
