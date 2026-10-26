//! C bindings of the curve precompiles (airbender-crypto's `precompiles` endpoints, the crypto
//! half of zksync-os's) and of the subgroup, cofactor and normalization helpers.
use crate::{commit, crypto, decode, input, point, point_bytes, read, with_active_oracle};
use crate::{Bytes, Pair};
use alloc::alloc::Global;
use alloc::vec::Vec;
use core::{mem, slice};
use crypto::ark_ec::short_weierstrass::{Affine, Projective, SWCurveConfig};
use crypto::ark_ec::{AffineRepr, CurveGroup};
use crypto::precompiles;
use crypto::{bls12_381 as bls, bn254 as bn};
#[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
use crypto::{
    hinted_ops,
    hints::{self, Bls12381Fq2Divider, Bls12381FqDivider},
};

/// The EIP-2537 encoding of compact BLS12-381 coordinates: each 48-byte base-field element
/// widened to 64 bytes with leading zeroes
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

fn bls_g1(bytes: &[u8; 96]) -> Option<bls::G1Affine> {
    point(bytes)
}
fn bls_g2(bytes: &[u8; 192]) -> Option<bls::G2Affine> {
    point(bytes)
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
        let pairs = input(pairs, num_pairs);
        // the EIP-197 encoding: `Pair` is two byte arrays with no padding between or after them
        const { assert!(mem::size_of::<Pair<64, 128>>() == 192) };
        let src = slice::from_raw_parts(pairs.as_ptr().cast::<u8>(), mem::size_of_val(pairs));
        with_active_oracle(|oracle| {
            precompiles::bn254_pairing_check(num_pairs, src, Global, oracle)
        })
        .ok()
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

/// The EIP-2537 MSM input of compact `(point, scalar)` pairs; `None` for none, which EIP-2537
/// rejects (and the endpoints leave to their caller)
fn msm_input<const P: usize, const Q: usize, const N: usize>(
    pairs: &[Pair<P, 32>],
) -> Option<Vec<[u8; N]>> {
    const { assert!(N == Q + 32) };
    if pairs.is_empty() {
        return None;
    }
    let encoded = pairs.iter().map(|pair| {
        let mut bytes = [0; N];
        bytes[..Q].copy_from_slice(&eip2537::<P, Q>(&pair.first.data));
        bytes[Q..].copy_from_slice(&pair.second.data);
        bytes
    });
    Some(encoded.collect())
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
        let input = msm_input::<96, 128, 160>(input(pairs, num_pairs))?;
        let sum =
            with_active_oracle(|oracle| precompiles::bls12_381_g1_msm(&input, Global, oracle));
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
        let input = msm_input::<192, 256, 288>(input(pairs, num_pairs))?;
        let sum =
            with_active_oracle(|oracle| precompiles::bls12_381_g2_msm(&input, Global, oracle));
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
        let pairs = input(pairs, num_pairs);
        // no pairs: rejected, as EIP-2537 does (the endpoint leaves it to its caller)
        if pairs.is_empty() {
            return None;
        }
        let input: Vec<[u8; 384]> = pairs
            .iter()
            .map(|pair| {
                let mut bytes = [0; 384];
                bytes[..128].copy_from_slice(&eip2537::<96, 128>(&pair.first.data));
                bytes[128..].copy_from_slice(&eip2537::<192, 256>(&pair.second.data));
                bytes
            })
            .collect();
        with_active_oracle(|oracle| precompiles::bls12_381_pairing_check(&input, Global, oracle))
            .ok()
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

/// The point of Jacobian coordinates `X || Y || Z`, normalized by `normalize`, if it is on the
/// curve (`Z = 0` is the point at infinity, whatever `X` and `Y`)
unsafe fn normalize<C: SWCurveConfig, const N: usize, const M: usize>(
    point: *const Bytes<N>,
    result: *mut Bytes<M>,
    normalize: impl FnOnce(&Projective<C>) -> Affine<C>,
) -> i32 {
    commit(result, || {
        let input = read(point).data;
        let size = N / 3;
        let p = Projective::<C>::new_unchecked(
            decode(&input[..size])?,
            decode(&input[size..2 * size])?,
            decode(&input[2 * size..])?,
        );
        let p = normalize(&p);
        p.is_on_curve().then(|| point_bytes(p))
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract. Coordinates are X, Y, Z (Jacobian).
pub unsafe extern "C" fn airbender_bn254_g1_to_affine(
    point: *const Bytes<96>,
    result: *mut Bytes<64>,
) -> i32 {
    normalize::<bn::g1::Config, 96, 64>(point, result, |p| {
        hinted!(
            |oracle| hinted_ops::to_affine_with_inverse(p, |z| hints::bn254_fq_inverse(oracle, z)),
            p.into_affine(),
        )
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract. Coordinates are X, Y, Z (Jacobian).
pub unsafe extern "C" fn airbender_bls12_g1_to_affine(
    point: *const Bytes<144>,
    result: *mut Bytes<96>,
) -> i32 {
    normalize::<bls::g1::Config, 144, 96>(point, result, |p| {
        hinted!(
            |oracle| bls::g1::into_affine_with_inverse(p, |z| hints::bls12_381_fq_inverse(
                oracle, z
            )),
            p.into_affine(),
        )
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract. Coordinates are X, Y, Z (Jacobian).
pub unsafe extern "C" fn airbender_bls12_g2_to_affine(
    point: *const Bytes<288>,
    result: *mut Bytes<192>,
) -> i32 {
    normalize::<bls::g2::Config, 288, 192>(point, result, |p| {
        hinted!(
            |oracle| bls::g2::into_affine_with_inverse(p, |z| hints::bls12_381_fq2_inverse(
                oracle, z
            )),
            p.into_affine(),
        )
    })
}

// These helpers accept any on-curve point; subgroup membership is the operation's result, not a
// precondition. Cofactor clearing likewise does not reduce its scalar under the assumption that
// the input is in the prime-order subgroup. With an oracle they are affine, with the dividers of
// checked hints (the infinity takes none), as the EIP-2537 endpoints check and clear.

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_bls12_g1_is_in_subgroup(
    point: *const Bytes<96>,
    result: *mut bool,
) -> i32 {
    commit(result, || {
        let p = bls_g1(&read(point).data)?;
        Some(
            p.is_zero()
                || hinted!(
                    |oracle| bls::g1::is_in_subgroup_with_divider(
                        &p,
                        &mut Bls12381FqDivider::new(oracle)
                    ),
                    p.is_in_correct_subgroup_assuming_on_curve(),
                ),
        )
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_bls12_g1_clear_cofactor(
    point: *const Bytes<96>,
    result: *mut Bytes<96>,
) -> i32 {
    commit(result, || {
        let p = bls_g1(&read(point).data)?;
        if p.is_zero() {
            return Some(bls_out1(p));
        }
        Some(bls_out1(hinted!(
            |oracle| bls::g1::clear_cofactor_with_divider(&p, &mut Bls12381FqDivider::new(oracle)),
            p.clear_cofactor(),
        )))
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_bls12_g2_is_in_subgroup(
    point: *const Bytes<192>,
    result: *mut bool,
) -> i32 {
    commit(result, || {
        let p = bls_g2(&read(point).data)?;
        Some(
            p.is_zero()
                || hinted!(
                    |oracle| bls::g2::is_in_subgroup_with_divider(
                        &p,
                        &mut Bls12381Fq2Divider::new(oracle)
                    ),
                    p.is_in_correct_subgroup_assuming_on_curve(),
                ),
        )
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level pointer contract.
pub unsafe extern "C" fn airbender_bls12_g2_clear_cofactor(
    point: *const Bytes<192>,
    result: *mut Bytes<192>,
) -> i32 {
    commit(result, || {
        let p = bls_g2(&read(point).data)?;
        if p.is_zero() {
            return Some(bls_out2(p));
        }
        Some(bls_out2(hinted!(
            |oracle| bls::g2::clear_cofactor_with_divider(&p, &mut Bls12381Fq2Divider::new(oracle)),
            p.clear_cofactor(),
        )))
    })
}
