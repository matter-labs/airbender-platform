//! The C encodings and error contract of the curve exports, and the subgroup, cofactor and
//! normalization bindings with the tapes of the Rust hinted operations they stand for.
mod common;

use airbender_crypto::ark_ec::short_weierstrass::{Affine, SWCurveConfig};
use airbender_crypto::ark_ec::{AffineRepr, CurveGroup};
use airbender_crypto::ark_ff::{AdditiveGroup, Field};
use airbender_crypto::bls12_381::{self as bls, Fq, Fq2};
use airbender_crypto::bn254 as bn;
use airbender_crypto::hints::{self, record_hints, Bls12381Fq2Divider, Bls12381FqDivider};
use airbender_crypto_c::*;
use common::{call, encode, point};

#[test]
fn bn_big_endian_and_in_place_output() {
    let mut point = Bytes { data: [0; 64] };
    point.data[31] = 1;
    point.data[63] = 2;
    let expected = point;
    unsafe {
        assert_eq!(
            zkvm_bn254_g1_add(&point, &Bytes { data: [0; 64] }, &mut point),
            EOK
        );
        assert_eq!(point, expected);
        let malformed = Bytes { data: [0xff; 64] };
        assert_eq!(zkvm_bn254_g1_add(&point, &malformed, &mut point), EFAIL);
        assert_eq!(point, expected);
    }
}

#[test]
fn bls_compact_sizes_and_fp2_coefficient_order() {
    let mut fp2 = Bytes { data: [0; 96] };
    fp2.data[47] = 1; // real component
    fp2.data[95] = 2; // imaginary component
    let mut output = Bytes { data: [0; 192] };
    unsafe {
        assert_eq!(zkvm_bls12_map_fp2_to_g2(&fp2, &mut output), EOK);
    }
    let expected = bls::eip2537::map_fp2_to_g2(Fq2::new(1.into(), 2.into()))
        .unwrap()
        .clear_cofactor();
    assert_eq!(output, point(&expected));
    unsafe {
        assert_eq!(
            zkvm_bls12_map_fp2_to_g2(&Bytes { data: [0xff; 96] }, &mut output),
            EFAIL
        );
    }
    assert_eq!(output, point(&expected));
}

#[test]
fn empty_arrays_and_late_failure_preserve_output() {
    let bn = [Pair {
        first: Bytes { data: [0; 64] },
        second: Bytes { data: [0; 128] },
    }; 5];
    let mut bls = [Pair {
        first: Bytes { data: [0; 96] },
        second: Bytes { data: [0; 192] },
    }; 5];
    let mut verified = false;
    unsafe {
        assert_eq!(zkvm_bn254_pairing(bn.as_ptr(), 0, &mut verified), EOK);
        assert!(verified);
        assert_eq!(zkvm_bls12_pairing(bls.as_ptr(), 0, &mut verified), EFAIL);
        assert!(verified);
        bls[4].first.data.fill(0xff);
        assert_eq!(zkvm_bls12_pairing(bls.as_ptr(), 5, &mut verified), EFAIL);
        assert!(verified);
        let pair = Pair {
            first: Bytes { data: [0; 96] },
            second: Bytes { data: [0; 32] },
        };
        let mut output = Bytes { data: [0xa5; 96] };
        assert_eq!(zkvm_bls12_g1_msm(&pair, 0, &mut output), EFAIL);
        assert_eq!(output.data, [0xa5; 96]);
    }
}

#[test]
fn canonical_off_curve_coordinates_are_rejected_before_operations() {
    macro_rules! check_add {
        ($size:literal, $add:ident) => {{
            let mut invalid = Bytes { data: [0; $size] };
            invalid.data[$size / 2 - 1] = 1;
            invalid.data[$size - 1] = 1;
            let mut output = Bytes {
                data: [0xa5; $size],
            };
            assert_eq!(
                unsafe { $add(&invalid, &Bytes { data: [0; $size] }, &mut output) },
                EFAIL
            );
            assert_eq!(output.data, [0xa5; $size]);
        }};
    }
    check_add!(64, zkvm_bn254_g1_add);
    check_add!(96, zkvm_bls12_g1_add);
    check_add!(192, zkvm_bls12_g2_add);
    // An infinity partner must not hide an off-curve G2 input.
    let mut pair = Pair {
        first: Bytes { data: [0; 64] },
        second: Bytes { data: [0; 128] },
    };
    pair.second.data[63] = 1;
    pair.second.data[127] = 1;
    let mut verified = true;
    assert_eq!(
        unsafe { zkvm_bn254_pairing(&pair, 1, &mut verified) },
        EFAIL
    );
    assert!(verified);
}

/// A point of the curve outside its prime-order subgroup: the first `x = from_u64(k)` with one
fn outside_subgroup<C: SWCurveConfig>(from_u64: impl Fn(u64) -> C::BaseField) -> Affine<C> {
    (1..)
        .find_map(|k| {
            let x = from_u64(k);
            let point = Affine::<C>::new_unchecked(x, (x.square() * x + C::COEFF_B).sqrt()?);
            (!point.is_in_correct_subgroup_assuming_on_curve()).then_some(point)
        })
        .unwrap()
}

