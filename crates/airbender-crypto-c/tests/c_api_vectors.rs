//! The vectors and edge inputs of the precompile endpoints through the C
//! exports, natively, without and with a recording: infinity, points off
//! the curve or outside the subgroup, scalars of at least the group order, empty MSM and pairing
//! inputs, ModExp with empty operands or a zero modulus. The C encodings (compact BLS
//! coordinates, separate recovery IDs and ModExp operands) reach the endpoints unchanged, a
//! recording changes no result, and inputs rejected while they are parsed record nothing.
//! Expected values are the zksync-os vectors, or independent arkworks and k256 computations.
mod common;
// zksync-os's 62 ModExp vectors, `(name, input, expected)`, as airbender-crypto's tests take them
#[path = "../../airbender-crypto/src/modexp/test_vectors.rs"]
mod modexp_vectors;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::{recorded, with_and_without_recording};
use crypto::ark_ec::short_weierstrass::{Affine, SWCurveConfig};
use crypto::ark_ec::{AffineRepr, CurveGroup};
use crypto::ark_ff::{BigInteger, Field, PrimeField};
use crypto::{bls12_381 as bls, bn254 as bn, MiniDigest};

/// The bytes of a hex string, at compile time (the crate has no hex dependency)
const fn hex<const N: usize>(text: &str) -> [u8; N] {
    const fn nibble(digit: u8) -> u8 {
        match digit {
            b'0'..=b'9' => digit - b'0',
            b'a'..=b'f' => digit - b'a' + 10,
            _ => panic!("not a hex digit"),
        }
    }
    let text = text.as_bytes();
    assert!(text.len() == 2 * N);
    let mut bytes = [0; N];
    let mut i = 0;
    while i < N {
        bytes[i] = nibble(text[2 * i]) << 4 | nibble(text[2 * i + 1]);
        i += 1;
    }
    bytes
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn keccak(data: &[u8]) -> [u8; 32] {
    <crypto::sha3::Keccak256 as MiniDigest>::digest(data).0
}

/// The canonical big-endian bytes of `value` into `out` (the delegated big integers of
/// bls12-381, if unified in, are wider: the value is in their low bytes)
fn put<F: PrimeField>(value: F, out: &mut [u8]) {
    let bytes = value.into_bigint().to_bytes_be();
    out.copy_from_slice(&bytes[bytes.len() - out.len()..]);
}

fn bn_g1(p: &bn::G1Affine) -> Bytes<64> {
    let mut out = Bytes { data: [0; 64] };
    if let Some((x, y)) = p.xy() {
        put(x, &mut out.data[..32]);
        put(y, &mut out.data[32..]);
    }
    out
}

/// EIP-197: the imaginary parts first
fn bn_g2(p: &bn::G2Affine) -> Bytes<128> {
    let mut out = Bytes { data: [0; 128] };
    if let Some((x, y)) = p.xy() {
        for (chunk, value) in out
            .data
            .as_chunks_mut::<32>()
            .0
            .iter_mut()
            .zip([x.c1, x.c0, y.c1, y.c0])
        {
            put(value, chunk);
        }
    }
    out
}

fn bls_g1(p: &bls::G1Affine) -> Bytes<96> {
    let mut out = Bytes { data: [0; 96] };
    if let Some((x, y)) = p.xy() {
        put(x, &mut out.data[..48]);
        put(y, &mut out.data[48..]);
    }
    out
}

/// EIP-2537: the real parts first
fn bls_g2(p: &bls::G2Affine) -> Bytes<192> {
    let mut out = Bytes { data: [0; 192] };
    if let Some((x, y)) = p.xy() {
        for (chunk, value) in out
            .data
            .as_chunks_mut::<48>()
            .0
            .iter_mut()
            .zip([x.c0, x.c1, y.c0, y.c1])
        {
            put(value, chunk);
        }
    }
    out
}

/// A point of the curve of `C` outside its prime-order subgroup: the first `x = k` (or
/// `(k, 0)`) with a point
fn outside_subgroup<C: SWCurveConfig>(from_u64: impl Fn(u64) -> C::BaseField) -> Affine<C> {
    (1..)
        .find_map(|k| {
            let x = from_u64(k);
            let y = (x.square() * x + C::COEFF_B).sqrt()?;
            let point = Affine::<C>::new_unchecked(x, y);
            assert!(point.is_on_curve());
            (!point.is_in_correct_subgroup_assuming_on_curve()).then_some(point)
        })
        .unwrap()
}

/// The order of the group, plus one, as big-endian bytes
fn order_plus_one<F: PrimeField>() -> [u8; 32] {
    let mut bytes: [u8; 32] = F::MODULUS.to_bytes_be().try_into().unwrap();
    bytes[31] += 1;
    bytes
}

fn order<F: PrimeField>() -> [u8; 32] {
    F::MODULUS.to_bytes_be().try_into().unwrap()
}

/// Scalars of the multiplications: zero, one, the order (zero again), the order plus one (one
/// again), and the largest 256-bit integer
fn scalars<F: PrimeField>() -> [[u8; 32]; 5] {
    let mut one = [0; 32];
    one[31] = 1;
    [
        [0; 32],
        one,
        order::<F>(),
        order_plus_one::<F>(),
        [0xff; 32],
    ]
}

fn call<T: Copy>(initial: T, f: impl FnOnce(&mut T) -> i32) -> (i32, T) {
    let mut output = initial;
    let status = f(&mut output);
    (status, output)
}

#[test]
fn modexp_vectors_of_zksync_os() {
    for (name, input, expected) in modexp_vectors::MODEXP_VECTORS {
        let input = unhex(input);
        let expected = unhex(expected);
        // the precompile's layout: three 32-byte big-endian lengths, then the operands, padded
        // with zeroes when the input is shorter
        let word = |offset: usize| -> Vec<u8> {
            (offset..offset + 32)
                .map(|i| input.get(i).copied().unwrap_or(0))
                .collect()
        };
        let length = |i: usize| {
            let word = word(32 * i);
            assert!(word[..24].iter().all(|byte| *byte == 0), "{name}");
            u64::from_be_bytes(word[24..].try_into().unwrap()) as usize
        };
        let (base_len, exp_len, mod_len) = (length(0), length(1), length(2));
        let operand = |offset: usize, len: usize| -> Vec<u8> {
            (96 + offset..96 + offset + len)
                .map(|i| input.get(i).copied().unwrap_or(0))
                .collect()
        };
        let base = operand(0, base_len);
        let exp = operand(base_len, exp_len);
        let modulus = operand(base_len + exp_len, mod_len);
        let ((status, output), _) = with_and_without_recording(|| {
            let mut output = vec![0xa5; mod_len + 1];
            let status = unsafe {
                zkvm_modexp(
                    base.as_ptr(),
                    base_len,
                    exp.as_ptr(),
                    exp_len,
                    modulus.as_ptr(),
                    mod_len,
                    output.as_mut_ptr(),
                )
            };
            (status, output)
        });
        assert_eq!(status, EOK, "{name}");
        assert_eq!(&output[..mod_len], expected.as_slice(), "{name}");
        assert_eq!(output[mod_len], 0xa5, "{name}: wrote past mod_len bytes");
    }
}

fn modexp(base: &[u8], exp: &[u8], modulus: &[u8]) -> (Vec<u8>, Vec<u32>) {
    let ((status, output), tape) = with_and_without_recording(|| {
        let mut output = vec![0xa5; modulus.len()];
        // the pointers of empty slices are dangling, but aligned and not null, as C requires
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
fn modexp_edge_operands() {
    // empty operands and an empty modulus: no output
    assert_eq!(modexp(&[], &[], &[]).0, Vec::<u8>::new());
    // an empty base is zero; an empty exponent is zero, and 0^0 = 1
    assert_eq!(modexp(&[], &[], &[7]).0, [1]);
    assert_eq!(modexp(&[], &[3], &[7]).0, [0]);
    assert_eq!(modexp(&[5], &[], &[0, 7]).0, [0, 1]);
    // a zero modulus, of any length, gives zeroes
    assert_eq!(modexp(&[2], &[10], &[0, 0, 0, 0]).0, [0; 4]);
    // modulus one
    assert_eq!(modexp(&[2], &[10], &[1]).0, [0]);
    // 1-byte, 32-byte and 33-byte moduli: the single-digit and the multi-digit hinted paths
    let (output, tape) = modexp(&[2], &[10], &[0, 0, 3, 232]);
    assert_eq!(output, [0, 0, 0, 24]);
    assert!(!tape.is_empty());
    let mut modulus = [0u8; 32];
    modulus[0] = 0x80;
    modulus[31] = 0x01;
    let (output, tape) = modexp(&[3], &[5], &modulus);
    let mut expected = [0u8; 32];
    expected[31] = 243;
    assert_eq!(output, expected);
    assert!(!tape.is_empty());
    let mut modulus = [0u8; 33];
    modulus[0] = 1;
    modulus[32] = 1;
    // (2^128)^3 = 2^128 2^256 = -2^128 modulo 2^256 + 1, which is 2^256 - 2^128 + 1
    let mut base = [0u8; 17];
    base[0] = 1;
    let (output, tape) = modexp(&base, &[3], &modulus);
    let mut expected = [0u8; 33];
    expected[1..17].fill(0xff);
    expected[32] = 1;
    assert_eq!(output, expected);
    assert!(!tape.is_empty());
}

/// `(digest, recovery ID, r || s)` of an EVM ecrecover input
fn ecrecover_input(input: &[u8; 128]) -> (Bytes<32>, u8, Bytes<64>) {
    let digest = Bytes {
        data: input[..32].try_into().unwrap(),
    };
    assert!(input[32..63].iter().all(|byte| *byte == 0));
    let signature = Bytes {
        data: input[64..].try_into().unwrap(),
    };
    (digest, input[63] - 27, signature)
}

fn ecrecover(digest: &Bytes<32>, recid: u8, signature: &Bytes<64>) -> ((i32, Bytes<64>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 64] }, |out| unsafe {
            zkvm_secp256k1_ecrecover(digest, signature, recid, out)
        })
    })
}

