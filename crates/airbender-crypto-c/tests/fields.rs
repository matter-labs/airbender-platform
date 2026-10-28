//! The C field bindings: encodings, failure atomicity, and the recorded hints (the tape of a C call
//! is that of the Rust hinted operation it stands for).
mod common;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::{call, encode, recorded};
use core::mem::MaybeUninit;
use crypto::affine_glv::Divider;
use crypto::ark_ff::{Field, PrimeField};
use crypto::hints::{self, record_hints, RecordingHintOracle, Secp256k1Element};
use crypto::hints::{Bls12381FqDivider, Bn254FqDivider, Secp256k1HooksWithOracle};
use crypto::secp256k1::{field::FieldElement, hooks::Secp256k1Hooks, scalars::Scalar};
use crypto::{bls12_381 as bls, bn254 as bn, MiniDigest};

type Unary<const N: usize> = unsafe extern "C" fn(*const Bytes<N>, *mut Bytes<N>) -> i32;
type Binary<const N: usize> =
    unsafe extern "C" fn(*const Bytes<N>, *const Bytes<N>, *mut Bytes<N>) -> i32;

/// One is its own inverse and quotient (in place too); a zero divisor and a non-canonical operand
/// fail and leave the output unchanged
fn field<const N: usize>(width: usize, inverse: Unary<N>, divide: Binary<N>) {
    let mut one = Bytes { data: [0; N] };
    one.data[width - 1] = 1; // c0 first, including Fq2/Fq12
    let mut output = Bytes { data: [0xa5; N] };
    unsafe {
        assert_eq!(inverse(&one, &mut output), EOK);
        assert_eq!(output, one);
        assert_eq!(divide(&output, &one, &mut output), EOK); // overlap
        assert_eq!(output, one);
        assert_eq!(divide(&one, &Bytes { data: [0; N] }, &mut output), EFAIL);
        assert_eq!(output, one);
        assert_eq!(inverse(&Bytes { data: [0xff; N] }, &mut output), EFAIL);
        assert_eq!(output, one);
    }
}

#[test]
fn canonical_big_endian_components_and_failure_atomicity() {
    field(32, airbender_bn254_fq_inverse, airbender_bn254_fq_divide);
    field(32, airbender_bn254_fq2_inverse, airbender_bn254_fq2_divide);
    field(
        32,
        airbender_bn254_fq12_inverse,
        airbender_bn254_fq12_divide,
    );
    field(48, airbender_bls12_fq_inverse, airbender_bls12_fq_divide);
    field(48, airbender_bls12_fq2_inverse, airbender_bls12_fq2_divide);
    field(
        48,
        airbender_bls12_fq12_inverse,
        airbender_bls12_fq12_divide,
    );
    field(
        32,
        airbender_secp256k1_fe_inverse,
        airbender_secp256k1_fe_divide,
    );
}

#[test]
fn the_same_contracts_hold_while_recording() {
    recorded(canonical_big_endian_components_and_failure_atomicity);
}

fn keccak(data: &[u8]) -> [u8; 32] {
    <crypto::sha3::Keccak256 as MiniDigest>::digest(data).0
}

/// A pseudo-random element of `F`
fn element<F: Field>(seed: u8) -> F {
    let components = (0..F::extension_degree())
        .map(|j| F::BasePrimeField::from_be_bytes_mod_order(&keccak(&[seed, j as u8])));
    F::from_base_prime_field_elems(components).unwrap()
}

fn decode<F: Field, const N: usize>(bytes: &Bytes<N>) -> F {
    let width = N / F::extension_degree() as usize;
    let components = bytes
        .data
        .chunks_exact(width)
        .map(F::BasePrimeField::from_be_bytes_mod_order);
    F::from_base_prime_field_elems(components).unwrap()
}

/// A Rust hinted operation on `(a, b)`, for its tape
type Hinted<F> = fn(&mut RecordingHintOracle, &F, &F);

