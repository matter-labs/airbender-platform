//! Encoding and failure contracts of the extra C field bindings, and the hints that they record:
//! the tape of a C call is the tape of the Rust hinted operation that it stands for.
mod common;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::{recorded, take_recording, with_and_without_recording};
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use crypto::affine_glv::Divider;
use crypto::ark_ff::{AdditiveGroup, BigInteger, Field, PrimeField};
use crypto::hints::{self, record_hints, RecordingHintOracle, Secp256k1Element};
use crypto::secp256k1::{field::FieldElement, hooks::Secp256k1Hooks, scalars::Scalar};
use crypto::{bls12_381 as bls, bn254 as bn, MiniDigest};

type Unary<const N: usize> = unsafe extern "C" fn(*const Bytes<N>, *mut Bytes<N>) -> i32;
type Binary<const N: usize> =
    unsafe extern "C" fn(*const Bytes<N>, *const Bytes<N>, *mut Bytes<N>) -> i32;

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

#[test]
fn recorded_hints_are_raw_guest_words() {
    let mut value = Bytes { data: [0; 32] };
    value.data[31] = 17;
    let mut inverse = Bytes { data: [0; 32] };
    let mut words = [0u32; 8];
    unsafe {
        airbender_crypto_record_begin();
        assert_eq!(airbender_secp256k1_fe_inverse(&value, &mut inverse), EOK);
        // too small: nothing is copied, and the words are kept for the next call
        assert_eq!(airbender_crypto_record_end(words.as_mut_ptr(), 7), 8);
        assert_eq!(words, [0; 8]);
        assert_eq!(airbender_crypto_record_end(words.as_mut_ptr(), 8), 8);
    }
    // The answer of one inverse query, as the RISC-V guest reads it: the Montgomery words of the
    // inverse, with no length or other framing
    let inverse = FieldElement::from_bytes(&inverse.data).unwrap();
    assert_eq!(words, inverse.to_montgomery_words());
    // claimed: a further end has nothing
    assert!(take_recording().is_empty());
}

fn keccak(data: &[u8]) -> [u8; 32] {
    <crypto::sha3::Keccak256 as MiniDigest>::digest(data).0
}

/// Distinct pseudo-random canonical elements of `F`
fn elements<F: Field>(count: u8) -> Vec<F> {
    (1..=count)
        .map(|i| {
            let components = (0..F::extension_degree())
                .map(|j| F::BasePrimeField::from_be_bytes_mod_order(&keccak(&[i, j as u8, 0x5a])));
            F::from_base_prime_field_elems(components).unwrap()
        })
        .collect()
}

/// The C encoding of `value`: canonical big-endian base-field components, `c0` first
fn encode<F: Field, const N: usize>(value: &F) -> Bytes<N> {
    let mut output = Bytes { data: [0; N] };
    let width = N / F::extension_degree() as usize;
    for (dst, component) in output
        .data
        .chunks_exact_mut(width)
        .zip(value.to_base_prime_field_elements())
    {
        let bytes = component.into_bigint().to_bytes_be();
        // the delegated big integers of bls12-381 are 64 bytes: the value is in the low ones
        dst.copy_from_slice(&bytes[bytes.len() - width..]);
    }
    output
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

/// For pseudo-random operands, the C inverse and quotient are right, recording does not change
/// them, and the tape of each is that of `hinted_inverse(a)` and `hinted_divide(a, b)`; zero
/// operands fail and record nothing.
fn hinted_field<F: Field, const N: usize>(
    inverse: Unary<N>,
    divide: Binary<N>,
    hinted_inverse: Hinted<F>,
    hinted_divide: Hinted<F>,
) {
    let values = elements::<F>(4);
    for pair in values.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (result, tape) = with_and_without_recording(|| {
            let mut output = Bytes { data: [0; N] };
            let status = unsafe { inverse(&encode(&a), &mut output) };
            (status, output)
        });
        assert_eq!(result.0, EOK);
        assert_eq!(decode::<F, N>(&result.1) * a, F::ONE);
        assert!(!tape.is_empty());
        assert_eq!(tape, record_hints(|o| hinted_inverse(o, &a, &b)).1);

        let (result, tape) = with_and_without_recording(|| {
            let mut output = Bytes { data: [0; N] };
            let status = unsafe { divide(&encode(&a), &encode(&b), &mut output) };
            (status, output)
        });
        assert_eq!(result.0, EOK);
        assert_eq!(decode::<F, N>(&result.1) * b, a);
        assert!(!tape.is_empty());
        assert_eq!(tape, record_hints(|o| hinted_divide(o, &a, &b)).1);
    }
    let zero = encode::<F, N>(&F::ZERO);
    let one = encode::<F, N>(&F::ONE);
    let (status, tape) = recorded(|| unsafe { inverse(&zero, &mut Bytes { data: [0; N] }) });
    assert_eq!((status, tape), (EFAIL, vec![]));
    let (status, tape) = recorded(|| unsafe { divide(&one, &zero, &mut Bytes { data: [0; N] }) });
    assert_eq!((status, tape), (EFAIL, vec![]));
}

