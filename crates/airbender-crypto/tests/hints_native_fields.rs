//! The bn254 and bls12-381 field hints through the public API, on the fields of the build. Without
//! `bigint_ops` those are the arkworks ones: a bls12-381 element is 6 limbs, `x 2^384`, and the
//! oracle converts the guest's answers to `x 2^512`. With it the build has the representation of
//! the guest, and the two targets of an answer coincide.
#![cfg(all(feature = "oracle_implementations", target_pointer_width = "64"))]

use airbender_crypto::affine_glv::Divider;
use airbender_crypto::ark_ff::{AdditiveGroup, BigInteger, Field, PrimeField, Zero};
use airbender_crypto::hints::{
    bls12_381_fq12_inverse, bls12_381_fq_inverse, bls12_381_fq_sqrt, bn254_fq_inverse,
    record_hints, Bls12381FqDivider, Bn254FqDivider, HintAnswer, HintError, HintTarget,
    MemoryOracle,
};
use airbender_crypto::{bls12_381, bn254};
use core::mem::MaybeUninit;
use std::collections::VecDeque;

/// The words of the low `limbs` limbs of the canonical integer of `x`
fn words<F: PrimeField>(x: F, limbs: usize) -> Vec<u32> {
    let integer = x.into_bigint();
    assert!(integer.as_ref()[limbs..].iter().all(|limb| *limb == 0));
    integer.as_ref()[..limbs]
        .iter()
        .flat_map(|limb| [*limb as u32, (*limb >> 32) as u32])
        .collect()
}

/// `x 2^bits`
fn times_power_of_two<F: Field>(x: F, bits: usize) -> F {
    x * F::from(2u64).pow([bits as u64])
}

fn answer_words<T: HintAnswer>(value: &T, target: HintTarget) -> Vec<u32> {
    let mut response = Vec::new();
    value.write(target, &mut response);
    response
}

/// The Montgomery form of a bls12-381 element on the RISC-V guest, in the 6 limbs of the answers
fn bls12_381_guest_words(x: bls12_381::Fq) -> Vec<u32> {
    words(times_power_of_two(x, 512), 6)
}

#[test]
fn bls12_381_answers_follow_the_target() {
    let limbs = <<bls12_381::Fq as PrimeField>::BigInt as BigInteger>::NUM_LIMBS;
    let a = -bls12_381::Fq::from(0x1234_5678u64);
    let native = words(times_power_of_two(a, 64 * limbs), 6);
    let guest = bls12_381_guest_words(a);
    assert_eq!(answer_words(&a, HintTarget::Native), native);
    assert_eq!(answer_words(&a, HintTarget::Guest), guest);
    if limbs == 6 {
        // `R = 2^384` here: the targets differ
        assert_ne!(native, guest);
    }
}

#[test]
fn the_recorded_answers_are_those_of_the_guest() {
    let a = bls12_381::Fq::from(0x1234_5678u64).square();
    let inverse = a.inverse().unwrap();

    let (answer, tape) = record_hints(|oracle| bls12_381_fq_inverse(oracle, &a));
    assert_eq!(answer, Some(inverse));
    assert_eq!(tape, bls12_381_guest_words(inverse));

    // the candidate root and the non-residue flag
    let (root, tape) = record_hints(|oracle| bls12_381_fq_sqrt(oracle, &a));
    let root = root.unwrap();
    assert_eq!(root.square(), a);
    assert_eq!(tape, [bls12_381_guest_words(root), vec![0]].concat());

    // the components in order, `c0` first, recursively
    let f = bls12_381::Fq12::from_base_prime_field_elems(
        (1..=12u64).map(|i| a * bls12_381::Fq::from(i) + inverse),
    )
    .unwrap();
    let f_inverse = f.inverse().unwrap();
    let (answer, tape) = record_hints(|oracle| bls12_381_fq12_inverse(oracle, &f));
    assert_eq!(answer, Some(f_inverse));
    let mut expected = Vec::new();
    for fq6 in [&f_inverse.c0, &f_inverse.c1] {
        for fq2 in [&fq6.c0, &fq6.c1, &fq6.c2] {
            expected.extend(bls12_381_guest_words(fq2.c0));
            expected.extend(bls12_381_guest_words(fq2.c1));
        }
    }
    assert_eq!(tape, expected);

    // bn254: `R = 2^256` on every target
    let b = bn254::Fq::from(0x1234_5678u64).square();
    let b_inverse = b.inverse().unwrap();
    let (answer, tape) = record_hints(|oracle| bn254_fq_inverse(oracle, &b));
    assert_eq!(answer, Some(b_inverse));
    assert_eq!(tape, words(times_power_of_two(b_inverse, 256), 4));
}