#[test]
fn ecrecover_vectors_of_zksync_os() {
    // from zksync-os e855bca2:basic_system/src/system_functions/ecrecover.rs (tests), with the
    // addresses they expect
    const GETH_INPUT: [u8; 128] = hex("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e000000000000000000000000000000000000000000000000000000000000001b38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");
    const REGRESSION: [u8; 128] = [
        34, 189, 7, 49, 212, 191, 250, 136, 64, 38, 37, 181, 186, 57, 224, 78, 233, 173, 214, 83,
        76, 49, 218, 108, 17, 157, 130, 90, 57, 130, 43, 41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 102, 102, 116, 99, 212, 10, 196,
        65, 102, 33, 136, 237, 62, 102, 50, 156, 33, 172, 161, 101, 19, 51, 146, 204, 26, 20, 184,
        68, 133, 96, 10, 135, 80, 135, 255, 193, 105, 5, 204, 108, 234, 239, 23, 70, 48, 206, 157,
        208, 196, 11, 63, 78, 148, 255, 0, 238, 54, 88, 166, 166, 127, 236, 38, 19,
    ];
    for (input, address) in [
        (
            GETH_INPUT,
            hex::<20>("ceaccac640adf55b2028469bd36ba501f28b699d"),
        ),
        (REGRESSION, hex("63f9725f107358c9115bc9d86c72dd5823e9b1e6")),
    ] {
        let (digest, recid, signature) = ecrecover_input(&input);
        let ((status, key), tape) = ecrecover(&digest, recid, &signature);
        assert_eq!(status, EOK);
        assert_eq!(keccak(&key.data)[12..], address);
        assert!(!tape.is_empty());
    }
    // The point at infinity as the key, and a decompression regression: no key
    for input in [
        hex("6b8d2c81b11b2d699528dde488dbdf2f94293d0d33c32e347f255fa4a6c1f0a9000000000000000000000000000000000000000000000000000000000000001b79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f817986b8d2c81b11b2d699528dde488dbdf2f94293d0d33c32e347f255fa4a6c1f0a9"),
        hex("00c547e4f7b0f325ad1e56f57e26c745b09a3e503d86e00e5255ff7f715d3d1c000000000000000000000000000000000000000000000000000000000000001c00b1693892219d736caba55bdb67216e485557ea6b6af75f37096c9aa6a5a75f00b940b1d03b21e36b0e47e79769f095fe2ab855bd91e3a38756b7d75a9c4549"),
    ] {
        let (digest, recid, signature) = ecrecover_input(&input);
        let ((status, key), _) = ecrecover(&digest, recid, &signature);
        assert_eq!((status, key.data), (EFAIL, [0xa5; 64]));
    }
}

