// EIP-2537 helpers for BLS12-381 precompiles.
// These are defined in the crypto crate to avoid ICE when compiling for RISC-V.
// The issue is that functions in external crates that use Fq/G1/G2 types trigger
// compiler bugs during predicate checking for generic constants.

use super::curves::{g1, g2};
use super::{Fq, Fq2, G1Affine, G2Affine};
use crate::ark_ec::hashing::curve_maps::swu::SWUMap;
use crate::ark_ec::hashing::curve_maps::wb::{IsogenyMap, WBConfig};
use crate::ark_ec::hashing::map_to_curve_hasher::MapToCurve;
use crate::ark_ec::hashing::HashToCurveError;
use crate::ark_ec::models::short_weierstrass::SWCurveConfig;
use crate::ark_ec::short_weierstrass::Affine;
use crate::ark_ec::AffineRepr;
use crate::ark_ff::{AdditiveGroup, Field, PrimeField};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark_ec::hashing::curve_maps::wb::WBMap;
    use proptest::{prop_assert_eq, proptest};

    #[test]
    fn map_fp_to_g1_matches_arkworks() {
        proptest!(|(bytes: [u8; 48])| {
            let mut repr = <Fq as PrimeField>::BigInt::zero();
            for (dst, src) in repr.as_mut().iter_mut().zip(bytes.chunks_exact(8)) {
                *dst = u64::from_le_bytes(src.try_into().unwrap());
            }
            if let Some(element) = Fq::from_bigint(repr) {
                let ours = map_fp_to_g1(element).unwrap();
                let reference = WBMap::<g1::Config>::map_to_curve(element).unwrap();
                prop_assert_eq!(ours, reference);
            }
        })
    }

    #[test]
    fn apply_isogeny_map_zero_denominator() {
        // Craft an isogeny map where x_den is constant zero.
        // Montgomery's trick: prod = 0 * y_den = 0 → fallback path.
        let one = Fq::ONE;
        let zero = Fq::ZERO;

        // Use g1::Config as both Domain and Codomain (same BaseField).
        let isogeny = IsogenyMap::<g1::Config, g1::Config> {
            x_map_numerator: &[one],
            x_map_denominator: &[zero], // always evaluates to zero
            y_map_numerator: &[one],
            y_map_denominator: &[one],
        };

        let input = G1Affine::generator();
        let result = apply_isogeny_map(&isogeny, input).unwrap();
        // x_den=0 → x_den_inv=0 → img_x=0, but y_den=1 → img_y preserved
        assert_eq!(result.x, Fq::ZERO);
        assert_ne!(result.y, Fq::ZERO);

        // Both denominators zero
        let isogeny_both_zero = IsogenyMap::<g1::Config, g1::Config> {
            x_map_numerator: &[one],
            x_map_denominator: &[zero],
            y_map_numerator: &[one],
            y_map_denominator: &[zero],
        };
        let result = apply_isogeny_map(&isogeny_both_zero, input).unwrap();
        assert_eq!(result.x, Fq::ZERO);
        assert_eq!(result.y, Fq::ZERO);
    }

    #[test]
    fn map_fp2_to_g2_matches_arkworks() {
        proptest!(|(bytes: [u8; 96])| {
            let mut repr0 = <Fq as PrimeField>::BigInt::zero();
            let mut repr1 = <Fq as PrimeField>::BigInt::zero();
            for (dst, src) in repr0.as_mut().iter_mut().zip(bytes[..48].chunks_exact(8)) {
                *dst = u64::from_le_bytes(src.try_into().unwrap());
            }
            for (dst, src) in repr1.as_mut().iter_mut().zip(bytes[48..].chunks_exact(8)) {
                *dst = u64::from_le_bytes(src.try_into().unwrap());
            }
            if let (Some(c0), Some(c1)) = (Fq::from_bigint(repr0), Fq::from_bigint(repr1)) {
                let element = Fq2 { c0, c1 };
                let ours = map_fp2_to_g2(element).unwrap();
                let reference = WBMap::<g2::Config>::map_to_curve(element).unwrap();
                prop_assert_eq!(ours, reference);
            }
        })
    }
}
