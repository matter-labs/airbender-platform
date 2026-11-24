//! Known answers of the precompile exports through the C ABI, without and with a recording. The
//! endpoints' edge inputs are tested in airbender-crypto.
mod common;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::{call, encode, point, with_and_without_recording};
use crypto::ark_ec::{AffineRepr, CurveGroup};
use crypto::ark_ff::{BigInteger, PrimeField};
use crypto::{bls12_381 as bls, bn254 as bn};

fn unhex<const N: usize>(text: &str) -> [u8; N] {
    let bytes: Vec<u8> = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect();
    bytes.try_into().unwrap()
}

fn modexp(base: &[u8], exp: &[u8], modulus: &[u8]) -> (Vec<u8>, Vec<u32>) {
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = vec![0xa5; modulus.len()];
        // empty slices have dangling pointers: aligned and not null, as C requires
        let status = unsafe {
            zkvm_modexp(
                base.as_ptr(),
                base.len(),
                exp.as_ptr(),
                exp.len(),
                modulus.as_ptr(),
                modulus.len(),
                output.as_mut_ptr(),
            )
        };
        (status, output)
    });
    assert_eq!(status, EOK);
    (output, tape)
}

#[test]
fn modexp_operands_and_moduli() {
    // empty operands are zero, and 0^0 = 1; an empty modulus has no output, a zero one gives zeros
    assert_eq!(modexp(&[], &[], &[]).0, Vec::<u8>::new());
    assert_eq!(modexp(&[], &[], &[7]).0, [1]);
    assert_eq!(modexp(&[2], &[10], &[0; 4]).0, [0; 4]);
    // 32- and 33-byte moduli: the single-digit and the multi-digit hinted paths
    let mut modulus = [0u8; 32];
    (modulus[0], modulus[31]) = (0x80, 1);
    let (output, tape) = modexp(&[3], &[5], &modulus);
    assert_eq!(output[..31], [0; 31]);
    assert_eq!((output[31], tape.is_empty()), (243, false));
    // (2^128)^3 = 2^256 - 2^128 + 1 modulo 2^256 + 1
    let mut modulus = [0u8; 33];
    (modulus[0], modulus[32]) = (1, 1);
    let mut base = [0u8; 17];
    base[0] = 1;
    let (output, tape) = modexp(&base, &[3], &modulus);
    let mut expected = [0u8; 33];
    expected[1..17].fill(0xff);
    expected[32] = 1;
    assert_eq!((output.as_slice(), tape.is_empty()), (&expected[..], false));
}

#[test]
fn bn254_add_mul_and_pairing() {
    let g = bn::G1Affine::generator();
    let g2 = (g + g).into_affine();
    let ((status, sum), tape) = call(Bytes { data: [0xa5; 64] }, |out| unsafe {
        zkvm_bn254_g1_add(&point(&g), &point(&g2), out)
    });
    assert_eq!((status, sum), (EOK, point(&(g + g2).into_affine())));
    assert!(!tape.is_empty());
    // scalars are 256-bit integers, reduced modulo the group order
    for scalar in [[0x5a; 32], [0xff; 32]] {
        let ((status, product), tape) = call(Bytes { data: [0xa5; 64] }, |out| unsafe {
            zkvm_bn254_g1_mul(&point(&g), &Bytes { data: scalar }, out)
        });
        let expected = (g * bn::Fr::from_be_bytes_mod_order(&scalar)).into_affine();
        assert_eq!((status, product), (EOK, point(&expected)));
        assert!(!tape.is_empty());
    }

    // EIP-197: the imaginary parts of G2 first
    let q = bn::G2Affine::generator();
    let pair = |p: &bn::G1Affine| Pair {
        first: point(p),
        second: encode(&[q.x.c1, q.x.c0, q.y.c1, q.y.c0]),
    };
    let pairing = |pairs: &[Pair<64, 128>]| {
        call(false, |out| unsafe {
            zkvm_bn254_pairing(pairs.as_ptr(), pairs.len(), out)
        })
    };
    let pairs = [pair(&g), pair(&-g)];
    let ((status, verified), tape) = pairing(&pairs);
    assert_eq!((status, verified, tape.is_empty()), (EOK, true, false));
    assert_eq!(pairing(&pairs[..1]).0, (EOK, false));
    assert_eq!(pairing(&[]).0, (EOK, true));
}

