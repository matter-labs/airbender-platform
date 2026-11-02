//! The bn254 and bls12-381 field hints outside `cfg(test)`, where the fields are the arkworks ones
//! (unless `proving` is on): a bls12-381 element is 6 limbs, `x 2^384`, and the oracle converts the
//! guest's answers to `x 2^512`. The unit tests run on the delegated fields, which cannot tell the
//! targets apart.
#![cfg(all(feature = "alloc", target_pointer_width = "64"))]

use airbender_crypto::ark_ff::{BigInteger, Field, PrimeField};
use airbender_crypto::hints::{
    bls12_381_fq12_inverse, bls12_381_fq_inverse, bls12_381_fq_sqrt, bn254_fq_inverse,
    record_hints, HintAnswer, HintTarget,
};
use airbender_crypto::{bls12_381, bn254};

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