#[test]
fn ecrecover_edge_inputs() {
    use crypto::k256::ecdsa::SigningKey;
    let key = SigningKey::from_bytes(&[3u8; 32].into()).unwrap();
    let digest = Bytes { data: [0x11; 32] };
    let (signature, id) = key.sign_prehash_recoverable(&digest.data).unwrap();
    let signature = Bytes {
        data: signature.to_bytes().into(),
    };
    let public_key: [u8; 64] = key.verifying_key().to_encoded_point(false).as_bytes()[1..]
        .try_into()
        .unwrap();
    let ((status, recovered), _) = ecrecover(&digest, id.to_byte(), &signature);
    assert_eq!((status, recovered.data), (EOK, public_key));
    // the other parity is another key, or none
    let ((status, recovered), _) = ecrecover(&digest, id.to_byte() ^ 1, &signature);
    assert!(status == EFAIL || recovered.data != public_key);
    // r or s zero or at least the order, and recovery IDs of an x above the order or above 3
    let n = hex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    for (r, s) in [
        ([0; 32], signature.data[32..].try_into().unwrap()),
        (signature.data[..32].try_into().unwrap(), [0; 32]),
        (n, signature.data[32..].try_into().unwrap()),
        (signature.data[..32].try_into().unwrap(), n),
    ] {
        let mut data = [0; 64];
        data[..32].copy_from_slice(&r);
        data[32..].copy_from_slice(&s);
        let ((status, _), tape) = ecrecover(&digest, 0, &Bytes { data });
        assert_eq!((status, tape), (EFAIL, vec![]));
    }
    for recid in [2, 3, 4, 27] {
        let ((status, _), _) = ecrecover(&digest, recid, &signature);
        assert_eq!(status, EFAIL);
    }
}