#[test]
fn subgroup_and_cofactor() {
    let outside1 = outside_subgroup::<bls::g1::Config>(Fq::from);
    let outside2 = outside_subgroup::<bls::g2::Config>(|k| Fq2::new(k.into(), Fq::ZERO));
    // infinity is a member without a hint
    for (p, member) in [
        (bls::G1Affine::generator(), true),
        (outside1, false),
        (bls::G1Affine::zero(), true),
    ] {
        let ((status, result), tape) = call(!member, |out| unsafe {
            airbender_bls12_g1_is_in_subgroup(&point(&p), out)
        });
        let hinted = record_hints(|o| {
            p.is_zero() || bls::g1::is_in_subgroup_with_divider(&p, &mut Bls12381FqDivider::new(o))
        });
        assert_eq!((status, result, tape), (EOK, member, hinted.1));
    }
    for (p, member) in [(bls::G2Affine::generator(), true), (outside2, false)] {
        let ((status, result), tape) = call(!member, |out| unsafe {
            airbender_bls12_g2_is_in_subgroup(&point(&p), out)
        });
        let (_, hinted) = record_hints(|o| {
            bls::g2::is_in_subgroup_with_divider(&p, &mut Bls12381Fq2Divider::new(o))
        });
        assert_eq!((status, result, tape), (EOK, member, hinted));
    }

    // clearing does not assume the subgroup: G1's effective cofactor 1 - x, unreduced
    let cleared = outside1
        .mul_bigint([0xd201_0000_0001_0001u64])
        .into_affine();
    let ((status, output), tape) = call(Bytes { data: [0; 96] }, |out| unsafe {
        airbender_bls12_g1_clear_cofactor(&point(&outside1), out)
    });
    let (_, hinted) = record_hints(|o| {
        bls::g1::clear_cofactor_with_divider(&outside1, &mut Bls12381FqDivider::new(o))
    });
    assert_eq!((status, output, tape), (EOK, point(&cleared), hinted));
    let cleared = outside2.clear_cofactor();
    assert!(cleared.is_in_correct_subgroup_assuming_on_curve());
    let ((status, output), tape) = call(Bytes { data: [0; 192] }, |out| unsafe {
        airbender_bls12_g2_clear_cofactor(&point(&outside2), out)
    });
    let (_, hinted) = record_hints(|o| {
        bls::g2::clear_cofactor_with_divider(&outside2, &mut Bls12381Fq2Divider::new(o))
    });
    assert_eq!((status, output, tape), (EOK, point(&cleared), hinted));
    let outcome = call(Bytes { data: [0xa5; 192] }, |out| unsafe {
        airbender_bls12_g2_clear_cofactor(&Bytes { data: [0; 192] }, out)
    });
    assert_eq!(outcome, ((EOK, Bytes { data: [0; 192] }), vec![]));
}

/// Jacobian `X || Y || Z`: one inverse hint of Z; Z = 0 is infinity, whatever X and Y
#[test]
fn jacobian_normalization() {
    let g = bls::G1Affine::generator();
    let z = Fq::from(3u64);
    let ((status, output), tape) = call(Bytes { data: [0; 96] }, |out| unsafe {
        airbender_bls12_g1_to_affine(&encode(&[g.x * z.square(), g.y * z.square() * z, z]), out)
    });
    assert_eq!((status, output), (EOK, point(&g)));
    assert_eq!(tape, record_hints(|o| hints::bls12_381_fq_inverse(o, &z)).1);
    let to_affine = |coordinates: [Fq; 3]| {
        call(Bytes { data: [0xa5; 96] }, |out| unsafe {
            airbender_bls12_g1_to_affine(&encode(&coordinates), out)
        })
    };
    let infinity = to_affine([g.x, g.y, Fq::ZERO]);
    assert_eq!(infinity, ((EOK, Bytes { data: [0; 96] }), vec![]));
    let off_curve = to_affine([g.x, g.x, Fq::ONE]);
    assert_eq!(off_curve.0, (EFAIL, Bytes { data: [0xa5; 96] }));

    let q = bls::G2Affine::generator();
    let z = Fq2::new(Fq::from(3u64), Fq::from(5u64));
    let ((status, output), tape) = call(Bytes { data: [0; 192] }, |out| unsafe {
        airbender_bls12_g2_to_affine(&encode(&[q.x * z.square(), q.y * z.square() * z, z]), out)
    });
    assert_eq!((status, output), (EOK, point(&q)));
    assert_eq!(
        tape,
        record_hints(|o| hints::bls12_381_fq2_inverse(o, &z)).1
    );

    let g = bn::G1Affine::generator();
    let z = bn::Fq::from(7u64);
    let ((status, output), tape) = call(Bytes { data: [0; 64] }, |out| unsafe {
        airbender_bn254_g1_to_affine(&encode(&[g.x * z.square(), g.y * z.square() * z, z]), out)
    });
    assert_eq!((status, output), (EOK, point(&g)));
    assert_eq!(tape, record_hints(|o| hints::bn254_fq_inverse(o, &z)).1);
}
