//! C bindings for field inversion, division and square roots: airbender-crypto's checked hinted
//! operations with an oracle, the field arithmetic without one.
use crate::{commit, crypto, decode, encode_into, read, Bytes};
use core::mem::MaybeUninit;
use crypto::ark_ff::{Field, Zero};
use crypto::secp256k1::hooks::{DefaultSecp256k1Hooks, Secp256k1Hooks};
use crypto::secp256k1::{field::FieldElement, scalars::Scalar};
#[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
use crypto::{
    affine_glv::Divider,
    hints::{self, HintOracle, Secp256k1HooksWithOracle},
};
use crypto::{bls12_381 as bls, bn254 as bn};

fn encode<F: Field, const N: usize>(value: F) -> Bytes<N> {
    let mut output = Bytes { data: [0; N] };
    encode_into(value, &mut output.data);
    output
}

/// `a / b` from a hinted divider, for a non-zero `b`
#[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
fn quotient<F: Copy, D: Divider<F>>(mut divider: D, a: F, b: F) -> F {
    let mut fraction = [a, b];
    let mut quotient = MaybeUninit::uninit();
    *divider.divide(&mut fraction, &mut quotient)
}

/// The inverse in bn254 `Fq2 = Fq[u] / (u² + 1)` from one checked base-field inverse hint, of
/// the norm `c0² + c1²`, as airbender-crypto's `bls12_381_fq2_inverse` takes it (the norm of a
/// non-zero element is not zero, as -1 is not a square in Fq); `None` for zero. It adds no hint
/// of its own.
#[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
fn bn254_fq2_inverse<O: HintOracle>(oracle: &mut O, value: &bn::Fq2) -> Option<bn::Fq2> {
    let norm = value.c0.square() + value.c1.square();
    let inverse = hints::bn254_fq_inverse(oracle, &norm)?;
    Some(bn::Fq2::new(value.c0 * inverse, -(value.c1 * inverse)))
}

/// The field bindings of one field: the inverse and the quotient (for a non-zero divisor) with
/// the oracle `o`, or else with the field's own inverse
macro_rules! field_bindings {
    (
        $field:ty, $size:literal, $inverse:ident, $divide:ident,
        inverse: |$o:ident, $a:ident| $hinted_inverse:expr,
        divide: |$o2:ident, $n:ident, $d:ident| $hinted_divide:expr $(,)?
    ) => {
        #[no_mangle]
        /// # Safety
        /// See the crate-level pointer contract and airbender_crypto.h.
        pub unsafe extern "C" fn $inverse(
            a: *const Bytes<$size>,
            result: *mut Bytes<$size>,
        ) -> i32 {
            commit(result, || {
                let $a = decode::<$field>(&read(a).data)?;
                hinted!(|$o| $hinted_inverse, $a.inverse()).map(encode)
            })
        }
        #[no_mangle]
        /// # Safety
        /// See the crate-level pointer contract and airbender_crypto.h.
        pub unsafe extern "C" fn $divide(
            a: *const Bytes<$size>,
            b: *const Bytes<$size>,
            result: *mut Bytes<$size>,
        ) -> i32 {
            commit(result, || {
                let $n = decode::<$field>(&read(a).data)?;
                let $d = decode::<$field>(&read(b).data)?;
                if $d.is_zero() {
                    return None;
                }
                let quotient: Option<$field> = hinted!(
                    |$o2| $hinted_divide,
                    $d.inverse().map(|inverse| $n * inverse)
                );
                quotient.map(encode)
            })
        }
    };
}