/// The inverse and the quotient are right, with the tape of `hinted_inverse(a)` and
/// `hinted_divide(a, b)`; zero operands fail before any hint
fn hinted_field<F: Field, const N: usize>(
    inverse: Unary<N>,
    divide: Binary<N>,
    hinted_inverse: Hinted<F>,
    hinted_divide: Hinted<F>,
) {
    let (a, b) = (element::<F>(1), element::<F>(2));
    let ((status, output), tape) = call(Bytes { data: [0; N] }, |out| unsafe {
        inverse(&encode(&[a]), out)
    });
    assert_eq!((status, decode::<F, N>(&output) * a), (EOK, F::ONE));
    assert!(!tape.is_empty());
    assert_eq!(tape, record_hints(|o| hinted_inverse(o, &a, &b)).1);
    let ((status, output), tape) = call(Bytes { data: [0; N] }, |out| unsafe {
        divide(&encode(&[a]), &encode(&[b]), out)
    });
    assert_eq!((status, decode::<F, N>(&output) * b), (EOK, a));
    assert!(!tape.is_empty());
    assert_eq!(tape, record_hints(|o| hinted_divide(o, &a, &b)).1);

    let (zero, one) = (encode::<F, N>(&[F::ZERO]), encode::<F, N>(&[F::ONE]));
    let unchanged = Bytes { data: [0xa5; N] };
    let failed = ((EFAIL, unchanged), vec![]);
    assert_eq!(
        call(unchanged, |out| unsafe { inverse(&zero, out) }),
        failed
    );
    assert_eq!(
        call(unchanged, |out| unsafe { divide(&one, &zero, out) }),
        failed
    );
}

/// `a / b` with a hinted divider
fn quotient<F: Copy, D: Divider<F>>(mut divider: D, a: &F, b: &F) -> F {
    let mut fraction = [*a, *b];
    let mut quotient = MaybeUninit::uninit();
    *divider.divide(&mut fraction, &mut quotient)
}

/// The bn254 `Fq2` inverse is one base-field inverse hint, of the norm `c0² + c1²`
fn bn254_norm_inverse(oracle: &mut RecordingHintOracle, value: &bn::Fq2) {
    _ = hints::bn254_fq_inverse(oracle, &(value.c0.square() + value.c1.square()));
}

#[test]
fn bn254_and_bls12_381_field_hints() {
    use hints::{bls12_381_fq12_inverse, bls12_381_fq2_inverse, bls12_381_fq_inverse};
    use hints::{bn254_fq12_inverse, bn254_fq_inverse};
    hinted_field::<bn::Fq, 32>(
        airbender_bn254_fq_inverse,
        airbender_bn254_fq_divide,
        |o, a, _| _ = bn254_fq_inverse(o, a),
        |o, a, b| _ = quotient(Bn254FqDivider::new(o), a, b),
    );
    hinted_field::<bn::Fq2, 64>(
        airbender_bn254_fq2_inverse,
        airbender_bn254_fq2_divide,
        |o, a, _| bn254_norm_inverse(o, a),
        |o, _, b| bn254_norm_inverse(o, b),
    );
    hinted_field::<bn::Fq12, 384>(
        airbender_bn254_fq12_inverse,
        airbender_bn254_fq12_divide,
        |o, a, _| _ = bn254_fq12_inverse(o, a),
        |o, _, b| _ = bn254_fq12_inverse(o, b),
    );
    hinted_field::<bls::Fq, 48>(
        airbender_bls12_fq_inverse,
        airbender_bls12_fq_divide,
        |o, a, _| _ = bls12_381_fq_inverse(o, a),
        |o, a, b| _ = quotient(Bls12381FqDivider::new(o), a, b),
    );
    hinted_field::<bls::Fq2, 96>(
        airbender_bls12_fq2_inverse,
        airbender_bls12_fq2_divide,
        |o, a, _| _ = bls12_381_fq2_inverse(o, a),
        |o, _, b| _ = bls12_381_fq2_inverse(o, b),
    );
    hinted_field::<bls::Fq12, 576>(
        airbender_bls12_fq12_inverse,
        airbender_bls12_fq12_divide,
        |o, a, _| _ = bls12_381_fq12_inverse(o, a),
        |o, _, b| _ = bls12_381_fq12_inverse(o, b),
    );
}

