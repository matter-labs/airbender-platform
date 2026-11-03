// EIP-2537 helpers for BLS12-381 precompiles.
// These are defined in the crypto crate to avoid ICE when compiling for RISC-V.
// The issue is that functions in external crates that use Fq/G1/G2 types trigger
// compiler bugs during predicate checking for generic constants.

use super::curves::{g1, g2};
use super::{Fq, Fq2, G1Affine, G2Affine};
use crate::ark_ec::hashing::curve_maps::parity;
use crate::ark_ec::hashing::curve_maps::swu::{SWUConfig, SWUMap};
use crate::ark_ec::hashing::curve_maps::wb::{IsogenyMap, WBConfig};
use crate::ark_ec::hashing::map_to_curve_hasher::MapToCurve;
use crate::ark_ec::hashing::HashToCurveError;
use crate::ark_ec::models::short_weierstrass::SWCurveConfig;
use crate::ark_ec::short_weierstrass::Affine;
use crate::ark_ec::AffineRepr;
use crate::ark_ff::{AdditiveGroup, Field, PrimeField, Zero};

const FIELD_ELEMENT_LEN: usize = 64;
const G1_LEN: usize = 128;
const G2_LEN: usize = 256;

#[inline(never)]
pub fn parse_fq_bytes(input: &[u8; FIELD_ELEMENT_LEN]) -> Option<Fq> {
    if input[..16].iter().all(|el| *el == 0) == false {
        return None;
    }
    let mut repr = <Fq as PrimeField>::BigInt::zero();
    let repr_slice = repr.as_mut();
    for (dst, src) in repr_slice.iter_mut().zip(input[16..].chunks_exact(8).rev()) {
        *dst = u64::from_be_bytes(src.try_into().unwrap());
    }
    Fq::from_bigint(repr)
}

#[inline(never)]
pub fn parse_fq2_bytes(input: &[u8; FIELD_ELEMENT_LEN * 2]) -> Option<Fq2> {
    let c0 = parse_fq_bytes(input[0..64].try_into().ok()?)?;
    let c1 = parse_fq_bytes(input[64..128].try_into().ok()?)?;
    Some(Fq2 { c0, c1 })
}

#[inline(never)]
pub fn parse_g1_bytes(input: &[u8; G1_LEN]) -> Option<(G1Affine, bool)> {
    if input.iter().all(|el| *el == 0) {
        return Some((G1Affine::identity(), false));
    }
    let x = parse_fq_bytes(input[0..64].try_into().ok()?)?;
    let y = parse_fq_bytes(input[64..128].try_into().ok()?)?;
    let point = G1Affine::new_unchecked(x, y);

    if !point.is_on_curve() {
        return None;
    }

    Some((point, true))
}

#[inline(never)]
pub fn parse_g2_bytes(input: &[u8; G2_LEN]) -> Option<(G2Affine, bool)> {
    if input.iter().all(|el| *el == 0) {
        return Some((G2Affine::identity(), false));
    }
    let x = parse_fq2_bytes(input[0..128].try_into().ok()?)?;
    let y = parse_fq2_bytes(input[128..256].try_into().ok()?)?;
    let point = G2Affine::new_unchecked(x, y);

    if !point.is_on_curve() {
        return None;
    }

    Some((point, true))
}

#[inline(never)]
pub fn serialize_fq_bytes(el: Fq, output: &mut [u8; FIELD_ELEMENT_LEN]) {
    output[..16].fill(0);
    let bigint = el.into_bigint();
    let words = bigint.as_ref();
    for (i, word) in words.iter().take(6).enumerate() {
        let bytes = word.to_be_bytes();
        let start = 16 + (5 - i) * 8;
        output[start..start + 8].copy_from_slice(&bytes);
    }
}

#[inline(never)]
pub fn serialize_fq2_bytes(el: Fq2, output: &mut [u8; FIELD_ELEMENT_LEN * 2]) {
    let (left, right) = output.split_at_mut(64);
    serialize_fq_bytes(el.c0, left.try_into().unwrap());
    serialize_fq_bytes(el.c1, right.try_into().unwrap());
}

#[inline(never)]
pub fn serialize_g1_bytes(el: G1Affine, output: &mut [u8; G1_LEN]) {
    if let Some((x, y)) = el.xy() {
        let (left, right) = output.split_at_mut(64);
        serialize_fq_bytes(x, left.try_into().unwrap());
        serialize_fq_bytes(y, right.try_into().unwrap());
    } else {
        output.fill(0);
    }
}

