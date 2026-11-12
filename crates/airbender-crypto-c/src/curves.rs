//! C bindings of the curve precompile endpoints.
use crate::{commit, crypto, input, point_bytes, read, with_active_oracle};
use crate::{Bytes, Pair};
use core::{mem, slice};
use crypto::bls12_381 as bls;
use crypto::precompiles::{
    self, MAX_BLS12_381_G1_MSM_PAIRS, MAX_BLS12_381_G2_MSM_PAIRS, MAX_BLS12_381_PAIRS,
    MAX_BN254_PAIRS,
};

/// Compact BLS12-381 coordinates widened to EIP-2537's 64 bytes per base-field element
fn eip2537<const N: usize, const M: usize>(compact: &[u8; N]) -> [u8; M] {
    const { assert!(N.is_multiple_of(48) && M == N / 48 * 64) };
    let mut padded = [0; M];
    for (dst, src) in padded
        .as_chunks_mut::<64>()
        .0
        .iter_mut()
        .zip(compact.as_chunks::<48>().0)
    {
        dst[16..].copy_from_slice(src);
    }
    padded
}

fn bls_out1(p: bls::G1Affine) -> Bytes<96> {
    point_bytes(p)
}
fn bls_out2(p: bls::G2Affine) -> Bytes<192> {
    point_bytes(p)
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bn254_g1_add(
    p1: *const Bytes<64>,
    p2: *const Bytes<64>,
    result: *mut Bytes<64>,
) -> i32 {
    commit(result, || {
        let coordinates = [read(p1).data, read(p2).data];
        let sum = with_active_oracle(|oracle| precompiles::bn254_g1_add(&coordinates, oracle));
        sum.ok().map(|data| Bytes { data })
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bn254_g1_mul(
    point: *const Bytes<64>,
    scalar: *const Bytes<32>,
    result: *mut Bytes<64>,
) -> i32 {
    commit(result, || {
        let point = read(point).data;
        let scalar = read(scalar).data;
        let ([x, y], []) = point.as_chunks::<32>() else {
            unreachable!("64 bytes are two coordinates")
        };
        let product = with_active_oracle(|oracle| precompiles::bn254_g1_mul(x, y, &scalar, oracle));
        product.ok().map(|data| Bytes { data })
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bn254_pairing(
    pairs: *const Pair<64, 128>,
    num_pairs: usize,
    verified: *mut bool,
) -> i32 {
    commit(verified, || {
        // A transaction has at most 2^24 gas (EIP-7825) and a check costs 45000 + 34000 per
        // pair (EIP-1108), so it can pay for (2^24 - 45000) / 34000 = 492 pairs: more fail
        // before the input is read.
        if num_pairs > MAX_BN254_PAIRS {
            return None;
        }
        let pairs = input(pairs, num_pairs);
        // the EIP-197 encoding: `Pair` is two byte arrays with no padding between or after them
        const { assert!(mem::size_of::<Pair<64, 128>>() == 192) };
        let src = slice::from_raw_parts(pairs.as_ptr().cast::<u8>(), mem::size_of_val(pairs));
        with_active_oracle(|oracle| precompiles::bn254_pairing_check(num_pairs, src, oracle)).ok()
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_g1_add(
    p1: *const Bytes<96>,
    p2: *const Bytes<96>,
    result: *mut Bytes<96>,
) -> i32 {
    commit(result, || {
        let (a, b) = (eip2537(&read(p1).data), eip2537(&read(p2).data));
        let sum = with_active_oracle(|oracle| precompiles::bls12_381_g1_add(&a, &b, oracle));
        sum.ok().map(bls_out1)
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_g2_add(
    p1: *const Bytes<192>,
    p2: *const Bytes<192>,
    result: *mut Bytes<192>,
) -> i32 {
    commit(result, || {
        let (a, b) = (eip2537(&read(p1).data), eip2537(&read(p2).data));
        let sum = with_active_oracle(|oracle| precompiles::bls12_381_g2_add(&a, &b, oracle));
        sum.ok().map(bls_out2)
    })
}

/// The EIP-2537 MSM input of compact `(point, scalar)` pairs, widened one pair at a time; `None`
/// for none, which EIP-2537 rejects
fn msm_input<const P: usize, const Q: usize, const N: usize>(
    pairs: &[Pair<P, 32>],
) -> Option<impl ExactSizeIterator<Item = [u8; N]> + '_> {
    const { assert!(N == Q + 32) };
    if pairs.is_empty() {
        return None;
    }
    Some(pairs.iter().map(|pair| {
        let mut bytes = [0; N];
        bytes[..Q].copy_from_slice(&eip2537::<P, Q>(&pair.first.data));
        bytes[Q..].copy_from_slice(&pair.second.data);
        bytes
    }))
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_g1_msm(
    pairs: *const Pair<96, 32>,
    num_pairs: usize,
    result: *mut Bytes<96>,
) -> i32 {
    commit(result, || {
        // A transaction has at most 2^24 gas (EIP-7825) and a pair costs 12000 gas at the
        // largest discount, 519/1000 from 128 pairs on (EIP-2537), so it can pay for
        // 2^24 / (12000 * 519 / 1000) = 2693 pairs: more fail before the input is read.
        if num_pairs > MAX_BLS12_381_G1_MSM_PAIRS {
            return None;
        }
        let input = msm_input::<96, 128, 160>(input(pairs, num_pairs))?;
        let sum = with_active_oracle(|oracle| precompiles::bls12_381_g1_msm(input, oracle));
        sum.ok().map(bls_out1)
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_g2_msm(
    pairs: *const Pair<192, 32>,
    num_pairs: usize,
    result: *mut Bytes<192>,
) -> i32 {
    commit(result, || {
        // A transaction has at most 2^24 gas (EIP-7825) and a pair costs 22500 gas at the
        // largest discount, 524/1000 from 128 pairs on (EIP-2537), so it can pay for
        // 2^24 / (22500 * 524 / 1000) = 1423 pairs: more fail before the input is read.
        if num_pairs > MAX_BLS12_381_G2_MSM_PAIRS {
            return None;
        }
        let input = msm_input::<192, 256, 288>(input(pairs, num_pairs))?;
        let sum = with_active_oracle(|oracle| precompiles::bls12_381_g2_msm(input, oracle));
        sum.ok().map(bls_out2)
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_pairing(
    pairs: *const Pair<96, 192>,
    num_pairs: usize,
    verified: *mut bool,
) -> i32 {
    commit(verified, || {
        // A transaction has at most 2^24 gas (EIP-7825) and a check costs 37700 + 32600 per
        // pair (EIP-2537), so it can pay for (2^24 - 37700) / 32600 = 513 pairs: more fail
        // before the input is read.
        if num_pairs > MAX_BLS12_381_PAIRS {
            return None;
        }
        let pairs = input(pairs, num_pairs);
        // EIP-2537 rejects an empty input; the endpoint leaves it to its caller
        if pairs.is_empty() {
            return None;
        }
        // widened to the EIP-2537 encoding one pair at a time
        let input = pairs.iter().map(|pair| {
            let mut bytes = [0; 384];
            bytes[..128].copy_from_slice(&eip2537::<96, 128>(&pair.first.data));
            bytes[128..].copy_from_slice(&eip2537::<192, 256>(&pair.second.data));
            bytes
        });
        with_active_oracle(|oracle| precompiles::bls12_381_pairing_check(input, oracle)).ok()
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_map_fp_to_g1(
    element: *const Bytes<48>,
    result: *mut Bytes<96>,
) -> i32 {
    commit(result, || {
        let fp = eip2537(&read(element).data);
        let point = with_active_oracle(|oracle| precompiles::bls12_381_map_fp_to_g1(&fp, oracle));
        point.ok().map(bls_out1)
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_bls12_map_fp2_to_g2(
    element: *const Bytes<96>,
    result: *mut Bytes<192>,
) -> i32 {
    commit(result, || {
        let fp2 = eip2537(&read(element).data);
        let point = with_active_oracle(|oracle| precompiles::bls12_381_map_fp2_to_g2(&fp2, oracle));
        point.ok().map(bls_out2)
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_kzg_point_eval(
    commitment: *const Bytes<48>,
    z: *const Bytes<32>,
    y: *const Bytes<32>,
    proof: *const Bytes<48>,
    verified: *mut bool,
) -> i32 {
    commit(verified, || {
        let (commitment, z, y, proof) = (read(commitment), read(z), read(y), read(proof));
        with_active_oracle(|oracle| {
            precompiles::kzg_verify_proof(&commitment.data, &proof.data, &z.data, &y.data, oracle)
        })
        .ok()
    })
}
