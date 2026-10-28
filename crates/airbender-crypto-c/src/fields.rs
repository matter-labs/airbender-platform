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

/// The inverse in bn254 `Fq2 = Fq[u] / (u² + 1)` from one checked inverse hint of the norm
/// `c0² + c1²`, which is non-zero for a non-zero element as -1 is not a square in Fq.
#[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
fn bn254_fq2_inverse<O: HintOracle>(oracle: &mut O, value: &bn::Fq2) -> Option<bn::Fq2> {
    let norm = value.c0.square() + value.c1.square();
    let inverse = hints::bn254_fq_inverse(oracle, &norm)?;
    Some(bn::Fq2::new(value.c0 * inverse, -(value.c1 * inverse)))
}

/// The inverse and quotient bindings of one field: hinted with an oracle, plain without
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

/// `$body` with `Secp256k1HooksWithOracle` when there is an oracle, the default hooks otherwise
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

/// The even square root of `a`, `None` for a nonsquare. A hinted root is only checked to square to
/// `a`, so its sign is the prover's: normalizing it here keeps the result independent of the hint.
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

    /// The recorder, with the other root in every square-root answer (8 canonical words), which
    /// passes the hook's check as well
    struct OtherRoot(RecordingHintOracle);

    impl HintOracle for OtherRoot {
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            // SAFETY: the caller's contract
            unsafe { self.0.send_query(query_id, input_word) }
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.0.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            // SAFETY: the caller's contract
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
        let honest = RecordingHintOracle::new;
        let lying = || OtherRoot(RecordingHintOracle::new());
        // 4: natural root 2, even; 81: natural root 9, odd
        for square in [element(4), element(81)] {
            let expected = even_sqrt(&mut DefaultSecp256k1Hooks, square).unwrap();
            assert!(!expected.is_odd());
            let mut check = expected;
            check.square_in_place();
            assert_eq!(check.to_bytes(), square.to_bytes());
            for root in [
                even_sqrt(&mut Secp256k1HooksWithOracle::new(&mut honest()), square),
                even_sqrt(&mut Secp256k1HooksWithOracle::new(&mut lying()), square),
            ] {
                assert_eq!(root.unwrap().to_bytes(), expected.to_bytes());
            }
            // the lying oracle does give the other root
            let (mut a, mut b) = (square, square);
            assert!(Secp256k1HooksWithOracle::new(&mut honest()).fe_sqrt_and_assign(&mut a));
            assert!(Secp256k1HooksWithOracle::new(&mut lying()).fe_sqrt_and_assign(&mut b));
            assert_ne!(a.to_bytes(), b.to_bytes());
        }
        // a nonsquare (3) has no root, on every path
        assert!(even_sqrt(&mut DefaultSecp256k1Hooks, element(3)).is_none());
        assert!(even_sqrt(
            &mut Secp256k1HooksWithOracle::new(&mut honest()),
            element(3)
        )
        .is_none());
        assert!(even_sqrt(&mut Secp256k1HooksWithOracle::new(&mut lying()), element(3)).is_none());
    }
}
