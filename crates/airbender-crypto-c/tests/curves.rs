//! The C encodings and error contract of the curve exports.
mod common;

use airbender_crypto::ark_ec::AffineRepr;
use airbender_crypto::bls12_381::{self as bls, Fq2};
use airbender_crypto_c::*;
use common::point;

/// One pair over the gas-cap bound fails on `num_pairs` alone: the input, here a single pair
/// (so that reading `num_pairs` of them would be out of bounds), is not read, and the output
/// stays. The most pairs, all at infinity, pass.
#[test]
fn too_many_pairs_fail_on_the_count_alone() {
    use airbender_crypto::precompiles::{
        MAX_BLS12_381_G1_MSM_PAIRS, MAX_BLS12_381_G2_MSM_PAIRS, MAX_BLS12_381_PAIRS,
        MAX_BN254_PAIRS,
    };
    let bn_pair = Pair {
        first: Bytes { data: [0; 64] },
        second: Bytes { data: [0; 128] },
    };
    let bls_pair = Pair {
        first: Bytes { data: [0; 96] },
        second: Bytes { data: [0; 192] },
    };
    let g1_pair = Pair {
        first: Bytes { data: [0; 96] },
        second: Bytes { data: [0; 32] },
    };
    let g2_pair = Pair {
        first: Bytes { data: [0; 192] },
        second: Bytes { data: [0; 32] },
    };
    let mut verified = true;
    let mut out96 = Bytes { data: [0xa5; 96] };
    let mut out192 = Bytes { data: [0xa5; 192] };
    unsafe {
        assert_eq!(
            zkvm_bn254_pairing(&bn_pair, MAX_BN254_PAIRS + 1, &mut verified),
            EFAIL
        );
        assert_eq!(
            zkvm_bls12_pairing(&bls_pair, MAX_BLS12_381_PAIRS + 1, &mut verified),
            EFAIL
        );
        assert!(verified);
        assert_eq!(
            zkvm_bls12_g1_msm(&g1_pair, MAX_BLS12_381_G1_MSM_PAIRS + 1, &mut out96),
            EFAIL
        );
        assert_eq!(out96.data, [0xa5; 96]);
        assert_eq!(
            zkvm_bls12_g2_msm(&g2_pair, MAX_BLS12_381_G2_MSM_PAIRS + 1, &mut out192),
            EFAIL
        );
        assert_eq!(out192.data, [0xa5; 192]);

        let bn = vec![bn_pair; MAX_BN254_PAIRS];
        assert_eq!(
            zkvm_bn254_pairing(bn.as_ptr(), bn.len(), &mut verified),
            EOK
        );
        assert!(verified);
        let bls = vec![bls_pair; MAX_BLS12_381_PAIRS];
        assert_eq!(
            zkvm_bls12_pairing(bls.as_ptr(), bls.len(), &mut verified),
            EOK
        );
        assert!(verified);
        let g1 = vec![g1_pair; MAX_BLS12_381_G1_MSM_PAIRS];
        assert_eq!(zkvm_bls12_g1_msm(g1.as_ptr(), g1.len(), &mut out96), EOK);
        assert_eq!(out96.data, [0; 96]);
        let g2 = vec![g2_pair; MAX_BLS12_381_G2_MSM_PAIRS];
        assert_eq!(zkvm_bls12_g2_msm(g2.as_ptr(), g2.len(), &mut out192), EOK);
        assert_eq!(out192.data, [0; 192]);
    }
}

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