#[test]
fn secp256k1_verify_takes_the_hints_of_the_hooks() {
    use crypto::hints::{record_hints, Secp256k1HooksWithOracle};
    use crypto::k256::ecdsa::SigningKey;
    let key = SigningKey::from_bytes(&[5u8; 32].into()).unwrap();
    let digest = Bytes { data: [0x22; 32] };
    let (signature, _) = key.sign_prehash_recoverable(&digest.data).unwrap();
    let signature = Bytes {
        data: signature.to_bytes().into(),
    };
    let public_key = Bytes {
        data: key.verifying_key().to_encoded_point(false).as_bytes()[1..]
            .try_into()
            .unwrap(),
    };
    let ((status, verified), tape) = with_and_without_recording(|| {
        call(false, |out| unsafe {
            zkvm_secp256k1_verify(&digest, &signature, &public_key, out)
        })
    });
    assert_eq!((status, verified), (EOK, true));
    let (expected, hinted) = record_hints(|o| {
        crypto::secp256k1::verify_prehash_with_hooks(
            &digest.data,
            &signature.data,
            &public_key.data,
            &mut Secp256k1HooksWithOracle::new(o),
        )
    });
    assert_eq!(expected, Some(true));
    assert!(!tape.is_empty());
    assert_eq!(tape, hinted);
    let mut wrong = digest;
    wrong.data[0] ^= 1;
    let ((status, verified), _) = with_and_without_recording(|| {
        call(true, |out| unsafe {
            zkvm_secp256k1_verify(&wrong, &signature, &public_key, out)
        })
    });
    assert_eq!((status, verified), (EOK, false));
    // a public key off the curve
    let mut off_curve = public_key;
    off_curve.data[63] ^= 1;
    let ((status, verified), tape) = with_and_without_recording(|| {
        call(true, |out| unsafe {
            zkvm_secp256k1_verify(&digest, &signature, &off_curve, out)
        })
    });
    assert_eq!((status, verified, tape), (EFAIL, true, vec![]));
}

fn bn_add(a: &Bytes<64>, b: &Bytes<64>) -> ((i32, Bytes<64>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 64] }, |out| unsafe {
            zkvm_bn254_g1_add(a, b, out)
        })
    })
}

fn bn_mul(p: &Bytes<64>, scalar: &[u8; 32]) -> ((i32, Bytes<64>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 64] }, |out| unsafe {
            zkvm_bn254_g1_mul(p, &Bytes { data: *scalar }, out)
        })
    })
}

#[test]
fn bn254_add_and_mul_edge_inputs() {
    let g = bn::G1Affine::generator();
    let g2 = (g + g).into_affine();
    let zero = bn_g1(&bn::G1Affine::zero());
    let ((status, sum), tape) = bn_add(&bn_g1(&g), &bn_g1(&g2));
    assert_eq!((status, sum), (EOK, bn_g1(&(g + g2).into_affine())));
    assert!(!tape.is_empty());
    // doubling, infinity on either side, and a point plus its negation
    assert_eq!(bn_add(&bn_g1(&g), &bn_g1(&g)).0, (EOK, bn_g1(&g2)));
    assert_eq!(bn_add(&zero, &bn_g1(&g)).0, (EOK, bn_g1(&g)));
    assert_eq!(bn_add(&bn_g1(&g), &zero).0, (EOK, bn_g1(&g)));
    assert_eq!(bn_add(&zero, &zero).0, (EOK, zero));
    assert_eq!(bn_add(&bn_g1(&g), &bn_g1(&-g)).0, (EOK, zero));
    // off the curve, and a coordinate that is not canonical (x = p, which is x = 0 reduced)
    let mut off_curve = bn_g1(&g);
    off_curve.data[63] ^= 1;
    let mut x_is_p = Bytes { data: [0; 64] };
    x_is_p.data[..32].copy_from_slice(&bn::Fq::MODULUS.to_bytes_be());
    x_is_p.data[32..].copy_from_slice(&bn_g1(&g).data[32..]);
    for bad in [off_curve, x_is_p] {
        let ((status, out), tape) = bn_add(&bn_g1(&g), &bad);
        assert_eq!((status, out.data, tape), (EFAIL, [0xa5; 64], vec![]));
        let ((status, _), tape) = bn_mul(&bad, &[1; 32]);
        assert_eq!((status, tape), (EFAIL, vec![]));
    }
    // scalars: zero, one, the order, the order plus one, and 2^256 - 1, which reduce
    let p = (g * bn::Fr::from(7u64)).into_affine();
    for scalar in scalars::<bn::Fr>() {
        let expected = (p * bn::Fr::from_be_bytes_mod_order(&scalar)).into_affine();
        let ((status, product), _) = bn_mul(&bn_g1(&p), &scalar);
        assert_eq!((status, product), (EOK, bn_g1(&expected)));
        // infinity times anything
        assert_eq!(bn_mul(&zero, &scalar).0, (EOK, zero));
    }
    let ((_, _), tape) = bn_mul(&bn_g1(&p), &[0x5a; 32]);
    assert!(!tape.is_empty());
}

