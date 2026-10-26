//! Check the C encodings and error contract; group arithmetic is tested in crypto.
mod common;

use airbender_crypto_c::*;

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
    use airbender_crypto::{ark_ec::AffineRepr, ark_ff::PrimeField, bls12_381 as bls};
    let mut fp2 = Bytes { data: [0; 96] };
    fp2.data[47] = 1; // real component
    fp2.data[95] = 2; // imaginary component
    let mut output = Bytes { data: [0; 192] };
    unsafe {
        assert_eq!(zkvm_bls12_map_fp2_to_g2(&fp2, &mut output), EOK);
    }
    // Compare encoding to the existing Rust API, without implementing curve math here.
    let expected = bls::eip2537::map_fp2_to_g2(bls::Fq2::new(1.into(), 2.into()))
        .unwrap()
        .clear_cofactor();
    for (bytes, field) in output.data.as_chunks::<48>().0.iter().zip([
        expected.x.c0,
        expected.x.c1,
        expected.y.c0,
        expected.y.c1,
    ]) {
        assert_eq!(bls::Fq::from_be_bytes_mod_order(bytes), field);
    }
    let saved = output;
    unsafe {
        assert_eq!(
            zkvm_bls12_map_fp2_to_g2(&Bytes { data: [0xff; 96] }, &mut output),
            EFAIL
        );
    }
    assert_eq!(output, saved);
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

/// The compact encoding of the coordinates `values` (canonical big-endian)
fn compact<const N: usize, F: airbender_crypto::ark_ff::PrimeField>(values: &[F]) -> Bytes<N> {
    use airbender_crypto::ark_ff::BigInteger;
    let mut output = Bytes { data: [0; N] };
    let width = N / values.len();
    for (dst, value) in output.data.chunks_exact_mut(width).zip(values) {
        let bytes = value.into_bigint().to_bytes_be();
        dst.copy_from_slice(&bytes[bytes.len() - width..]);
    }
    output
}