/// The secp256k1 bindings take the hints of `Secp256k1HooksWithOracle`
#[test]
fn secp256k1_field_hints() {
    let (a, b) = (keccak(&[1]), keccak(&[2]));
    let element = |bytes: [u8; 32]| FieldElement::from_bytes(&bytes).unwrap();
    let ((status, output), tape) = call(Bytes { data: [0; 32] }, |out| unsafe {
        airbender_secp256k1_fe_inverse(&Bytes { data: a }, out)
    });
    let mut inverse = element(a);
    let ((), hinted) =
        record_hints(|o| Secp256k1HooksWithOracle::new(o).fe_invert_and_assign(&mut inverse));
    assert_eq!((status, output.data), (EOK, inverse.to_bytes().into()));
    assert_eq!(tape, hinted);
    // the raw words that the guest reads: the Montgomery words of the inverse, unframed
    assert_eq!(tape, inverse.to_montgomery_words());

    let ((status, output), tape) = call(Bytes { data: [0; 32] }, |out| unsafe {
        airbender_secp256k1_fe_divide(&Bytes { data: a }, &Bytes { data: b }, out)
    });
    let (quotient, hinted) = record_hints(|o| {
        let mut fraction = [element(a), element(b)];
        let mut quotient = MaybeUninit::uninit();
        *Secp256k1HooksWithOracle::new(o).fe_divide(&mut fraction, &mut quotient)
    });
    assert_eq!((status, output.data), (EOK, quotient.to_bytes().into()));
    assert_eq!(tape, hinted);

    let scalar = Scalar::from_be_bytes_reduced(&a);
    let ((status, output), tape) = call(Bytes { data: [0; 32] }, |out| unsafe {
        let a = Bytes {
            data: scalar.to_repr().into(),
        };
        airbender_secp256k1_scalar_inverse(&a, out)
    });
    let mut inverse = scalar;
    let ((), hinted) =
        record_hints(|o| Secp256k1HooksWithOracle::new(o).scalar_invert_and_assign(&mut inverse));
    assert_eq!((status, output.data), (EOK, inverse.to_repr().into()));
    assert_eq!(tape, hinted);

    // zero and non-canonical operands fail before any hint
    let (zero, p) = (Bytes { data: [0; 32] }, Bytes { data: [0xff; 32] });
    let unchanged = Bytes { data: [0xa5; 32] };
    let failing: [&dyn Fn(&mut Bytes<32>) -> i32; 3] = [
        &|out| unsafe { airbender_secp256k1_fe_inverse(&zero, out) },
        &|out| unsafe { airbender_secp256k1_scalar_inverse(&zero, out) },
        &|out| unsafe { airbender_secp256k1_fe_sqrt(&p, out) },
    ];
    for f in failing {
        assert_eq!(call(unchanged, f), ((EFAIL, unchanged), vec![]));
    }
}

/// The square root is the even one, on both paths, also where the natural root `x^((p+1)/4)`
/// (which the oracle answers) is odd: 9 for 81. A nonsquare fails, proven by a hint.
#[test]
fn secp256k1_square_roots_are_even() {
    let small = |value: u8| {
        let mut bytes = [0u8; 32];
        bytes[31] = value;
        bytes
    };
    let sqrt = |value: u8| {
        call(Bytes { data: [0xa5; 32] }, |out| unsafe {
            airbender_secp256k1_fe_sqrt(&Bytes { data: small(value) }, out)
        })
    };
    let hinted = |value: u8| {
        let mut x = FieldElement::from_bytes(&small(value)).unwrap();
        record_hints(|o| Secp256k1HooksWithOracle::new(o).fe_sqrt_and_assign(&mut x)).1
    };
    let mut minus_nine = FieldElement::from_bytes(&small(9)).unwrap();
    minus_nine.negate_in_place(1);
    let minus_nine = minus_nine.to_bytes().into();
    for (square, root) in [(4, small(2)), (81, minus_nine), (0, small(0))] {
        let ((status, output), tape) = sqrt(square);
        assert_eq!((status, output.data), (EOK, root));
        assert_eq!(tape, hinted(square));
        assert_eq!(tape.is_empty(), square == 0);
    }
    let ((status, output), tape) = sqrt(3);
    assert_eq!((status, output.data), (EFAIL, [0xa5; 32]));
    assert!(!tape.is_empty());
    assert_eq!(tape, hinted(3));
}