fn bn_pairing(pairs: &[Pair<64, 128>]) -> ((i32, bool), Vec<u32>) {
    with_and_without_recording(|| {
        call(false, |out| unsafe {
            // a valid pointer even for no pairs
            zkvm_bn254_pairing(pairs.as_ptr(), pairs.len(), out)
        })
    })
}

#[test]
fn bn254_pairing_vectors_and_edge_inputs() {
    // from zksync-os e855bca2:basic_system/src/system_functions/bn254_pairing_check.rs (tests):
    // two pairs whose product is the identity
    const SRC: [u8; 384] = hex(concat!(
        "1c76476f4def4bb94541d57ebba1193381ffa7aa76ada664dd31c16024c43f59",
        "3034dd2920f673e204fee2811c678745fc819b55d3e9d294e45c9b03a76aef41",
        "209dd15ebff5d46c4bd888e51a93cf99a7329636c63514396b4a452003a35bf7",
        "04bf11ca01483bfa8b34b43561848d28905960114c8ac04049af4b6315a41678",
        "2bb8324af6cfc93537a2ad1a445cfd0ca2a71acd7ac41fadbf933c2a51be344d",
        "120a2a4cf30c1bf9845f20c6fe39e07ea2cce61f0c9bb048165fe5e4de877550",
        "111e129f1cf1097710d41c4ac70fcdfa5ba2023c6ff1cbeac322de49d1b6df7c",
        "2032c61a830e3c17286de9462bf242fca2883585b93870a73853face6a6bf411",
        "198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2",
        "1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed",
        "090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b",
        "12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa",
    ));
    let pair = |bytes: &[u8]| Pair {
        first: Bytes {
            data: bytes[..64].try_into().unwrap(),
        },
        second: Bytes {
            data: bytes[64..192].try_into().unwrap(),
        },
    };
    let vector = [pair(&SRC[..192]), pair(&SRC[192..])];
    let ((status, verified), tape) = bn_pairing(&vector);
    assert_eq!((status, verified), (EOK, true));
    assert!(!tape.is_empty());
    assert_eq!(bn_pairing(&vector[..1]).0, (EOK, false));
    // no pairs, and pairs with a point at infinity: the identity
    assert_eq!(bn_pairing(&[]).0, (EOK, true));
    let g1 = bn::G1Affine::generator();
    let g2 = bn::G2Affine::generator();
    let with = |p: &bn::G1Affine, q: &bn::G2Affine| Pair {
        first: bn_g1(p),
        second: bn_g2(q),
    };
    assert_eq!(
        bn_pairing(&[with(&bn::G1Affine::zero(), &g2)]).0,
        (EOK, true)
    );
    assert_eq!(
        bn_pairing(&[with(&g1, &bn::G2Affine::zero())]).0,
        (EOK, true)
    );
    assert_eq!(
        bn_pairing(&[with(&g1, &g2), with(&-g1, &g2)]).0,
        (EOK, true)
    );
    // G2 outside the subgroup, G2 off the curve, G1 off the curve
    let outside = outside_subgroup::<bn::g2::Config>(|k| bn::Fq2::new(k.into(), 0u64.into()));
    let mut off_curve_g2 = bn_g2(&g2);
    off_curve_g2.data[127] ^= 1;
    let mut off_curve_g1 = bn_g1(&g1);
    off_curve_g1.data[63] ^= 1;
    for bad in [
        with(&g1, &outside),
        Pair {
            first: bn_g1(&g1),
            second: off_curve_g2,
        },
        Pair {
            first: off_curve_g1,
            second: bn_g2(&g2),
        },
    ] {
        assert_eq!(bn_pairing(&[with(&g1, &g2), bad]).0, (EFAIL, false));
    }
}

fn bls_add1(a: &Bytes<96>, b: &Bytes<96>) -> ((i32, Bytes<96>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 96] }, |out| unsafe {
            zkvm_bls12_g1_add(a, b, out)
        })
    })
}

fn bls_add2(a: &Bytes<192>, b: &Bytes<192>) -> ((i32, Bytes<192>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 192] }, |out| unsafe {
            zkvm_bls12_g2_add(a, b, out)
        })
    })
}

