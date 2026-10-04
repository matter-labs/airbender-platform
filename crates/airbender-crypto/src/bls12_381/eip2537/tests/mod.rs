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

fn sqrt_ratio<F: Field>(v: &F, z: &F) -> (F, bool) {
    if !v.is_zero() {
        if let Some(root) = v.sqrt() {
            return (root, true);
        }
    }
    ((*z * v).sqrt().unwrap(), false)
}

#[test]
fn checked_maps_match_reference_boundaries_and_root_signs() {
    let values = [Fq::ZERO, Fq::ONE, -Fq::ONE, Fq::from(2u64), Fq::from(17u64)];
    for v in values {
        assert_eq!(
            map_fp_to_g1_with_hints(v, |v| v.inverse(), sqrt_ratio),
            map_fp_to_g1(v).unwrap()
        );
        assert_eq!(
            map_fp_to_g1_with_hints(
                v,
                |v| v.inverse(),
                |v, z| {
                    let (r, b) = sqrt_ratio(v, z);
                    (-r, b)
                }
            ),
            map_fp_to_g1(v).unwrap()
        );
        for w in values {
            let v = Fq2::new(v, w);
            assert_eq!(
                map_fp2_to_g2_with_hints(v, |v| v.inverse(), sqrt_ratio),
                map_fp2_to_g2(v).unwrap()
            );
            assert_eq!(
                map_fp2_to_g2_with_hints(
                    v,
                    |v| v.inverse(),
                    |v, z| {
                        let (r, b) = sqrt_ratio(v, z);
                        (-r, b)
                    }
                ),
                map_fp2_to_g2(v).unwrap()
            );
        }
    }
}

#[test]
fn checked_maps_match_reference_random() {
    proptest!(|(bytes: [u8; 96])| {
        // Build canonical padded BigInts directly: the delegated Fq's generic
        // from_le_bytes_mod_order path assumes an unpadded serialization buffer.
        let canonical = |bytes: &[u8]| {
            let mut repr = <Fq as PrimeField>::BigInt::zero();
            for (dst,src) in repr.as_mut().iter_mut().zip(bytes.chunks_exact(8)) {
                *dst = u64::from_le_bytes(src.try_into().unwrap());
            }
            repr.as_mut()[5] &= 0x1fff_ffff_ffff_ffff;
            Fq::from_bigint(repr)
        };
        let v = canonical(&bytes[..48]);
        let w = canonical(&bytes[48..]);
        proptest::prop_assume!(v.is_some() && w.is_some());
        let (v,w) = (v.unwrap(),w.unwrap());
        prop_assert_eq!(map_fp_to_g1_with_hints(v, |v| v.inverse(), sqrt_ratio), map_fp_to_g1(v).unwrap());
        let v = Fq2::new(v,w);
        prop_assert_eq!(map_fp2_to_g2_with_hints(v, |v| v.inverse(), sqrt_ratio), map_fp2_to_g2(v).unwrap());
    });
}

// A wrong hint of either callback fails an assertion, so a prover cannot turn a valid mapping
// into a failed call.
#[test]
#[should_panic(expected = "invalid inverse hint")]
fn checked_map_panics_on_invalid_inverse_hint() {
    let _ = map_fp_to_g1_with_hints(Fq::ONE, |_| Some(Fq::ONE + Fq::ONE), sqrt_ratio);
}

#[test]
#[should_panic(expected = "invalid sqrt_ratio hint")]
fn checked_map_panics_on_invalid_sqrt_ratio_hint() {
    let _ = map_fp2_to_g2_with_hints(
        Fq2::new(Fq::ONE, Fq::ONE),
        |v| v.inverse(),
        |v, z| {
            let (r, b) = sqrt_ratio(v, z);
            (r + Fq2::ONE, b)
        },
    );
}

#[test]
fn hinted_isogeny_preserves_zero_denominators_and_infinity() {
    let one = Fq::ONE;
    let zero = Fq::ZERO;
    for y_den in [&[one][..], &[zero][..]] {
        let map = IsogenyMap::<g1::Config, g1::Config> {
            x_map_numerator: &[one],
            x_map_denominator: &[zero],
            y_map_numerator: &[one],
            y_map_denominator: y_den,
        };
        let p = G1Affine::generator();
        assert_eq!(
            apply_isogeny_map_with_inverse(&map, p, &mut |v| v.inverse()),
            apply_isogeny_map(&map, p).unwrap()
        );
        assert!(
            apply_isogeny_map_with_inverse(&map, G1Affine::identity(), &mut |_| panic!(
                "infinity needs no hint"
            ))
            .infinity
        );
    }
}

#[test]
fn checked_zero_root_preserves_legendre_branch() {
    let zeta = <<g1::Config as WBConfig>::IsogenousCurve as SWUConfig>::ZETA;
    assert_eq!(
        checked_sqrt_ratio(&Fq::ZERO, &zeta, &mut sqrt_ratio),
        (Fq::ZERO, false)
    );
    let zeta = <<g2::Config as WBConfig>::IsogenousCurve as SWUConfig>::ZETA;
    assert_eq!(
        checked_sqrt_ratio(&Fq2::ZERO, &zeta, &mut sqrt_ratio),
        (Fq2::ZERO, false)
    );
}

// The root 0 squares to both 0 and ZETA * 0, so only the flag can be wrong here.
#[test]
#[should_panic(expected = "invalid sqrt_ratio hint")]
fn checked_zero_root_panics_on_square_flag() {
    let zeta = <<g1::Config as WBConfig>::IsogenousCurve as SWUConfig>::ZETA;
    checked_sqrt_ratio(&Fq::ZERO, &zeta, &mut |_, _| (Fq::ZERO, true));
}