#[inline(never)]
pub fn serialize_g2_bytes(el: G2Affine, output: &mut [u8; G2_LEN]) {
    if let Some((x, y)) = el.xy() {
        let (left, right) = output.split_at_mut(128);
        serialize_fq2_bytes(x, left.try_into().unwrap());
        serialize_fq2_bytes(y, right.try_into().unwrap());
    } else {
        output.fill(0);
    }
}

// Heap-free reimplementation of arkworks' IsogenyMap::apply + polynomial evaluation.
// Original: https://github.com/arkworks-rs/algebra/blob/af564e48/ec/src/hashing/curve_maps/wb.rs#L42-L64
fn evaluate_polynomial<F: Field>(coeffs: &[F], x: &F) -> F {
    if coeffs.is_empty() {
        return F::ZERO;
    }
    if x.is_zero() {
        return coeffs[0];
    }
    coeffs
        .iter()
        .rfold(F::ZERO, |result, coeff| result * x + coeff)
}

// Heap-free `IsogenyMap::apply` using Horner evaluation + Montgomery's trick.
fn apply_isogeny_map<
    Domain: SWCurveConfig,
    Codomain: SWCurveConfig<BaseField = Domain::BaseField>,
>(
    map: &IsogenyMap<'_, Domain, Codomain>,
    domain_point: Affine<Domain>,
) -> Result<Affine<Codomain>, HashToCurveError> {
    match domain_point.xy() {
        Some((x, y)) => {
            let x_num = evaluate_polynomial(map.x_map_numerator, &x);
            let x_den = evaluate_polynomial(map.x_map_denominator, &x);
            let y_num = evaluate_polynomial(map.y_map_numerator, &x);
            let y_den = evaluate_polynomial(map.y_map_denominator, &x);

            let zero = Domain::BaseField::ZERO;
            let prod = x_den * y_den;
            let (x_den_inv, y_den_inv) = if let Some(prod_inv) = prod.inverse() {
                (y_den * prod_inv, x_den * prod_inv)
            } else {
                (
                    x_den.inverse().unwrap_or(zero),
                    y_den.inverse().unwrap_or(zero),
                )
            };
            let img_x = x_num * x_den_inv;
            let img_y = (y_num * y) * y_den_inv;
            Ok(Affine::<Codomain>::new_unchecked(img_x, img_y))
        }
        None => Ok(Affine::identity()),
    }
}

#[inline(never)]
pub fn map_fp_to_g1(element: Fq) -> Result<G1Affine, HashToCurveError> {
    let point_on_iso_curve =
        SWUMap::<<g1::Config as WBConfig>::IsogenousCurve>::map_to_curve(element)?;
    apply_isogeny_map(&g1::Config::ISOGENY_MAP, point_on_iso_curve)
}

#[inline(never)]
pub fn map_fp2_to_g2(element: Fq2) -> Result<G2Affine, HashToCurveError> {
    let point_on_iso_curve =
        SWUMap::<<g2::Config as WBConfig>::IsogenousCurve>::map_to_curve(element)?;
    apply_isogeny_map(&g2::Config::ISOGENY_MAP, point_on_iso_curve)
}

// A wrong hint fails an assertion: an error would let the prover fail a valid mapping.
fn checked_inverse<F: Field>(value: &F, inverse: &mut impl FnMut(&F) -> Option<F>) -> F {
    if value.is_zero() {
        // inv0: a zero denominator is deterministic and does not need advice.
        return F::ZERO;
    }
    let result = inverse(value).expect("invalid inverse hint");
    assert!(*value * result == F::ONE, "invalid inverse hint");
    result
}

fn checked_sqrt_ratio<F: Field>(
    value: &F,
    zeta: &F,
    sqrt_ratio: &mut impl FnMut(&F, &F) -> (F, bool),
) -> (F, bool) {
    let (root, is_square) = sqrt_ratio(value, zeta);
    let expected_square = if is_square { *value } else { *zeta * value };
    // Legendre(0) is Zero, not QR, in the reference's branch predicate.
    assert!(
        root.square() == expected_square && !(value.is_zero() && is_square),
        "invalid sqrt_ratio hint"
    );
    (root, is_square)
}