#[test]
fn bls12_381_add_edge_inputs() {
    let g = bls::G1Affine::generator();
    let g2 = (g + g).into_affine();
    let outside = outside_subgroup::<bls::g1::Config>(bls::Fq::from);
    let zero = bls_g1(&bls::G1Affine::zero());
    let ((status, sum), tape) = bls_add1(&bls_g1(&g), &bls_g1(&g2));
    assert_eq!((status, sum), (EOK, bls_g1(&(g + g2).into_affine())));
    assert!(!tape.is_empty());
    assert_eq!(bls_add1(&bls_g1(&g), &bls_g1(&g)).0, (EOK, bls_g1(&g2)));
    assert_eq!(bls_add1(&zero, &bls_g1(&g)).0, (EOK, bls_g1(&g)));
    assert_eq!(bls_add1(&bls_g1(&g), &bls_g1(&-g)).0, (EOK, zero));
    // EIP-2537 addition does not ask for the subgroup
    let sum = (outside + g).into_affine();
    assert_eq!(
        bls_add1(&bls_g1(&outside), &bls_g1(&g)).0,
        (EOK, bls_g1(&sum))
    );
    let mut off_curve = bls_g1(&g);
    off_curve.data[95] ^= 1;
    for bad in [off_curve, Bytes { data: [0xff; 96] }] {
        let ((status, out), tape) = bls_add1(&bad, &bls_g1(&g));
        assert_eq!((status, out.data, tape), (EFAIL, [0xa5; 96], vec![]));
    }

    let q = bls::G2Affine::generator();
    let q2 = (q + q).into_affine();
    let outside = outside_subgroup::<bls::g2::Config>(|k| bls::Fq2::new(k.into(), 0u64.into()));
    let zero = bls_g2(&bls::G2Affine::zero());
    let ((status, sum), tape) = bls_add2(&bls_g2(&q), &bls_g2(&q2));
    assert_eq!((status, sum), (EOK, bls_g2(&(q + q2).into_affine())));
    assert!(!tape.is_empty());
    assert_eq!(bls_add2(&bls_g2(&q), &bls_g2(&q)).0, (EOK, bls_g2(&q2)));
    assert_eq!(bls_add2(&bls_g2(&q), &zero).0, (EOK, bls_g2(&q)));
    assert_eq!(bls_add2(&bls_g2(&q), &bls_g2(&-q)).0, (EOK, zero));
    let sum = (outside + q).into_affine();
    assert_eq!(
        bls_add2(&bls_g2(&outside), &bls_g2(&q)).0,
        (EOK, bls_g2(&sum))
    );
    let mut off_curve = bls_g2(&q);
    off_curve.data[191] ^= 1;
    for bad in [off_curve, Bytes { data: [0xff; 192] }] {
        let ((status, out), tape) = bls_add2(&bls_g2(&q), &bad);
        assert_eq!((status, out.data, tape), (EFAIL, [0xa5; 192], vec![]));
    }
}

fn g1_msm(pairs: &[Pair<96, 32>]) -> ((i32, Bytes<96>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 96] }, |out| unsafe {
            zkvm_bls12_g1_msm(pairs.as_ptr(), pairs.len(), out)
        })
    })
}

fn g2_msm(pairs: &[Pair<192, 32>]) -> ((i32, Bytes<192>), Vec<u32>) {
    with_and_without_recording(|| {
        call(Bytes { data: [0xa5; 192] }, |out| unsafe {
            zkvm_bls12_g2_msm(pairs.as_ptr(), pairs.len(), out)
        })
    })
}

#[test]
fn bls12_381_msm_edge_inputs() {
    let g = bls::G1Affine::generator();
    let term = |p: &bls::G1Affine, k: [u8; 32]| Pair {
        first: bls_g1(p),
        second: Bytes { data: k },
    };
    let times =
        |p: &bls::G1Affine, k: &[u8; 32]| p.into_group() * bls::Fr::from_be_bytes_mod_order(k);
    // empty: rejected, as EIP-2537 does
    assert_eq!(g1_msm(&[]).0, (EFAIL, Bytes { data: [0xa5; 96] }));
    for k in scalars::<bls::Fr>() {
        assert_eq!(
            g1_msm(&[term(&g, k)]).0,
            (EOK, bls_g1(&times(&g, &k).into_affine()))
        );
        assert_eq!(
            g1_msm(&[term(&bls::G1Affine::zero(), k)]).0,
            (EOK, bls_g1(&bls::G1Affine::zero()))
        );
    }
    // two terms (the affine path) and four (Pippenger)
    let points: Vec<_> = (2..6u64)
        .map(|k| (g * bls::Fr::from(k)).into_affine())
        .collect();
    let ks: Vec<[u8; 32]> = (0..4u8).map(|i| keccak(&[i, 0x61])).collect();
    for n in [2, 4] {
        let terms: Vec<_> = (0..n).map(|i| term(&points[i], ks[i])).collect();
        let expected = (0..n)
            .map(|i| times(&points[i], &ks[i]))
            .sum::<bls::G1Projective>();
        let ((status, sum), tape) = g1_msm(&terms);
        assert_eq!((status, sum), (EOK, bls_g1(&expected.into_affine())));
        assert!(!tape.is_empty());
    }
    // outside the subgroup, even with a zero scalar
    let outside = outside_subgroup::<bls::g1::Config>(bls::Fq::from);
    assert_eq!(
        g1_msm(&[term(&g, ks[0]), term(&outside, [0; 32])]).0 .0,
        EFAIL
    );

    let q = bls::G2Affine::generator();
    let term = |p: &bls::G2Affine, k: [u8; 32]| Pair {
        first: bls_g2(p),
        second: Bytes { data: k },
    };
    let times =
        |p: &bls::G2Affine, k: &[u8; 32]| p.into_group() * bls::Fr::from_be_bytes_mod_order(k);
    assert_eq!(g2_msm(&[]).0, (EFAIL, Bytes { data: [0xa5; 192] }));
    for k in scalars::<bls::Fr>() {
        assert_eq!(
            g2_msm(&[term(&q, k)]).0,
            (EOK, bls_g2(&times(&q, &k).into_affine()))
        );
    }
    let points: Vec<_> = (2..6u64)
        .map(|k| (q * bls::Fr::from(k)).into_affine())
        .collect();
    for n in [2, 4] {
        let terms: Vec<_> = (0..n).map(|i| term(&points[i], ks[i])).collect();
        let expected = (0..n)
            .map(|i| times(&points[i], &ks[i]))
            .sum::<bls::G2Projective>();
        let ((status, sum), tape) = g2_msm(&terms);
        assert_eq!((status, sum), (EOK, bls_g2(&expected.into_affine())));
        assert!(!tape.is_empty());
    }
    let outside = outside_subgroup::<bls::g2::Config>(|k| bls::Fq2::new(k.into(), 0u64.into()));
    assert_eq!(g2_msm(&[term(&outside, ks[0])]).0 .0, EFAIL);
}

