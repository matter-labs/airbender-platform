//! The prover's side of the witnessed recovery (`recover_witnessed`): the key by the plain
//! recovery, and the coefficients by lattice reduction (the integral LLL of Cohen, Algorithm
//! 2.6.7, in dimension 4). Host only.

use super::context::ECRECOVER_CONTEXT;
use super::hooks::{DefaultSecp256k1Hooks, RecoveryWitness};
use super::points::Affine;
use super::recover::compute_from_point;
use super::recover_witnessed::WITNESS_COEFFICIENT_BITS;
use super::scalars::Scalar;
// the crate links `alloc` only with its `alloc` feature; this host-only module (where the
// final binary has an allocator) links it on its own
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use num_bigint::{BigInt, Sign};
use num_traits::{One, Signed, ToPrimitive, Zero};

/// The witness for the signature `(r, s)` of `message` (reduced) with the point `point`, as the
/// `Secp256k1Hooks::recovery_witness` hook of a prover answers: `None` when the recovered key is
/// the point at infinity (the recovery fails then, computed), or when no short enough vector
/// was found (it always is: the bound is well above the Minkowski bound)
pub fn recovery_witness(
    point: &Affine,
    r: &Scalar,
    s: &Scalar,
    message: &Scalar,
) -> Option<RecoveryWitness> {
    let key = compute_from_point(
        point,
        *r,
        *s,
        *message,
        &ECRECOVER_CONTEXT,
        &mut DefaultSecp256k1Hooks,
    )
    .ok()?;
    let mut key_x = [0u8; 32];
    let mut key_y = [0u8; 32];
    key.write_coordinates(&mut key_x, &mut key_y);
    Some(RecoveryWitness {
        key_x,
        key_y,
        coefficients: decompose_4d(r, s)?,
    })
}

fn big(scalar: &Scalar) -> BigInt {
    let bytes: [u8; 32] = scalar.to_repr().into();
    BigInt::from_bytes_be(Sign::Plus, &bytes)
}

/// A short non-zero vector `(a, b, c, d)` with `(a + b λ) r ≡ (c + d λ) s (mod n)`, every
/// coordinate within `WITNESS_COEFFICIENT_BITS` bits
pub fn decompose_4d(r: &Scalar, s: &Scalar) -> Option<[i128; 4]> {
    // the order: minus one, plus one
    let n = big(&-Scalar::ONE) + BigInt::one();
    let lambda = big(&-Scalar::MINUS_LAMBDA);
    // t = r / s
    let mut t = *r;
    t *= s.invert_vartime();
    let t = big(&t);
    // the lattice: c ≡ a t + b λ t - d λ (mod n)
    let lambda_t = (&lambda * &t) % &n;
    let minus_lambda = &n - &lambda;
    let basis = vec![
        vec![BigInt::one(), BigInt::zero(), t, BigInt::zero()],
        vec![BigInt::zero(), BigInt::one(), lambda_t, BigInt::zero()],
        vec![BigInt::zero(), BigInt::zero(), n, BigInt::zero()],
        vec![BigInt::zero(), BigInt::zero(), minus_lambda, BigInt::one()],
    ];
    let reduced = lll(basis);
    let shortest = reduced
        .iter()
        .filter(|v| v.iter().any(|x| !x.is_zero()))
        .min_by_key(|v| v.iter().map(|x| x.abs()).max().unwrap())?;
    let bound = BigInt::one() << WITNESS_COEFFICIENT_BITS;
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
