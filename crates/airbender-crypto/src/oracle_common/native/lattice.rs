//! The short vectors that witnessed scalar multiplications are checked with (the secp256k1
//! recovery in `secp256k1::recover_witnessed`, the multiplications of `affine_glv::check_mul_witness`): for a
//! group of prime order `n` with an endomorphism of eigenvalue `λ`, a short non-zero vector of
//! the lattice `{(a, b, c, d) : (a + b λ) t ≡ c + d λ (mod n)}`, by lattice reduction (the
//! integral LLL of Cohen, Algorithm 2.6.7, in dimension 4). Host only.

use crate::affine_glv::{MulWitness, WITNESS_COEFFICIENT_BITS};
use alloc::vec;
use alloc::vec::Vec;
use ark_ec::scalar_mul::glv::GLVConfig;
use ark_ec::short_weierstrass::{Affine, SWCurveConfig};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInteger, PrimeField};
use num_bigint::{BigInt, Sign};
use num_traits::{One, Signed, ToPrimitive, Zero};

/// The witness of `k p` for the check of `affine_glv::check_mul_witness`: the product by the
/// plain multiplication, and a short vector of the lattice of `k`. `None` (the querier computes
/// the product then) if the product is the point at infinity, or if no short enough vector was
/// found (it always is: the bound is well above the Minkowski bound).
pub fn mul_witness<C: SWCurveConfig + GLVConfig>(
    p: &Affine<C>,
    k: C::ScalarField,
) -> Option<MulWitness<C::BaseField>> {
    let product = p.mul_bigint(k.into_bigint()).into_affine();
    if product.infinity {
        return None;
    }
    let big = |limbs: <C::ScalarField as PrimeField>::BigInt| {
        BigInt::from_bytes_le(Sign::Plus, &limbs.to_bytes_le())
    };
    let coefficients = short_vector_4d(
        &big(k.into_bigint()),
        &big(C::LAMBDA.into_bigint()),
        &big(C::ScalarField::MODULUS),
        WITNESS_COEFFICIENT_BITS,
    )?;
    Some(MulWitness {
        x: product.x,
        y: product.y,
        coefficients,
    })
}

/// A non-zero vector `(a, b, c, d)` with `(a + b λ) t ≡ c + d λ (mod n)` and every coordinate
/// below `2^bits` in absolute value, for `t` and `lambda` reduced modulo `n`; `None` if the
/// reduction finds none that short. The lattice has determinant `n`, so it has a vector with
/// coordinates up to `n^(1/4)`, and the reduced basis starts with one within `2^(3/4)` of
/// that in norm: for a 256-bit `n`, coordinates below `2^65`.
pub fn short_vector_4d(t: &BigInt, lambda: &BigInt, n: &BigInt, bits: usize) -> Option<[i128; 4]> {
    // the lattice: c ≡ a t + b λ t - d λ (mod n)
    let lambda_t = (lambda * t) % n;
    let minus_lambda = n - lambda;
    let basis = vec![
        vec![BigInt::one(), BigInt::zero(), t.clone(), BigInt::zero()],
        vec![BigInt::zero(), BigInt::one(), lambda_t, BigInt::zero()],
        vec![BigInt::zero(), BigInt::zero(), n.clone(), BigInt::zero()],
        vec![BigInt::zero(), BigInt::zero(), minus_lambda, BigInt::one()],
    ];
    let reduced = lll(basis);
    let shortest = reduced
        .iter()
        .filter(|v| v.iter().any(|x| !x.is_zero()))
        .min_by_key(|v| v.iter().map(|x| x.abs()).max().unwrap())?;
    let bound = BigInt::one() << bits;
    if shortest.iter().any(|x| x.abs() >= bound) {
        return None;
    }
    Some([
        shortest[0].to_i128()?,
        shortest[1].to_i128()?,
        shortest[2].to_i128()?,
        shortest[3].to_i128()?,
    ])
}