fn bls_pairing(pairs: &[Pair<96, 192>]) -> ((i32, bool), Vec<u32>) {
    with_and_without_recording(|| {
        call(false, |out| unsafe {
            zkvm_bls12_pairing(pairs.as_ptr(), pairs.len(), out)
        })
    })
}

#[test]
fn bls12_381_pairing_edge_inputs() {
    let g1 = bls::G1Affine::generator();
    let g2 = bls::G2Affine::generator();
    let with = |p: &bls::G1Affine, q: &bls::G2Affine| Pair {
        first: bls_g1(p),
        second: bls_g2(q),
    };
    assert_eq!(bls_pairing(&[]).0, (EFAIL, false));
    let ((status, verified), tape) = bls_pairing(&[with(&g1, &g2), with(&-g1, &g2)]);
    assert_eq!((status, verified), (EOK, true));
    assert!(!tape.is_empty());
    assert_eq!(bls_pairing(&[with(&g1, &g2)]).0, (EOK, false));
    assert_eq!(
        bls_pairing(&[with(&bls::G1Affine::zero(), &g2)]).0,
        (EOK, true)
    );
    assert_eq!(
        bls_pairing(&[with(&g1, &bls::G2Affine::zero())]).0,
        (EOK, true)
    );
    let outside1 = outside_subgroup::<bls::g1::Config>(bls::Fq::from);
    let outside2 = outside_subgroup::<bls::g2::Config>(|k| bls::Fq2::new(k.into(), 0u64.into()));
    assert_eq!(
        bls_pairing(&[with(&g1, &g2), with(&outside1, &g2)]).0,
        (EFAIL, false)
    );
    assert_eq!(bls_pairing(&[with(&g1, &outside2)]).0, (EFAIL, false));
}

#[test]
fn bls12_381_map_edge_inputs() {
    let fp = |value: bls::Fq| {
        let mut out = Bytes { data: [0; 48] };
        put(value, &mut out.data);
        out
    };
    let p_minus_one = -bls::Fq::from(1u64);
    let random = bls::Fq::from_be_bytes_mod_order(&keccak(b"map"));
    for value in [
        bls::Fq::from(0u64),
        bls::Fq::from(1u64),
        p_minus_one,
        random,
    ] {
        let ((status, point), tape) = with_and_without_recording(|| {
            call(Bytes { data: [0xa5; 96] }, |out| unsafe {
                zkvm_bls12_map_fp_to_g1(&fp(value), out)
            })
        });
        let expected = bls::eip2537::map_fp_to_g1(value).unwrap().clear_cofactor();
        assert!(expected.is_in_correct_subgroup_assuming_on_curve());
        assert_eq!((status, point), (EOK, bls_g1(&expected)));
        assert!(!tape.is_empty());
        for (c0, c1) in [(value, random), (random, value)] {
            let mut fp2 = Bytes { data: [0; 96] };
            put(c0, &mut fp2.data[..48]);
            put(c1, &mut fp2.data[48..]);
            let ((status, point), tape) = with_and_without_recording(|| {
                call(Bytes { data: [0xa5; 192] }, |out| unsafe {
                    zkvm_bls12_map_fp2_to_g2(&fp2, out)
                })
            });
            let expected = bls::eip2537::map_fp2_to_g2(bls::Fq2::new(c0, c1))
                .unwrap()
                .clear_cofactor();
            assert!(expected.is_in_correct_subgroup_assuming_on_curve());
            assert_eq!((status, point), (EOK, bls_g2(&expected)));
            assert!(!tape.is_empty());
        }
    }
    // the modulus is not canonical
    let modulus = bls::Fq::MODULUS.to_bytes_be();
    let p = Bytes {
        data: modulus[modulus.len() - 48..].try_into().unwrap(),
    };
    let ((status, _), tape) = recorded(|| {
        call(Bytes { data: [0xa5; 96] }, |out| unsafe {
            zkvm_bls12_map_fp_to_g1(&p, out)
        })
    });
    assert_eq!((status, tape), (EFAIL, vec![]));
    let mut fp2 = Bytes { data: [0; 96] };
    fp2.data[48..].copy_from_slice(&p.data);
    let ((status, _), tape) = recorded(|| {
        call(Bytes { data: [0xa5; 192] }, |out| unsafe {
            zkvm_bls12_map_fp2_to_g2(&fp2, out)
        })
    });
    assert_eq!((status, tape), (EFAIL, vec![]));
}

