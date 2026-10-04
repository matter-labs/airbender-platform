use super::*;
use crate::affine_glv::InvertingDivider;
use crate::bls12_381::eip2537::{map_fp2_to_g2, map_fp_to_g1};

#[test]
fn affine_wrappers_match_reference_and_exceptional_cases() {
    let generator = G2Affine::generator();
    let identity = G2Affine::identity();
    let raw = map_fp2_to_g2(Fq2::new(Fq::from(7u64), Fq::from(11u64))).unwrap();
    let points = [identity, generator, -generator, raw, -raw];
    for p in &points {
        for q in &points {
            assert_eq!(
                add_affine_with_divider(p, q, &mut InvertingDivider),
                (p.into_group() + q).into_affine()
            );
        }
        assert_eq!(
            is_in_subgroup_with_divider(p, &mut InvertingDivider),
            p.is_in_correct_subgroup_assuming_on_curve()
        );
        assert_eq!(
            clear_cofactor_with_divider(p, &mut InvertingDivider),
            p.clear_cofactor()
        );
        assert_eq!(
            into_affine_with_inverse(&p.into_group(), |z| z.inverse()),
            *p
        );
    }
    // The order-two branch y=0 does not divide by zero, even for unchecked points.
    let order_two = G2Affine::new_unchecked(Fq2::ONE, Fq2::ZERO);
    assert!(add_affine_with_divider(&order_two, &order_two, &mut InvertingDivider).infinity);
    assert!(is_in_subgroup_with_divider(
        &identity,
        &mut InvertingDivider
    ));
    assert!(!is_in_subgroup_with_divider(&raw, &mut InvertingDivider));
}

#[test]
fn cofactor_and_subgroup_match_for_uncleared_maps() {
    for i in 0..16u64 {
        let p = map_fp2_to_g2(Fq2::new(Fq::from(i), Fq::from(i * i))).unwrap();
        assert_eq!(
            is_in_subgroup_with_divider(&p, &mut InvertingDivider),
            p.is_in_correct_subgroup_assuming_on_curve()
        );
        let cleared = clear_cofactor_with_divider(&p, &mut InvertingDivider);
        assert_eq!(cleared, p.clear_cofactor());
        assert!(cleared.is_in_correct_subgroup_assuming_on_curve());
        let p = map_fp_to_g1(Fq::from(i)).unwrap();
        assert_eq!(
            g1::clear_cofactor_with_divider(&p, &mut InvertingDivider),
            p.clear_cofactor()
        );
        assert_eq!(
            g1::into_affine_with_inverse(&p.into_group(), |z| z.inverse()),
            p
        );
    }
}

#[test]
fn normalization_uses_checked_inverse_and_skips_infinity() {
    assert!(
        into_affine_with_inverse(&G2Projective::ZERO, |_| panic!("infinity needs no inverse"))
            .infinity
    );
    assert!(
        g1::into_affine_with_inverse(&G1Projective::ZERO, |_| panic!("infinity needs no inverse"))
            .infinity
    );
    let p = G2Affine::generator();
    let z = Fq2::new(Fq::from(3u64), Fq::from(2u64));
    let scaled = G2Projective::new_unchecked(p.x * z.square(), p.y * z.square() * z, z);
    assert_eq!(into_affine_with_inverse(&scaled, |z| z.inverse()), p);
    assert!(
        std::panic::catch_unwind(|| into_affine_with_inverse(&scaled, |_| Some(Fq2::ZERO)))
            .is_err()
    );
    let p = G1Affine::generator();
    let z = Fq::from(3u64);
    let scaled = G1Projective::new_unchecked(p.x * z.square(), p.y * z.square() * z, z);
    assert_eq!(g1::into_affine_with_inverse(&scaled, |z| z.inverse()), p);
    assert!(
        std::panic::catch_unwind(|| g1::into_affine_with_inverse(&scaled, |_| Some(Fq::ZERO)))
            .is_err()
    );
}

#[test]
fn normalization_of_z_one_issues_no_inverse_query() {
    let p = G2Affine::generator();
    assert_eq!(p.into_group().z, Fq2::ONE);
    assert_eq!(
        into_affine_with_inverse(&p.into_group(), |_| panic!("z = 1 needs no inverse")),
        p
    );
    let p = G1Affine::generator();
    assert_eq!(p.into_group().z, Fq::ONE);
    assert_eq!(
        g1::into_affine_with_inverse(&p.into_group(), |_| panic!("z = 1 needs no inverse")),
        p
    );
}

#[test]
fn extension_component_copy_and_initialization() {
    use crate::extension_tower::CopyAssign;
    let source = Fq2::new(Fq::from(19u64), Fq::from(23u64));
    let mut destination = Fq2::ZERO;
    destination.copy_assign(&source);
    assert_eq!(source, destination);
    let mut slot = core::mem::MaybeUninit::<Fq2>::uninit();
    unsafe {
        Fq2::init_copy(slot.as_mut_ptr(), &source);
        assert_eq!(slot.assume_init(), source);
    }
}