/// Subgroup checks, cofactor clearing and Jacobian normalization: right without and with a
/// recording, with the tapes of the hinted Rust operations they stand for
#[test]
fn subgroup_cofactor_and_normalization() {
    use airbender_crypto::ark_ec::{AffineRepr, CurveGroup};
    use airbender_crypto::ark_ff::Field;
    use airbender_crypto::bls12_381::{self as bls, Fq, Fq2};
    use airbender_crypto::hints::{self, record_hints, Bls12381Fq2Divider, Bls12381FqDivider};
    use common::with_and_without_recording;
    let g1 = |p: &bls::G1Affine| match p.xy() {
        Some((x, y)) => compact::<96, Fq>(&[x, y]),
        None => Bytes { data: [0; 96] },
    };
    let g2 = |p: &bls::G2Affine| match p.xy() {
        Some((x, y)) => compact::<192, Fq>(&[x.c0, x.c1, y.c0, y.c1]),
        None => Bytes { data: [0; 192] },
    };
    // (4, y): on the curve, outside the subgroup
    let x = Fq::from(4u64);
    let outside =
        bls::G1Affine::new_unchecked(x, (x.square() * x + Fq::from(4u64)).sqrt().unwrap());
    assert!(!outside.is_in_correct_subgroup_assuming_on_curve());
    let generator = bls::G1Affine::generator();
    for (point, expected) in [
        (generator, true),
        (outside, false),
        (bls::G1Affine::zero(), true),
    ] {
        let ((status, member), tape) = with_and_without_recording(|| {
            let mut member = !expected;
            let status = unsafe { airbender_bls12_g1_is_in_subgroup(&g1(&point), &mut member) };
            (status, member)
        });
        assert_eq!((status, member), (EOK, expected));
        if !point.is_zero() {
            let (_, hinted) = record_hints(|o| {
                bls::g1::is_in_subgroup_with_divider(&point, &mut Bls12381FqDivider::new(o))
            });
            assert_eq!(tape, hinted);
        }
    }
    // the effective cofactor 1 - x of G1, by double-and-add, without any reduction
    let cleared = outside.mul_bigint([0xd201_0000_0001_0001u64]).into_affine();
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = Bytes { data: [0; 96] };
        let status = unsafe { airbender_bls12_g1_clear_cofactor(&g1(&outside), &mut output) };
        (status, output)
    });
    assert_eq!((status, output), (EOK, g1(&cleared)));
    let (_, hinted) = record_hints(|o| {
        bls::g1::clear_cofactor_with_divider(&outside, &mut Bls12381FqDivider::new(o))
    });
    assert_eq!(tape, hinted);

    let q = bls::G2Affine::generator();
    let x = Fq2::new(Fq::from(1u64), Fq::from(0u64));
    let b =
        <bls::g2::Config as airbender_crypto::ark_ec::short_weierstrass::SWCurveConfig>::COEFF_B;
    let outside = (1u64..)
        .find_map(|k| {
            let x = x + Fq2::new(Fq::from(k), Fq::from(0u64));
            let p = bls::G2Affine::new_unchecked(x, (x.square() * x + b).sqrt()?);
            (!p.is_in_correct_subgroup_assuming_on_curve()).then_some(p)
        })
        .unwrap();
    for (point, expected) in [(q, true), (outside, false)] {
        let ((status, member), tape) = with_and_without_recording(|| {
            let mut member = !expected;
            let status = unsafe { airbender_bls12_g2_is_in_subgroup(&g2(&point), &mut member) };
            (status, member)
        });
        assert_eq!((status, member), (EOK, expected));
        let (_, hinted) = record_hints(|o| {
            bls::g2::is_in_subgroup_with_divider(&point, &mut Bls12381Fq2Divider::new(o))
        });
        assert_eq!(tape, hinted);
    }
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = Bytes { data: [0; 192] };
        let status = unsafe { airbender_bls12_g2_clear_cofactor(&g2(&outside), &mut output) };
        (status, output)
    });
    let cleared = outside.clear_cofactor();
    assert!(cleared.is_in_correct_subgroup_assuming_on_curve());
    assert_eq!((status, output), (EOK, g2(&cleared)));
    let (_, hinted) = record_hints(|o| {
        bls::g2::clear_cofactor_with_divider(&outside, &mut Bls12381Fq2Divider::new(o))
    });
    assert_eq!(tape, hinted);
    // infinity is its own cofactor-cleared point, without hints
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = Bytes { data: [0xa5; 192] };
        let status =
            unsafe { airbender_bls12_g2_clear_cofactor(&Bytes { data: [0; 192] }, &mut output) };
        (status, output)
    });
    assert_eq!((status, output.data, tape), (EOK, [0; 192], vec![]));

    // Jacobian X || Y || Z with Z = 3, Z = 1 and Z = 0, and off the curve
    let z = Fq::from(3u64);
    let (gx, gy) = generator.xy().unwrap();
    let jacobian = compact::<144, Fq>(&[gx * z.square(), gy * z.square() * z, z]);
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = Bytes { data: [0; 96] };
        let status = unsafe { airbender_bls12_g1_to_affine(&jacobian, &mut output) };
        (status, output)
    });
    assert_eq!((status, output), (EOK, g1(&generator)));
    assert_eq!(tape, record_hints(|o| hints::bls12_381_fq_inverse(o, &z)).1);
    for (coordinates, expected) in [
        ([gx, gy, Fq::from(1u64)], Some(g1(&generator))),
        ([gx, gy, Fq::from(0u64)], Some(Bytes { data: [0; 96] })),
        ([gx, gx, Fq::from(1u64)], None),
    ] {
        let ((status, output), tape) = with_and_without_recording(|| {
            let mut output = Bytes { data: [0xa5; 96] };
            let status = unsafe {
                airbender_bls12_g1_to_affine(&compact::<144, Fq>(&coordinates), &mut output)
            };
            (status, output)
        });
        match expected {
            Some(point) => assert_eq!((status, output, tape), (EOK, point, vec![])),
            None => assert_eq!((status, output.data), (EFAIL, [0xa5; 96])),
        }
    }
    let z2 = Fq2::new(Fq::from(3u64), Fq::from(5u64));
    let (qx, qy) = q.xy().unwrap();
    let (x, y) = (qx * z2.square(), qy * z2.square() * z2);
    let jacobian = compact::<288, Fq>(&[x.c0, x.c1, y.c0, y.c1, z2.c0, z2.c1]);
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = Bytes { data: [0; 192] };
        let status = unsafe { airbender_bls12_g2_to_affine(&jacobian, &mut output) };
        (status, output)
    });
    assert_eq!((status, output), (EOK, g2(&q)));
    assert_eq!(
        tape,
        record_hints(|o| hints::bls12_381_fq2_inverse(o, &z2)).1
    );

    use airbender_crypto::bn254 as bn;
    let z = bn::Fq::from(7u64);
    let jacobian = compact::<96, bn::Fq>(&[
        bn::Fq::from(1u64) * z.square(),
        bn::Fq::from(2u64) * z.square() * z,
        z,
    ]);
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = Bytes { data: [0; 64] };
        let status = unsafe { airbender_bn254_g1_to_affine(&jacobian, &mut output) };
        (status, output)
    });
    let mut g = Bytes { data: [0; 64] };
    g.data[31] = 1;
    g.data[63] = 2;
    assert_eq!((status, output), (EOK, g));
    assert_eq!(tape, record_hints(|o| hints::bn254_fq_inverse(o, &z)).1);
}