fn kzg(
    commitment: &[u8; 48],
    z: &[u8; 32],
    y: &[u8; 32],
    proof: &[u8; 48],
) -> ((i32, bool), Vec<u32>) {
    with_and_without_recording(|| {
        call(false, |out| unsafe {
            zkvm_kzg_point_eval(
                &Bytes { data: *commitment },
                &Bytes { data: *z },
                &Bytes { data: *y },
                &Bytes { data: *proof },
                out,
            )
        })
    })
}

#[test]
fn kzg_vectors_and_edge_inputs() {
    // zksync-os e855bca2:basic_system/src/system_functions/point_evaluation.rs (tests), from
    // c-kzg-4844 verify_kzg_proof_case_correct_proof_4_4
    const COMMITMENT: [u8; 48] = hex("8f59a8d2a1a625a17f3fea0fe5eb8c896db3764f3185481bc22f91b4aaffcca25f26936857bc3a7c2539ea8ec3a952b7");
    const Z: [u8; 32] = hex("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000000");
    const Y: [u8; 32] = hex("1522a4a7f34e1ea350ae07c29c96c7e79655aa926122e95fe69fcbd932ca49e9");
    const PROOF: [u8; 48] = hex("a62ad71d14c5719385c0686f1871430475bf3a00f0aa3f7b8dd99a9abc2160744faf0070725e00b60ad9a026a15b1a8c");
    let ((status, verified), tape) = kzg(&COMMITMENT, &Z, &Y, &PROOF);
    assert_eq!((status, verified), (EOK, true));
    assert!(!tape.is_empty());
    // c-kzg-4844 vectors (`(commitment, proof, z, y)`, valid), from the pre-port affine_glv.rs tests
    // as airbender-crypto's kzg tests take them
    type Case = ([u8; 48], [u8; 48], [u8; 32], [u8; 32]);
    const C_KZG_CASES: [(Case, bool); 3] = [
        ((hex("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06"), hex("a1fcd37a924af9ec04143b44853c26f6b0738f6e15a3e0755057e7d5460406c7e148adb0e2d608982140d0ae42fe0b3b"), hex("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62"), hex("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0")), true),
        ((hex("b49d88afcd7f6c61a8ea69eff5f609d2432b47e7e4cd50b02cdddb4e0c1460517e8df02e4e64dc55e3d8ca192d57193a"), hex("a38758fca85407078c0a7e5fd6d38b34340c809baa0e1fed9deaabb11aa503062acbbe23fcbe620a21b40a83bfa71b89"), hex("0000000000000000000000000000000000000000000000000000000000000002"), hex("6a75e4fe63e5e148c853462a680c3e3ccedea34719d28f19bf1b35ae4eea37d6")), true),
        ((hex("a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06"), hex("b3477fc9a5bfab5fdb5523251818ee5a6d52613c59502a3d2df58217f4e366cd9ef37dee55bf2c705a2b08e7808b6fa0"), hex("5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62"), hex("5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0")), false),
    ];
    for ((commitment, proof, z, y), valid) in C_KZG_CASES {
        assert_eq!(kzg(&commitment, &z, &y, &proof).0, (EOK, valid));
    }
    // the point at infinity as both, with z = y = 0: the zero polynomial
    let infinity = hex("c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    assert_eq!(kzg(&infinity, &[0; 32], &[0; 32], &infinity).0, (EOK, true));
    // a scalar of at least the order, and a point that is not a compressed point
    let r = order::<bls::Fr>();
    assert_eq!(kzg(&infinity, &[0; 32], &r, &infinity).0, (EFAIL, false));
    assert_eq!(kzg(&infinity, &r, &[0; 32], &infinity).0, (EFAIL, false));
    let ((status, _), tape) = kzg(&[0xff; 48], &Z, &Y, &PROOF);
    assert_eq!((status, tape), (EFAIL, vec![]));
}