/// `a / b` with a hinted divider
fn quotient<F: Copy, D: Divider<F>>(mut divider: D, a: &F, b: &F) -> F {
    let mut fraction = [*a, *b];
    let mut quotient = MaybeUninit::uninit();
    *divider.divide(&mut fraction, &mut quotient)
}

/// The bn254 `Fq2` inverse takes one base-field inverse hint, of the norm `c0² + c1²` (bn254
/// `Fq2` is `Fq[u] / (u² + 1)`): no hint of its own
fn bn254_norm_inverse(oracle: &mut RecordingHintOracle, value: &bn::Fq2) -> Option<bn::Fq> {
    hints::bn254_fq_inverse(oracle, &(value.c0.square() + value.c1.square()))
}

#[test]
fn bn254_field_hints() {
    hinted_field::<bn::Fq, 32>(
        airbender_bn254_fq_inverse,
        airbender_bn254_fq_divide,
        |o, a, _| {
            let _ = hints::bn254_fq_inverse(o, a);
        },
        |o, a, b| {
            let _ = quotient(hints::Bn254FqDivider::new(o), a, b);
        },
    );
    hinted_field::<bn::Fq2, 64>(
        airbender_bn254_fq2_inverse,
        airbender_bn254_fq2_divide,
        |o, a, _| {
            let _ = bn254_norm_inverse(o, a);
        },
        |o, _, b| {
            let _ = bn254_norm_inverse(o, b);
        },
    );
    hinted_field::<bn::Fq12, 384>(
        airbender_bn254_fq12_inverse,
        airbender_bn254_fq12_divide,
        |o, a, _| {
            let _ = hints::bn254_fq12_inverse(o, a);
        },
        |o, _, b| {
            let _ = hints::bn254_fq12_inverse(o, b);
        },
    );
    // the norm construction relies on `u² = -1`
    let u = bn::Fq2::new(bn::Fq::ZERO, bn::Fq::ONE);
    assert_eq!(u.square(), -bn::Fq2::ONE);
}

#[test]
fn bls12_381_field_hints() {
    hinted_field::<bls::Fq, 48>(
        airbender_bls12_fq_inverse,
        airbender_bls12_fq_divide,
        |o, a, _| {
            let _ = hints::bls12_381_fq_inverse(o, a);
        },
        |o, a, b| {
            let _ = quotient(hints::Bls12381FqDivider::new(o), a, b);
        },
    );
    hinted_field::<bls::Fq2, 96>(
        airbender_bls12_fq2_inverse,
        airbender_bls12_fq2_divide,
        |o, a, _| {
            let _ = hints::bls12_381_fq2_inverse(o, a);
        },
        |o, _, b| {
            let _ = hints::bls12_381_fq2_inverse(o, b);
        },
    );
    hinted_field::<bls::Fq12, 576>(
        airbender_bls12_fq12_inverse,
        airbender_bls12_fq12_divide,
        |o, a, _| {
            let _ = hints::bls12_381_fq12_inverse(o, a);
        },
        |o, _, b| {
            let _ = hints::bls12_381_fq12_inverse(o, b);
        },
    );
}

