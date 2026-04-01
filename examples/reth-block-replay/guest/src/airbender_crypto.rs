use airbender::crypto::ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use airbender::crypto::ark_ff::{One, PrimeField, Zero};
use airbender::crypto::bn254::{
    curves::Bn254, Fq as BnFq, Fq2 as BnFq2, Fr as BnFr, G1Affine as BnG1Affine,
    G1Projective as BnG1Projective, G2Affine as BnG2Affine,
};
use airbender::crypto::sha3::Keccak256;
use airbender::crypto::MiniDigest;
use revm_precompile::{Crypto, PrecompileError};
use std::vec::Vec;

#[derive(Debug, Clone)]
pub struct AirbenderCrypto;

#[inline(never)]
fn airbender_ecrecover(
    sig: &[u8; 64],
    recid: u8,
    msg: &[u8; 32],
) -> Result<[u8; 32], PrecompileError> {
    use airbender::crypto::k256::ecdsa::{hazmat::bits2field, RecoveryId, Signature};
    use airbender::crypto::k256::elliptic_curve::ops::Reduce;
    use airbender::crypto::k256::{Scalar, Secp256k1, U256};

    let mut signature =
        Signature::from_slice(sig).map_err(|_| PrecompileError::Secp256k1RecoverFailed)?;

    let mut rec = recid;
    if let Some(normalized) = signature.normalize_s() {
        signature = normalized;
        rec ^= 1;
    }

    let recovery_id = RecoveryId::from_byte(rec).ok_or(PrecompileError::Secp256k1RecoverFailed)?;

    let field_bytes =
        bits2field::<Secp256k1>(msg).map_err(|_| PrecompileError::Secp256k1RecoverFailed)?;
    let message = <Scalar as Reduce<U256>>::reduce_bytes(&field_bytes);

    let recovered = airbender::crypto::secp256k1::recover(&message, &signature, &recovery_id)
        .map_err(|_| PrecompileError::Secp256k1RecoverFailed)?;

    // Uncompressed pubkey: 04 || x(32) || y(32). Hash bytes[1..] with keccak256.
    let encoded = recovered.to_encoded_point(false);
    let hash = Keccak256::digest(&encoded.as_bytes()[1..]);

    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    // EVM ecrecover: zero first 12 bytes, last 20 = Ethereum address
    result[..12].fill(0);

    Ok(result)
}

const BN_FQ_LEN: usize = 32;

/// 32-byte big-endian → BN254 Fq
#[inline(never)]
fn read_bn_fq(input: &[u8]) -> Result<BnFq, PrecompileError> {
    debug_assert_eq!(input.len(), BN_FQ_LEN);

    let mut repr = <BnFq as PrimeField>::BigInt::zero();
    let words = repr.as_mut();
    for (dst, src) in words.iter_mut().zip(input.chunks_exact(8).rev()) {
        *dst = u64::from_be_bytes(src.try_into().unwrap());
    }

    BnFq::from_bigint(repr).ok_or(PrecompileError::Bn254FieldPointNotAMember)
}

/// 64-byte → BN254 Fq2. EVM layout: imaginary (c1) first, then real (c0).
#[inline(never)]
fn read_bn_fq2(input: &[u8]) -> Result<BnFq2, PrecompileError> {
    let c1 = read_bn_fq(&input[..BN_FQ_LEN])?;
    let c0 = read_bn_fq(&input[BN_FQ_LEN..2 * BN_FQ_LEN])?;
    Ok(BnFq2::new(c0, c1))
}

/// 64-byte → G1 affine. (0,0) = point at infinity.
#[inline(never)]
fn read_bn_g1(input: &[u8]) -> Result<BnG1Affine, PrecompileError> {
    let px = read_bn_fq(&input[..BN_FQ_LEN])?;
    let py = read_bn_fq(&input[BN_FQ_LEN..2 * BN_FQ_LEN])?;

    if px.is_zero() && py.is_zero() {
        return Ok(BnG1Affine::zero());
    }

    let point = BnG1Affine::new_unchecked(px, py);
    if !point.is_on_curve() || !point.is_in_correct_subgroup_assuming_on_curve() {
        return Err(PrecompileError::Bn254AffineGFailedToCreate);
    }
    Ok(point)
}

/// 128-byte → G2 affine. All-zeros = point at infinity.
#[inline(never)]
fn read_bn_g2(input: &[u8]) -> Result<BnG2Affine, PrecompileError> {
    let x = read_bn_fq2(&input[..2 * BN_FQ_LEN])?;
    let y = read_bn_fq2(&input[2 * BN_FQ_LEN..4 * BN_FQ_LEN])?;

    if x.is_zero() && y.is_zero() {
        return Ok(BnG2Affine::zero());
    }

    let point = BnG2Affine::new_unchecked(x, y);
    if !point.is_on_curve() || !point.is_in_correct_subgroup_assuming_on_curve() {
        return Err(PrecompileError::Bn254AffineGFailedToCreate);
    }
    Ok(point)
}

#[inline(never)]
fn read_bn_scalar(input: &[u8]) -> BnFr {
    BnFr::from_be_bytes_mod_order(input)
}

/// G1 affine → 64-byte big-endian. Infinity → all zeros.
#[inline(never)]
fn encode_bn_g1(point: BnG1Affine) -> [u8; 64] {
    let mut output = [0u8; 64];
    if point.is_zero() {
        return output;
    }

    fn write_fq_be(fq: &BnFq, dst: &mut [u8]) {
        let bigint = fq.clone().into_bigint();
        let words: &[u64] = bigint.as_ref();
        for (word, chunk) in words.iter().zip(dst.chunks_exact_mut(8).rev()) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
    }

    let (x, y) = point.xy().unwrap();
    write_fq_be(&x, &mut output[..32]);
    write_fq_be(&y, &mut output[32..]);

    output
}

impl Crypto for AirbenderCrypto {
    #[inline(never)]
    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], PrecompileError> {
        airbender_ecrecover(sig, recid, msg)
    }

    #[inline(never)]
    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let a = read_bn_g1(p1)?;
        let b = read_bn_g1(p2)?;
        let result = (BnG1Projective::from(a) + b).into_affine();
        Ok(encode_bn_g1(result))
    }

    #[inline(never)]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let p = read_bn_g1(point)?;
        let s = read_bn_scalar(scalar);
        let result = p.mul_bigint(s.into_bigint()).into_affine();
        Ok(encode_bn_g1(result))
    }

    #[inline(never)]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
        let mut g1_points = Vec::with_capacity(pairs.len());
        let mut g2_points = Vec::with_capacity(pairs.len());

        for (g1_bytes, g2_bytes) in pairs {
            let g1 = read_bn_g1(g1_bytes)?;
            let g2 = read_bn_g2(g2_bytes)?;

            if !g1.is_zero() && !g2.is_zero() {
                g1_points.push(g1);
                g2_points.push(g2);
            }
        }

        if g1_points.is_empty() {
            return Ok(true);
        }

        let result = Bn254::multi_pairing(&g1_points, &g2_points);
        Ok(result.0.is_one())
    }
}
