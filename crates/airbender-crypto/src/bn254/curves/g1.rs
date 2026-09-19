#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
use crate::ark_ff_delegation::BigInt as ScalarBigInt;
#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
use crate::ark_ff_delegation::MontFp;
use ark_ec::{
    bn,
    models::{short_weierstrass::SWCurveConfig, CurveConfig},
    scalar_mul::glv::GLVConfig,
    short_weierstrass::{Affine, Projective},
    AffineRepr,
};
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
use ark_ff::BigInt as ScalarBigInt;
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
use ark_ff::MontFp;
use ark_ff::{AdditiveGroup, Field, PrimeField, Zero};
use ruint::aliases::U512;

use crate::{
    bn254::fields::{Fq, Fr},
    glv_decomposition::GLVConfigNoAllocator,
};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Config;

pub type G1Affine = Affine<Config>;

impl CurveConfig for Config {
    type BaseField = Fq;
    type ScalarField = Fr;

    /// COFACTOR = 1
    const COFACTOR: &'static [u64] = &[0x1];

    /// COFACTOR_INV = COFACTOR^{-1} mod r = 1
    const COFACTOR_INV: Fr = Fr::ONE;
}

impl SWCurveConfig for Config {
    /// COEFF_A = 0
    const COEFF_A: Fq = Fq::ZERO;

    /// COEFF_B = 3
    const COEFF_B: Fq = MontFp!("3");

    /// AFFINE_GENERATOR_COEFFS = (G1_GENERATOR_X, G1_GENERATOR_Y)
    const GENERATOR: G1Affine = G1Affine::new_unchecked(G1_GENERATOR_X, G1_GENERATOR_Y);

    #[inline(always)]
    fn mul_by_a(_: Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }

    #[inline]
    fn mul_projective(
        p: &bn::G1Projective<super::Config>,
        scalar: &[u64],
    ) -> bn::G1Projective<super::Config> {
        let s = Self::ScalarField::from_sign_and_limbs(true, scalar);
        GLVConfig::glv_mul_projective(*p, s)
    }

    #[inline]
    fn mul_affine(base: &Affine<Self>, scalar: &[u64]) -> bn::G1Projective<super::Config> {
        Self::mul_projective(&base.into_group(), scalar)
    }

    #[inline]
    fn is_in_correct_subgroup_assuming_on_curve(_p: &G1Affine) -> bool {
        // G1 = E(Fq) so if the point is on the curve, it is also in the subgroup.
        true
    }
}

impl GLVConfig for Config {
    const ENDO_COEFFS: &'static [Self::BaseField] = &[MontFp!(
        "21888242871839275220042445260109153167277707414472061641714758635765020556616"
    )];

    // 21888242871839275217838484774961031246154997185409878258781734729429964517155,
    // built from limbs so that it types as either scalar field implementation
    const LAMBDA: Self::ScalarField = Fr::from_sign_and_limbs(
        true,
        &[
            13315467537088212771,
            14715414200443454953,
            327476452638867716,
            3486998266802970665,
        ],
    );

    const SCALAR_DECOMP_COEFFS: [(bool, <Self::ScalarField as PrimeField>::BigInt); 4] = [
        // 147946756881789319000765030803803410728
        (
            false,
            ScalarBigInt([9372478919628755240, 8020209761171036668, 0, 0]),
        ),
        // 9931322734385697763
        (true, ScalarBigInt([9931322734385697763, 0, 0, 0])),
        (false, ScalarBigInt([9931322734385697763, 0, 0, 0])),
        // 147946756881789319010696353538189108491
        (
            false,
            ScalarBigInt([857057580304901387, 8020209761171036669, 0, 0]),
        ),
    ];

    fn endomorphism(p: &Projective<Self>) -> Projective<Self> {
        let mut res = (*p).clone();
        res.x *= Self::ENDO_COEFFS[0];
        res
    }
    fn endomorphism_affine(p: &Affine<Self>) -> Affine<Self> {
        let mut res = (*p).clone();
        res.x *= Self::ENDO_COEFFS[0];
        res
    }

    fn glv_mul_projective(p: Projective<Self>, k: Self::ScalarField) -> Projective<Self> {
        crate::glv_decomposition::glv_mul_projective_jsf::<Self>(p, k)
    }

    fn scalar_decomposition(
        k: Self::ScalarField,
    ) -> ((bool, Self::ScalarField), (bool, Self::ScalarField)) {
        Self::scalar_decomposition_no_allocator(k)
    }
}

impl GLVConfigNoAllocator for Config {
    const BETA_1: (bool, U512) = (
        false,
        U512::from_limbs([
            7440537858994729442,
            12177485554411886469,
            1601953548471081566,
            1485435879091901900,
            6023842690951505253,
            5534624963584316114,
            2,
            0,
        ]),
    );

    const BETA_2: (bool, U512) = (
        false,
        U512::from_limbs([
            10866705332225114937,
            3332646303595026058,
            10351474459561409124,
            7978627105577135858,
            15644699364383830999,
            2,
            0,
            0,
        ]),
    );
}

/// G1_GENERATOR_X = 1
pub const G1_GENERATOR_X: Fq = Fq::ONE;