/// The secp256k1 bindings take the hints of `Secp256k1HooksWithOracle`: the base field inverse,
/// division and square root, and the scalar inverse
#[test]
fn secp256k1_field_hints() {
    let bytes = |i: u8| keccak(&[i, 0xec]);
    let element = |i: u8| FieldElement::from_bytes(&bytes(i)).unwrap();
    let call = |f: &dyn Fn(&mut Bytes<32>) -> i32| {
        let mut output = Bytes { data: [0; 32] };
        let status = f(&mut output);
        (status, output)
    };
    for i in 1..4 {
        let (a, b) = (element(i), element(i + 1));
        let mut expected = a;
        expected.invert_in_place();
        let ((status, output), tape) = with_and_without_recording(|| {
            call(&|out| unsafe { airbender_secp256k1_fe_inverse(&Bytes { data: bytes(i) }, out) })
        });
        assert_eq!((status, output.data), (EOK, expected.to_bytes().into()));
        let mut x = a;
        let ((), hinted) =
            record_hints(|o| hints::Secp256k1HooksWithOracle::new(o).fe_invert_and_assign(&mut x));
        assert_eq!(tape, hinted);

        let ((status, output), tape) = with_and_without_recording(|| {
            call(&|out| unsafe {
                airbender_secp256k1_fe_divide(
                    &Bytes { data: bytes(i) },
                    &Bytes { data: bytes(i + 1) },
                    out,
                )
            })
        });
        let mut q = b;
        q.invert_in_place();
        q.mul_in_place(&a);
        assert_eq!((status, output.data), (EOK, q.to_bytes().into()));
        let (_, hinted) = record_hints(|o| {
            let mut fraction = [a, b];
            let mut quotient = MaybeUninit::uninit();
            *hints::Secp256k1HooksWithOracle::new(o).fe_divide(&mut fraction, &mut quotient)
        });
        assert_eq!(tape, hinted);

        // the square of a, then a itself (a square or not)
        let mut square = a;
        square.square_in_place();
        for operand in [square, a] {
            let ((status, output), tape) = with_and_without_recording(|| {
                call(&|out| unsafe {
                    airbender_secp256k1_fe_sqrt(
                        &Bytes {
                            data: operand.to_bytes().into(),
                        },
                        out,
                    )
                })
            });
            let mut x = operand;
            let (is_square, hinted) = record_hints(|o| {
                hints::Secp256k1HooksWithOracle::new(o).fe_sqrt_and_assign(&mut x)
            });
            assert_eq!(tape, hinted);
            if is_square {
                // the even root, on both paths (`with_and_without_recording`)
                assert_eq!(output.data[31] & 1, 0);
                let mut root = FieldElement::from_bytes(&output.data).unwrap();
                root.square_in_place();
                assert_eq!((status, root.to_bytes()), (EOK, operand.to_bytes()));
            } else {
                assert_eq!((status, output.data), (EFAIL, [0; 32]));
            }
        }

        let scalar = Scalar::from_be_bytes_reduced(&bytes(i));
        let ((status, output), tape) = with_and_without_recording(|| {
            call(&|out| unsafe {
                airbender_secp256k1_scalar_inverse(
                    &Bytes {
                        data: scalar.to_repr().into(),
                    },
                    out,
                )
            })
        });
        let mut y = scalar;
        let ((), hinted) = record_hints(|o| {
            hints::Secp256k1HooksWithOracle::new(o).scalar_invert_and_assign(&mut y)
        });
        assert_eq!((status, output.data), (EOK, y.to_repr().into()));
        assert_eq!(tape, hinted);
    }
    // zero and non-canonical operands fail before any hint
    let zero = Bytes { data: [0; 32] };
    let p = Bytes { data: [0xff; 32] };
    for (status, tape) in [
        recorded(|| unsafe { airbender_secp256k1_fe_inverse(&zero, &mut Bytes { data: [0; 32] }) }),
        recorded(|| unsafe {
            airbender_secp256k1_scalar_inverse(&zero, &mut Bytes { data: [0; 32] })
        }),
        recorded(|| unsafe {
            airbender_secp256k1_fe_divide(&zero, &zero, &mut Bytes { data: [0; 32] })
        }),
        recorded(|| unsafe { airbender_secp256k1_fe_sqrt(&p, &mut Bytes { data: [0; 32] }) }),
    ] {
        assert_eq!((status, tape), (EFAIL, vec![]));
    }
    // sqrt(0) = 0 needs no hint
    let (status, tape) =
        recorded(|| unsafe { airbender_secp256k1_fe_sqrt(&zero, &mut Bytes { data: [0xa5; 32] }) });
    assert_eq!((status, tape), (EOK, vec![]));
    // the pointer contract holds for an empty recording too
    unsafe {
        assert_eq!(
            airbender_crypto_record_end(NonNull::dangling().as_ptr(), 0),
            0
        )
    };
}

/// The square root is the even one, with or without hints, also where the natural root
/// `x^((p+1)/4)` (which the oracle answers and the default hook computes) is odd; a nonsquare
/// fails without changing the output
#[test]
fn secp256k1_square_roots_are_even() {
    let bytes = |value: u8| {
        let mut bytes = [0u8; 32];
        bytes[31] = value;
        bytes
    };
    let minus = |value: u8| {
        let mut x = FieldElement::from_bytes(&bytes(value)).unwrap();
        x.negate_in_place(1);
        x.to_bytes().into()
    };
    // 4: 2 (natural 2); 36: 6 (natural p - 6); 81: p - 9 (natural 9); 0: 0
    for (square, root) in [
        (bytes(4), bytes(2)),
        (bytes(36), bytes(6)),
        (bytes(81), minus(9)),
        (bytes(0), bytes(0)),
    ] {
        let ((status, output), tape) = with_and_without_recording(|| {
            let mut output = Bytes { data: [0xa5; 32] };
            let status =
                unsafe { airbender_secp256k1_fe_sqrt(&Bytes { data: square }, &mut output) };
            (status, output.data)
        });
        assert_eq!((status, output), (EOK, root));
        assert_eq!(tape.is_empty(), square == [0; 32]);
    }
    for nonsquare in [bytes(3), bytes(5)] {
        let ((status, output), tape) = with_and_without_recording(|| {
            let mut output = Bytes { data: [0xa5; 32] };
            let status =
                unsafe { airbender_secp256k1_fe_sqrt(&Bytes { data: nonsquare }, &mut output) };
            (status, output.data)
        });
        assert_eq!((status, output), (EFAIL, [0xa5; 32]));
        assert!(!tape.is_empty(), "the nonsquare is proven by a hint");
    }
}