// The same inversion-avoiding SWU formulas as arkworks 0.5, with checked hints.
// A square root of v or ZETA*v proves the branch since ZETA is a fixed nonsquare.
fn swu_with_hints<P: SWUConfig>(
    element: P::BaseField,
    inverse: &mut impl FnMut(&P::BaseField) -> Option<P::BaseField>,
    sqrt_ratio: &mut impl FnMut(&P::BaseField, &P::BaseField) -> (P::BaseField, bool),
) -> Affine<P> {
    let a = P::COEFF_A;
    let b = P::COEFF_B;
    let zeta_u2 = P::ZETA * element.square();
    let ta = zeta_u2.square() + zeta_u2;
    let num_x1 = b * (ta + P::BaseField::ONE);
    let div = a * if ta.is_zero() { P::ZETA } else { -ta };
    let div_inv = checked_inverse(&div, inverse);
    let div2 = div.square();
    let div3 = div2 * div;
    let num_gx1 = (num_x1.square() + a * div2) * num_x1 + b * div3;
    let gx1 = num_gx1 * div_inv.square() * div_inv;
    let (y1, gx1_square) = checked_sqrt_ratio(&gx1, &P::ZETA, sqrt_ratio);
    let num_x = if gx1_square { num_x1 } else { zeta_u2 * num_x1 };
    let y = if gx1_square {
        y1
    } else {
        zeta_u2 * element * y1
    };
    let y = if parity(&y) == parity(&element) {
        y
    } else {
        -y
    };
    Affine::new_unchecked(num_x * div_inv, y)
}

fn apply_isogeny_map_with_inverse<
    Domain: SWCurveConfig,
    Codomain: SWCurveConfig<BaseField = Domain::BaseField>,
>(
    map: &IsogenyMap<'_, Domain, Codomain>,
    point: Affine<Domain>,
    inverse: &mut impl FnMut(&Domain::BaseField) -> Option<Domain::BaseField>,
) -> Affine<Codomain> {
    let Some((x, y)) = point.xy() else {
        return Affine::identity();
    };
    let x_num = evaluate_polynomial(map.x_map_numerator, &x);
    let x_den = evaluate_polynomial(map.x_map_denominator, &x);
    let y_num = evaluate_polynomial(map.y_map_numerator, &x);
    let y_den = evaluate_polynomial(map.y_map_denominator, &x);
    let prod = x_den * y_den;
    let (x_inv, y_inv) = if prod.is_zero() {
        (
            checked_inverse(&x_den, inverse),
            checked_inverse(&y_den, inverse),
        )
    } else {
        let prod_inv = checked_inverse(&prod, inverse);
        (y_den * prod_inv, x_den * prod_inv)
    };
    Affine::new_unchecked(x_num * x_inv, y_num * y * y_inv)
}

/// Map to G1 using checked inverse and square-root-ratio callbacks, before cofactor clearing.
/// `sqrt_ratio(v,z)` returns `(r,true)` with r²=v or `(r,false)` with r²=z*v.
/// For v=0 the boolean must be false, matching arkworks' Legendre convention.
/// Either sign of r is accepted: the map fixes the output sign deterministically.
/// Panics on a wrong hint; with valid hints the map has no failure case.
#[inline(never)]
pub fn map_fp_to_g1_with_hints(
    element: Fq,
    mut inverse: impl FnMut(&Fq) -> Option<Fq>,
    mut sqrt_ratio: impl FnMut(&Fq, &Fq) -> (Fq, bool),
) -> G1Affine {
    let point = swu_with_hints::<<g1::Config as WBConfig>::IsogenousCurve>(
        element,
        &mut inverse,
        &mut sqrt_ratio,
    );
    apply_isogeny_map_with_inverse(&g1::Config::ISOGENY_MAP, point, &mut inverse)
}

/// Map to G2 using the same checked callback contract as `map_fp_to_g1_with_hints`.
#[inline(never)]
pub fn map_fp2_to_g2_with_hints(
    element: Fq2,
    mut inverse: impl FnMut(&Fq2) -> Option<Fq2>,
    mut sqrt_ratio: impl FnMut(&Fq2, &Fq2) -> (Fq2, bool),
) -> G2Affine {
    let point = swu_with_hints::<<g2::Config as WBConfig>::IsogenousCurve>(
        element,
        &mut inverse,
        &mut sqrt_ratio,
    );
    apply_isogeny_map_with_inverse(&g2::Config::ISOGENY_MAP, point, &mut inverse)
}

#[cfg(test)]
mod tests;