field_bindings!(
    bn::Fq, 32, airbender_bn254_fq_inverse, airbender_bn254_fq_divide,
    inverse: |o, a| hints::bn254_fq_inverse(o, &a),
    divide: |o, a, b| Some(quotient(hints::Bn254FqDivider::new(o), a, b)),
);
field_bindings!(
    bn::Fq2, 64, airbender_bn254_fq2_inverse, airbender_bn254_fq2_divide,
    inverse: |o, a| bn254_fq2_inverse(o, &a),
    divide: |o, a, b| bn254_fq2_inverse(o, &b).map(|inverse| a * inverse),
);
field_bindings!(
    bn::Fq12, 384, airbender_bn254_fq12_inverse, airbender_bn254_fq12_divide,
    inverse: |o, a| hints::bn254_fq12_inverse(o, &a),
    divide: |o, a, b| hints::bn254_fq12_inverse(o, &b).map(|inverse| a * inverse),
);
field_bindings!(
    bls::Fq, 48, airbender_bls12_fq_inverse, airbender_bls12_fq_divide,
    inverse: |o, a| hints::bls12_381_fq_inverse(o, &a),
    divide: |o, a, b| Some(quotient(hints::Bls12381FqDivider::new(o), a, b)),
);
field_bindings!(
    bls::Fq2, 96, airbender_bls12_fq2_inverse, airbender_bls12_fq2_divide,
    inverse: |o, a| hints::bls12_381_fq2_inverse(o, &a),
    divide: |o, a, b| hints::bls12_381_fq2_inverse(o, &b).map(|inverse| a * inverse),
);
field_bindings!(
    bls::Fq12, 576, airbender_bls12_fq12_inverse, airbender_bls12_fq12_divide,
    inverse: |o, a| hints::bls12_381_fq12_inverse(o, &a),
    divide: |o, a, b| hints::bls12_381_fq12_inverse(o, &b).map(|inverse| a * inverse),
);

/// `f(hooks)` with the secp256k1 hooks of the call: `Secp256k1HooksWithOracle` with an oracle,
/// which takes the base field inverse, division and square root and the scalar inverse as
/// checked hints, and the default hooks without one
macro_rules! with_secp256k1_hooks {
    (|$hooks:ident| $body:expr) => {
        hinted!(
            |oracle| {
                let $hooks = &mut Secp256k1HooksWithOracle::new(oracle);
                $body
            },
            {
                let $hooks = &mut DefaultSecp256k1Hooks;
                $body
            },
        )
    };
}