/// G1_GENERATOR_Y = 2
pub const G1_GENERATOR_Y: Fq = MontFp!("2");

#[cfg(test)]
mod tests {
    use super::GLVConfigNoAllocator;
    use super::{Config, CurveConfig, GLVConfig, PrimeField};
    use proptest::{prop_assert_eq, proptest};
    type ScalarField = <Config as CurveConfig>::ScalarField;

    #[test]
    fn compare_scalar_decomposition() {
        proptest!(|(bytes: [u8; 32])| {
            let k = ScalarField::from_be_bytes_mod_order(&bytes);

            let (k1, k2) = Config::scalar_decomposition(k.clone());
            let (k1_ref, k2_ref) = Config::scalar_decomposition_ref(k);

            prop_assert_eq!(k1, k1_ref);
            prop_assert_eq!(k2, k2_ref);
        })
    }

    #[test]
    fn test_betas() {
        use ark_std::ops::Neg;
        use num_bigint::{BigInt, BigUint, Sign};
        use num_integer::Integer;
        use ruint::aliases::U512;

        let coeff_bigints: [BigInt; 4] = Config::SCALAR_DECOMP_COEFFS.map(|x| {
            BigInt::from_biguint(x.0.then_some(Sign::Plus).unwrap_or(Sign::Minus), x.1.into())
        });

        let [_, n12, _, n22] = coeff_bigints;

        let n = 512u64;
        let r = BigInt::from(<<Config as CurveConfig>::ScalarField>::MODULUS);

        let beta_1_ref = (n22 << n).div_rem(&r).0;

        let sign = Config::BETA_1
            .0
            .then_some(Sign::Plus)
            .unwrap_or(Sign::Minus);
        let data = BigUint::from_bytes_be(&Config::BETA_1.1.to_be_bytes::<{ U512::BYTES }>());
        let beta_1 = BigInt::from_biguint(sign, data);
        assert_eq!(beta_1, beta_1_ref);

        let beta_2_ref = ((n12 << n).neg()).div_rem(&r).0;

        let sign = Config::BETA_2
            .0
            .then_some(Sign::Plus)
            .unwrap_or(Sign::Minus);
        let data = BigUint::from_bytes_be(&Config::BETA_2.1.to_be_bytes::<{ U512::BYTES }>());
        let beta_2 = BigInt::from_biguint(sign, data);
        assert_eq!(beta_2, beta_2_ref);
    }
}

#[cfg(test)]
mod mul_tests {
    use super::{Config, G1Affine};
    use ark_ec::{models::short_weierstrass::SWCurveConfig, AffineRepr, CurveGroup};
    use ark_ff::{BigInteger, PrimeField, UniformRand};

    type RefAffine = ark_bn254::G1Affine;

    fn to_ours(p: RefAffine) -> G1Affine {
        if p.infinity {
            return G1Affine::identity();
        }
        let x =
            crate::bn254::Fq::from_bigint(crate::ark_ff_delegation::BigInt(p.x.into_bigint().0))
                .unwrap();
        let y =
            crate::bn254::Fq::from_bigint(crate::ark_ff_delegation::BigInt(p.y.into_bigint().0))
                .unwrap();
        G1Affine::new_unchecked(x, y)
    }

    fn assert_same(ours: G1Affine, reference: RefAffine) {
        assert_eq!(ours.infinity, reference.infinity);
        if !ours.infinity {
            assert_eq!(ours.x.into_bigint().0, reference.x.into_bigint().0);
            assert_eq!(ours.y.into_bigint().0, reference.y.into_bigint().0);
        }
    }

    #[test]
    fn scalar_multiplication_matches_the_reference() {
        use ark_std::test_rng;
        let mut rng = test_rng();
        let r = ark_bn254::Fr::MODULUS;
        let mut r_minus_1 = r;
        r_minus_1.sub_with_borrow(&ark_ff::BigInt::from(1u64));
        let mut r_plus_1 = r;
        r_plus_1.add_with_carry(&ark_ff::BigInt::from(1u64));
        let mut scalars: Vec<[u64; 4]> = vec![
            [0, 0, 0, 0],
            [1, 0, 0, 0],
            [2, 0, 0, 0],
            [3, 0, 0, 0],
            r_minus_1.0,
            r.0,
            r_plus_1.0,
            [u64::MAX; 4],
            [u64::MAX, u64::MAX, 0, 0],
        ];
        for _ in 0..40 {
            scalars.push(ark_bn254::Fr::rand(&mut rng).into_bigint().0);
            scalars.push([
                u64::rand(&mut rng),
                u64::rand(&mut rng),
                u64::rand(&mut rng),
                u64::rand(&mut rng),
            ]);
        }
        let mut points: Vec<RefAffine> = vec![RefAffine::identity(), RefAffine::generator()];
        for _ in 0..6 {
            points.push(RefAffine::rand(&mut rng));
        }
        for reference in &points {
            let ours = to_ours(*reference);
            for scalar in &scalars {
                let expected = ark_bn254::g1::Config::mul_affine(reference, scalar).into_affine();
                let got = Config::mul_affine(&ours, scalar).into_affine();
                assert_same(got, expected);
            }
        }
    }
}