/// An oracle that answers from a tape, as the guest's does; unread words fail the query
struct Tape(VecDeque<u32>);

impl MemoryOracle for Tape {
    type Error = HintError;

    unsafe fn send_query(&mut self, _query_id: u32, _input_word: usize) -> Result<(), HintError> {
        Ok(())
    }

    fn read_word(&mut self) -> Result<u32, HintError> {
        self.0
            .pop_front()
            .ok_or(HintError("the hint tape is exhausted"))
    }

    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
        for i in 0..num_words {
            let word = self.read_word()?;
            // SAFETY: `i < num_words`, guaranteed by the caller
            unsafe { dst.add(i).write(word) };
        }
        Ok(())
    }

    fn finish_query(&mut self) -> Result<(), HintError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(HintError("the hint tape has excess words"))
        }
    }
}

/// The words of `words` as an integer plus the modulus of `F`: another representative of the
/// same element, which is not canonical
fn plus_modulus<F: PrimeField>(words: &[u32]) -> Vec<u32> {
    let modulus = F::MODULUS;
    let mut carry = 0u64;
    let mut sum = Vec::new();
    for (i, word) in words.iter().enumerate() {
        let limb = modulus.as_ref()[i / 2];
        let half = if i % 2 == 0 {
            limb as u32
        } else {
            (limb >> 32) as u32
        };
        let t = u64::from(*word) + u64::from(half) + carry;
        sum.push(t as u32);
        carry = t >> 32;
    }
    assert_eq!(carry, 0, "the sum fits the words");
    sum
}

/// The panic message of `call`
fn panic_message<R>(call: impl FnOnce() -> R) -> String {
    let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        .map(drop)
        .unwrap_err();
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap_or_default()
}

/// The divisions of the curve arithmetic on a tape: the quotient is taken, and every other
/// answer is rejected, the quotient's other representative included
macro_rules! division_tests {
    ($name:ident, $field:ty, $divider:ident) => {
        #[test]
        fn $name() {
            type F = $field;
            let x = F::from(0x1234_5678_9abc_def1u64).pow([7]);
            let elements = [F::ZERO, F::ONE, -F::ONE, x, -x, x.square(), F::from(2u64)];
            let divide = |n: F, d: F, tape: Vec<u32>| {
                let mut oracle = Tape(tape.into());
                let mut quotient = MaybeUninit::uninit();
                let q = *$divider::new(&mut oracle).divide(&mut [n, d], &mut quotient);
                assert!(oracle.0.is_empty());
                q
            };
            for n in elements {
                for d in elements.into_iter().filter(|d| !d.is_zero()) {
                    let q = n / d;
                    let answer = answer_words(&q, HintTarget::Native);
                    assert_eq!(divide(n, d, answer.clone()), q);

                    // other elements
                    for wrong in [q + F::ONE, q - F::ONE, q + x] {
                        let message = panic_message(|| {
                            divide(n, d, answer_words(&wrong, HintTarget::Native))
                        });
                        assert!(message.contains("division hint is wrong"), "{message}");
                    }
                    // the quotient, not canonical
                    let message = panic_message(|| divide(n, d, plus_modulus::<F>(&answer)));
                    assert!(message.contains("canonical"), "{message}");
                    // a short answer, and a long one
                    let short = answer[..answer.len() - 1].to_vec();
                    assert!(!panic_message(|| divide(n, d, short)).is_empty());
                    let long = [answer.clone(), vec![0]].concat();
                    assert!(!panic_message(|| divide(n, d, long)).is_empty());
                }
            }
            // the modulus itself, and all ones
            let answer = answer_words(&F::ZERO, HintTarget::Native);
            for words in [plus_modulus::<F>(&answer), vec![u32::MAX; answer.len()]] {
                let message = panic_message(|| divide(F::ZERO, F::ONE, words));
                assert!(message.contains("canonical"), "{message}");
            }
        }
    };
}

division_tests!(
    bn254_divisions_check_the_quotient,
    bn254::Fq,
    Bn254FqDivider
);
division_tests!(
    bls12_381_divisions_check_the_quotient,
    bls12_381::Fq,
    Bls12381FqDivider
);