fn fe_bytes(value: FieldElement) -> Bytes<32> {
    Bytes {
        data: value.to_bytes().into(),
    }
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_secp256k1_fe_inverse(
    a: *const Bytes<32>,
    result: *mut Bytes<32>,
) -> i32 {
    commit(result, || {
        let mut a = FieldElement::from_bytes(&read(a).data)?;
        if a.is_zero() {
            return None;
        }
        with_secp256k1_hooks!(|hooks| hooks.fe_invert_and_assign(&mut a));
        Some(fe_bytes(a))
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_secp256k1_scalar_inverse(
    a: *const Bytes<32>,
    result: *mut Bytes<32>,
) -> i32 {
    commit(result, || {
        let mut a = Scalar::from_be_bytes_checked(&read(a).data)?;
        if a.is_zero() {
            return None;
        }
        with_secp256k1_hooks!(|hooks| hooks.scalar_invert_and_assign(&mut a));
        Some(Bytes {
            data: a.to_repr().into(),
        })
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_secp256k1_fe_sqrt(
    a: *const Bytes<32>,
    result: *mut Bytes<32>,
) -> i32 {
    commit(result, || {
        let a = FieldElement::from_bytes(&read(a).data)?;
        with_secp256k1_hooks!(|hooks| even_sqrt(hooks, a)).map(fe_bytes)
    })
}

/// The even square root of `a` (the one root of the C API), `None` for a nonsquare. The hooks
/// give either root: a hinted root is only checked to square to `a`, so its sign is the
/// prover's, and the root is normalized here, on both paths.
fn even_sqrt<H: Secp256k1Hooks>(hooks: &mut H, a: FieldElement) -> Option<FieldElement> {
    let mut root = a;
    if !hooks.fe_sqrt_and_assign(&mut root) {
        return None;
    }
    root.normalize_in_place();
    if root.is_odd() {
        root.negate_in_place(1);
        root.normalize_in_place();
    }
    Some(root)
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_secp256k1_fe_divide(
    a: *const Bytes<32>,
    b: *const Bytes<32>,
    result: *mut Bytes<32>,
) -> i32 {
    commit(result, || {
        let a = FieldElement::from_bytes(&read(a).data)?;
        let b = FieldElement::from_bytes(&read(b).data)?;
        if b.is_zero() {
            return None;
        }
        let quotient = with_secp256k1_hooks!(|hooks| {
            let mut fraction = [a, b];
            let mut quotient = MaybeUninit::uninit();
            *hooks.fe_divide(&mut fraction, &mut quotient)
        });
        Some(fe_bytes(quotient))
    })
}

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use super::*;
    use crypto::hints::{HintError, RecordingHintOracle, Secp256k1Element};

    /// The recorder, with the other root in every square-root answer: the root that an
    /// adversarial tape gives, which passes the hook's check as well (an 8-word answer here is
    /// the canonical words of the root)
    struct OtherRoot(RecordingHintOracle);

    impl HintOracle for OtherRoot {
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            // SAFETY: the caller's contract, unchanged
            unsafe { self.0.send_query(query_id, input_word) }
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.0.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            // SAFETY: the caller's contract, unchanged
            unsafe { self.0.write_words(dst, num_words)? };
            assert_eq!(num_words, 8, "only square roots are asked here");
            // SAFETY: just written, 8 aligned words
            let words = unsafe { &mut *dst.cast::<[u32; 8]>() };
            let mut root = FieldElement::from_canonical_words(words).unwrap();
            root.negate_in_place(1);
            *words = root.to_canonical_words();
            Ok(())
        }

        fn finish_query(&mut self) -> Result<(), HintError> {
            self.0.finish_query()
        }
    }

    fn element(value: u64) -> FieldElement {
        let mut bytes = [0; 32];
        bytes[24..].copy_from_slice(&value.to_be_bytes());
        FieldElement::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn the_root_does_not_depend_on_the_sign_of_the_hint() {
        // 4 (root 2), 36 and 81, whose natural roots x^((p+1)/4) are p - 6 and 9, both odd,
        // a large square, and 0
        let mut big = element(0x1234_5678_9abc_def1);
        big.square_in_place();
        for square in [element(36), element(81)] {
            let mut natural = square;
            assert!(DefaultSecp256k1Hooks.fe_sqrt_and_assign(&mut natural));
            assert_eq!(natural.to_bytes()[31] & 1, 1, "an odd natural root");
        }
        for square in [element(4), element(36), element(81), big, element(0)] {
            let expected = even_sqrt(&mut DefaultSecp256k1Hooks, square).unwrap();
            assert_eq!(expected.to_bytes()[31] & 1, 0);
            let mut check = expected;
            check.square_in_place();
            assert_eq!(check.to_bytes(), square.to_bytes());
            let honest = even_sqrt(
                &mut Secp256k1HooksWithOracle::new(&mut RecordingHintOracle::new()),
                square,
            );
            let lying = even_sqrt(
                &mut Secp256k1HooksWithOracle::new(&mut OtherRoot(RecordingHintOracle::new())),
                square,
            );
            assert_eq!(honest.map(|r| r.to_bytes()), Some(expected.to_bytes()));
            assert_eq!(lying.map(|r| r.to_bytes()), Some(expected.to_bytes()));
            // the hook alone takes the other root: the export's normalization is what pins it
            if !square.is_zero() {
                let (mut honest, mut lying) = (square, square);
                assert!(
                    Secp256k1HooksWithOracle::new(&mut RecordingHintOracle::new())
                        .fe_sqrt_and_assign(&mut honest)
                );
                assert!(
                    Secp256k1HooksWithOracle::new(&mut OtherRoot(RecordingHintOracle::new()))
                        .fe_sqrt_and_assign(&mut lying)
                );
                assert_ne!(honest.to_bytes(), lying.to_bytes());
            }
        }
        // a nonsquare (3): no root, on every path, with the honest or the negated proof
        assert!(even_sqrt(&mut DefaultSecp256k1Hooks, element(3)).is_none());
        assert!(even_sqrt(
            &mut Secp256k1HooksWithOracle::new(&mut RecordingHintOracle::new()),
            element(3)
        )
        .is_none());
        assert!(even_sqrt(
            &mut Secp256k1HooksWithOracle::new(&mut OtherRoot(RecordingHintOracle::new())),
            element(3)
        )
        .is_none());
    }
}