#[test]
fn bls12_381_add_msm_and_pairing() {
    let g = bls::G1Affine::generator();
    let q = bls::G2Affine::generator();
    let g1 = |k: u64| (g * bls::Fr::from(k)).into_affine();
    let g2 = |k: u64| (q * bls::Fr::from(k)).into_affine();
    let ((status, sum), tape) = call(Bytes { data: [0xa5; 96] }, |out| unsafe {
        zkvm_bls12_g1_add(&point(&g1(1)), &point(&g1(2)), out)
    });
    assert_eq!((status, sum, tape.is_empty()), (EOK, point(&g1(3)), false));
    let ((status, sum), tape) = call(Bytes { data: [0xa5; 192] }, |out| unsafe {
        zkvm_bls12_g2_add(&point(&g2(1)), &point(&g2(2)), out)
    });
    assert_eq!((status, sum, tape.is_empty()), (EOK, point(&g2(3)), false));

    // MSMs: two terms take the affine path, four Pippenger's
    let scalar = |i: u64| [i as u8 + 1; 32];
    let times = |i: u64| bls::Fr::from_be_bytes_mod_order(&scalar(i));
    for n in [2, 4] {
        let terms: Vec<_> = (0..n)
            .map(|i| Pair {
                first: point(&g1(i + 2)),
                second: Bytes { data: scalar(i) },
            })
            .collect();
        let expected: bls::G1Projective = (0..n).map(|i| g1(i + 2) * times(i)).sum();
        let ((status, sum), tape) = call(Bytes { data: [0xa5; 96] }, |out| unsafe {
            zkvm_bls12_g1_msm(terms.as_ptr(), terms.len(), out)
        });
        assert_eq!((status, sum), (EOK, point(&expected.into_affine())));
        assert!(!tape.is_empty());
    }
    let terms: Vec<_> = (0..2)
        .map(|i| Pair {
            first: point(&g2(i + 2)),
            second: Bytes { data: scalar(i) },
        })
        .collect();
    let expected: bls::G2Projective = (0..2).map(|i| g2(i + 2) * times(i)).sum();
    let ((status, sum), tape) = call(Bytes { data: [0xa5; 192] }, |out| unsafe {
        zkvm_bls12_g2_msm(terms.as_ptr(), terms.len(), out)
    });
    assert_eq!((status, sum), (EOK, point(&expected.into_affine())));
    assert!(!tape.is_empty());

    let pairs = [g, -g].map(|p| Pair {
        first: point(&p),
        second: point(&q),
    });
    let pairing = |pairs: &[Pair<96, 192>]| {
        call(false, |out| unsafe {
            zkvm_bls12_pairing(pairs.as_ptr(), pairs.len(), out)
        })
    };
    let ((status, verified), tape) = pairing(&pairs);
    assert_eq!((status, verified, tape.is_empty()), (EOK, true, false));
    assert_eq!(pairing(&pairs[..1]).0, (EOK, false));
}

#[test]
fn bls12_381_maps() {
    let one = bls::Fq::from(1u64);
    for value in [
        bls::Fq::from(0u64),
        bls::Fq::from_be_bytes_mod_order(&[0x5a; 48]),
    ] {
        let ((status, p), tape) = call(Bytes { data: [0xa5; 96] }, |out| unsafe {
            zkvm_bls12_map_fp_to_g1(&encode(&[value]), out)
        });
        let expected = bls::eip2537::map_fp_to_g1(value).unwrap().clear_cofactor();
        assert_eq!((status, p, tape.is_empty()), (EOK, point(&expected), false));
        let fp2 = bls::Fq2::new(value, one);
        let ((status, p), tape) = call(Bytes { data: [0xa5; 192] }, |out| unsafe {
            zkvm_bls12_map_fp2_to_g2(&encode(&[fp2]), out)
        });
        let expected = bls::eip2537::map_fp2_to_g2(fp2).unwrap().clear_cofactor();
        assert_eq!((status, p, tape.is_empty()), (EOK, point(&expected), false));
    }
    // the modulus is not canonical: rejected before any hint
    let modulus = bls::Fq::MODULUS.to_bytes_be();
    let p = Bytes {
        data: modulus[modulus.len() - 48..].try_into().unwrap(),
    };
    let unchanged = Bytes { data: [0xa5; 96] };
    let outcome = call(unchanged, |out| unsafe { zkvm_bls12_map_fp_to_g1(&p, out) });
    assert_eq!(outcome, ((EFAIL, unchanged), vec![]));
}

#[test]
fn kzg_point_evaluation() {
    // c-kzg-4844 verify_kzg_proof_case_correct_proof_4_4
    let commitment = unhex("8f59a8d2a1a625a17f3fea0fe5eb8c896db3764f3185481bc22f91b4aaffcca25f26936857bc3a7c2539ea8ec3a952b7");
    let z = unhex("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000000");
    let y = unhex("1522a4a7f34e1ea350ae07c29c96c7e79655aa926122e95fe69fcbd932ca49e9");
    let proof = unhex("a62ad71d14c5719385c0686f1871430475bf3a00f0aa3f7b8dd99a9abc2160744faf0070725e00b60ad9a026a15b1a8c");
    let kzg = |commitment: [u8; 48], z: [u8; 32], y: [u8; 32]| {
        call(false, |out| unsafe {
            zkvm_kzg_point_eval(
                &Bytes { data: commitment },
                &Bytes { data: z },
                &Bytes { data: y },
                &Bytes { data: proof },
                out,
            )
        })
    };
    let ((status, verified), tape) = kzg(commitment, z, y);
    assert_eq!((status, verified, tape.is_empty()), (EOK, true, false));
    let mut wrong = y;
    wrong[31] ^= 1;
    assert_eq!(kzg(commitment, z, wrong).0, (EOK, false));
    // z not below the order, and a commitment that is not a compressed point
    let order = bls::Fr::MODULUS.to_bytes_be().try_into().unwrap();
    assert_eq!(kzg(commitment, order, y).0, (EFAIL, false));
    assert_eq!(kzg([0xff; 48], z, y), ((EFAIL, false), vec![]));
}