/// Floor division
fn div_floor(a: &BigInt, b: &BigInt) -> BigInt {
    let (q, rem) = (a / b, a % b);
    if !rem.is_zero() && (rem.is_negative() != b.is_negative()) {
        q - BigInt::one()
    } else {
        q
    }
}

/// The nearest integer to `a / b` for `b > 0`
fn div_round(a: &BigInt, b: &BigInt) -> BigInt {
    div_floor(&(a * 2u32 + b), &(b * 2u32))
}

/// An LLL-reduced basis of the lattice spanned by `b` (linearly independent integer vectors):
/// Cohen, A Course in Computational Algebraic Number Theory, Algorithm 2.6.7 (integral LLL),
/// with `delta = 3/4`. `d[i]` is the Gram determinant of the first `i` vectors (`d[0] = 1`),
/// `lambda[k][j]` the scaled Gram-Schmidt coefficient `mu[k][j] d[j + 1]`.
// the index loops follow the algorithm's statement
#[allow(clippy::needless_range_loop)]
fn lll(mut b: Vec<Vec<BigInt>>) -> Vec<Vec<BigInt>> {
    let n = b.len();
    let dot = |x: &[BigInt], y: &[BigInt]| -> BigInt { x.iter().zip(y).map(|(p, q)| p * q).sum() };
    let mut d = vec![BigInt::zero(); n + 1];
    d[0] = BigInt::one();
    d[1] = dot(&b[0], &b[0]);
    let mut lambda = vec![vec![BigInt::zero(); n]; n];
    let mut k = 1usize;
    let mut kmax = 0usize;

    fn red(k: usize, l: usize, b: &mut [Vec<BigInt>], d: &[BigInt], lambda: &mut [Vec<BigInt>]) {
        if (&lambda[k][l] * 2u32).abs() > d[l + 1] {
            let q = div_round(&lambda[k][l], &d[l + 1]);
            for i in 0..b[k].len() {
                let subtrahend = &q * &b[l][i];
                b[k][i] -= subtrahend;
            }
            let subtrahend = &q * &d[l + 1];
            lambda[k][l] -= subtrahend;
            for i in 0..l {
                let subtrahend = &q * &lambda[l][i];
                lambda[k][i] -= subtrahend;
            }
        }
    }

    while k < n {
        if k > kmax {
            kmax = k;
            for j in 0..=k {
                let mut u = dot(&b[k], &b[j]);
                for i in 0..j {
                    u = (&d[i + 1] * &u - &lambda[k][i] * &lambda[j][i]) / &d[i];
                }
                if j < k {
                    lambda[k][j] = u;
                } else {
                    assert!(!u.is_zero(), "the vectors are linearly dependent");
                    d[k + 1] = u;
                }
            }
        }
        loop {
            red(k, k - 1, &mut b, &d, &mut lambda);
            let lhs = &d[k + 1] * &d[k - 1] * 4u32;
            let rhs = &d[k] * &d[k] * 3u32 - &lambda[k][k - 1] * &lambda[k][k - 1] * 4u32;
            if lhs < rhs {
                // swap b[k] and b[k - 1], and update the Gram-Schmidt data
                b.swap(k, k - 1);
                for j in 0..k - 1 {
                    let t = lambda[k][j].clone();
                    lambda[k][j] = lambda[k - 1][j].clone();
                    lambda[k - 1][j] = t;
                }
                let lam = lambda[k][k - 1].clone();
                let big_b = (&d[k - 1] * &d[k + 1] + &lam * &lam) / &d[k];
                for i in k + 1..=kmax {
                    let t = lambda[i][k].clone();
                    lambda[i][k] = (&d[k + 1] * &lambda[i][k - 1] - &lam * &t) / &d[k];
                    lambda[i][k - 1] = (&big_b * &t + &lam * &lambda[i][k]) / &d[k + 1];
                }
                d[k] = big_b;
                k = k.max(2) - 1;
            } else {
                break;
            }
        }
        for l in (0..k.saturating_sub(1)).rev() {
            red(k, l, &mut b, &d, &mut lambda);
        }
        k += 1;
    }
    b
}
