use ark_ec::{
    bls12,
    bls12::Bls12Config,
    hashing::curve_maps::wb::{IsogenyMap, WBConfig},
    models::CurveConfig,
    scalar_mul::glv::GLVConfig,
    short_weierstrass::{Affine, SWCurveConfig},
    AffineRepr, PrimeGroup,
};
use ark_ff::{AdditiveGroup, One, PrimeField, Zero};
use ruint::aliases::U512;

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
use crate::ark_ff_delegation::{BigIntMacro as BigInt, MontFp};
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
use ark_ff::{BigInt, MontFp};
use ark_serialize::{Compress, SerializationError};
use core::ops::Neg;

use super::g1_swu_iso;
use crate::{
    bls12_381::{
        util::{
            read_g1_compressed, read_g1_uncompressed, serialize_fq, EncodingFlags,
            G1_SERIALIZED_SIZE,
        },
        Fq, Fr,
    },
    glv_decomposition::GLVConfigNoAllocator,
};

pub type G1Affine = bls12::G1Affine<crate::bls12_381::curves::Config>;
pub type G1Projective = bls12::G1Projective<crate::bls12_381::curves::Config>;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Config;

impl CurveConfig for Config {
    type BaseField = Fq;
    type ScalarField = Fr;

    /// COFACTOR = (x - 1)^2 / 3  = 76329603384216526031706109802092473003
    const COFACTOR: &'static [u64] = &[0x8c00aaab0000aaab, 0x396c8c005555e156];

    /// COFACTOR_INV = COFACTOR^{-1} mod r
    /// = 52435875175126190458656871551744051925719901746859129887267498875565241663483
    const COFACTOR_INV: Fr =
        MontFp!("52435875175126190458656871551744051925719901746859129887267498875565241663483");
}

impl SWCurveConfig for Config {
    /// COEFF_A = 0
    const COEFF_A: Fq = Fq::ZERO;

    /// COEFF_B = 4
    const COEFF_B: Fq = MontFp!("4");

    /// AFFINE_GENERATOR_COEFFS = (G1_GENERATOR_X, G1_GENERATOR_Y)
    const GENERATOR: G1Affine = G1Affine::new_unchecked(G1_GENERATOR_X, G1_GENERATOR_Y);

    #[inline(always)]
    fn mul_by_a(_: Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }

    #[inline]
    fn mul_projective(p: &G1Projective, scalar: &[u64]) -> G1Projective {
        let s = scalar_from_limbs(scalar);
        GLVConfig::glv_mul_projective(*p, s)
    }

    #[inline]
    fn mul_affine(base: &Affine<Self>, scalar: &[u64]) -> G1Projective {
        Self::mul_projective(&base.into_group(), scalar)
    }

    #[inline]
    fn is_in_correct_subgroup_assuming_on_curve(p: &G1Affine) -> bool {
        // Algorithm from Section 6 of https://eprint.iacr.org/2021/1130.
        //
        // Check that endomorphism_p(P) == -[X^2]P

        // An early-out optimization described in Section 6.
        // If uP == P but P != point of infinity, then the point is not in the right
        // subgroup.
        let x_times_p = p.mul_bigint(crate::bls12_381::curves::Config::X);
        if x_times_p.eq(p) && !p.infinity {
            return false;
        }

        let minus_x_squared_times_p = x_times_p
            .mul_bigint(crate::bls12_381::curves::Config::X)
            .neg();
        let endomorphism_p = endomorphism(p);
        minus_x_squared_times_p.eq(&endomorphism_p)
    }

    #[inline]
    fn clear_cofactor(p: &G1Affine) -> G1Affine {
        // Using the effective cofactor, as explained in
        // Section 5 of https://eprint.iacr.org/2019/403.pdf.
        //
        // It is enough to multiply by (1 - x), instead of (x - 1)^2 / 3
        let h_eff = one_minus_x().into_bigint();
        Config::mul_affine(&p, h_eff.as_ref()).into()
    }

    fn deserialize_with_mode<R: ark_serialize::Read>(
        mut reader: R,
        compress: ark_serialize::Compress,
        validate: ark_serialize::Validate,
    ) -> Result<Affine<Self>, ark_serialize::SerializationError> {
        let p = if compress == ark_serialize::Compress::Yes {
            read_g1_compressed(&mut reader)?
        } else {
            read_g1_uncompressed(&mut reader)?
        };

        if validate == ark_serialize::Validate::Yes && !p.is_in_correct_subgroup_assuming_on_curve()
        {
            return Err(SerializationError::InvalidData);
        }
        Ok(p)
    }

    fn serialize_with_mode<W: ark_serialize::Write>(
        item: &Affine<Self>,
        mut writer: W,
        compress: ark_serialize::Compress,
    ) -> Result<(), SerializationError> {
        let encoding = EncodingFlags {
            is_compressed: compress == ark_serialize::Compress::Yes,
            is_infinity: item.is_zero(),
            is_lexographically_largest: item.y > -item.y,
        };
        let mut p = *item;
        if encoding.is_infinity {
            p = G1Affine::zero();
        }
        // need to access the field struct `x` directly, otherwise we get None from xy()
        // method
        let x_bytes = serialize_fq(p.x);
        if encoding.is_compressed {
            let mut bytes: [u8; G1_SERIALIZED_SIZE] = x_bytes;

            encoding.encode_flags(&mut bytes);
            writer.write_all(&bytes)?;
        } else {
            let mut bytes = [0u8; 2 * G1_SERIALIZED_SIZE];
            bytes[0..G1_SERIALIZED_SIZE].copy_from_slice(&x_bytes[..]);
            bytes[G1_SERIALIZED_SIZE..].copy_from_slice(&serialize_fq(p.y)[..]);

            encoding.encode_flags(&mut bytes);
            writer.write_all(&bytes)?;
        };

        Ok(())
    }

    fn serialized_size(compress: Compress) -> usize {
        if compress == Compress::Yes {
            G1_SERIALIZED_SIZE
        } else {
            G1_SERIALIZED_SIZE * 2
        }
    }
}

/// The scalar (little-endian limbs, any value below 2^256) as a field element, reduced modulo
/// the group order. On the delegated field this is one Montgomery multiplication by `R²`
/// (`x R^-1 R² = x R`, the Montgomery form of `x`, with the reduction of the multiplication:
/// `x R² < 2^256 r`, so the product is below `2r` and one conditional subtraction makes it
/// canonical), where `from_sign_and_limbs` would run arkworks' software multiplication.
#[inline(always)]
/// `k P + l Q` with the doublings shared between the two GLV multiplications
pub fn mul_two(p: &G1Projective, k: Fr, q: &G1Projective, l: Fr) -> G1Projective {
    crate::glv_decomposition::glv_mul_two_projective_jsf::<Config>(*p, k, *q, l)
}

fn scalar_from_limbs(scalar: &[u64]) -> Fr {
    #[cfg(any(
        all(target_arch = "riscv32", feature = "bigint_ops"),
        test,
        feature = "proving"
    ))]
    if scalar.len() <= 4 {
        let mut repr = <Fr as PrimeField>::BigInt::default();
        repr.as_mut()[..scalar.len()].copy_from_slice(scalar);
        let mut s = Fr::new_unchecked(repr);
        s *= &Fr::new_unchecked(Fr::R2);
        return s;
    }
    Fr::from_sign_and_limbs(true, scalar)
}

impl GLVConfig for Config {
    const ENDO_COEFFS: &'static[Self::BaseField] = &[
        MontFp!("793479390729215512621379701633421447060886740281060493010456487427281649075476305620758731620350")
    ];

    const LAMBDA: Self::ScalarField =
        MontFp!("52435875175126190479447740508185965837461563690374988244538805122978187051009");

    const SCALAR_DECOMP_COEFFS: [(bool, <Self::ScalarField as PrimeField>::BigInt); 4] = [
        (true, BigInt!("228988810152649578064853576960394133504")),
        (true, BigInt!("1")),
        (false, BigInt!("1")),
        (true, BigInt!("228988810152649578064853576960394133503")),
    ];

    fn endomorphism(p: &G1Projective) -> G1Projective {
        let mut res = (*p).clone();
        res.x *= Self::ENDO_COEFFS[0];
        res
    }

    fn glv_mul_projective(p: G1Projective, k: Self::ScalarField) -> G1Projective {
        crate::glv_decomposition::glv_mul_projective_jsf::<Self>(p, k)
    }

    fn endomorphism_affine(p: &Affine<Self>) -> Affine<Self> {
        let mut res = (*p).clone();
        res.x *= Self::ENDO_COEFFS[0];
        res
    }

    fn scalar_decomposition(
        k: Self::ScalarField,
    ) -> ((bool, Self::ScalarField), (bool, Self::ScalarField)) {
        Self::scalar_decomposition_no_allocator(k)
    }
}

impl GLVConfigNoAllocator for Config {
    const BETA_1: (bool, U512) = (
        true,
        U512::from_limbs([
            2263426366270003411,
            10926721885838854917,
            11648686701815784454,
            238326537624862759,
            7203196592358157870,
            8965520006802549469,
            1,
            0,
        ]),
    );

    const BETA_2: (bool, U512) = (
        false,
        U512::from_limbs([
            4788304978035696531,
            7279011843745230193,
            4086414915577876179,
            3841734232051169148,
            2,
            0,
            0,
            0,
        ]),
    );
}

fn one_minus_x() -> Fr {
    const X: Fr = Fr::from_sign_and_limbs(
        !crate::bls12_381::curves::Config::X_IS_NEGATIVE,
        crate::bls12_381::curves::Config::X,
    );
    Fr::one() - X
}

// Parameters from the [IETF draft v16, section E.2](https://www.ietf.org/archive/id/draft-irtf-cfrg-hash-to-curve-16.html#name-11-isogeny-map-for-bls12-381).
impl WBConfig for Config {
    type IsogenousCurve = g1_swu_iso::SwuIsoConfig;

    const ISOGENY_MAP: IsogenyMap<'static, Self::IsogenousCurve, Self> =
        g1_swu_iso::ISOGENY_MAP_TO_G1;
}

/// G1_GENERATOR_X =
/// 3685416753713387016781088315183077757961620795782546409894578378688607592378376318836054947676345821548104185464507
pub const G1_GENERATOR_X: Fq = MontFp!("3685416753713387016781088315183077757961620795782546409894578378688607592378376318836054947676345821548104185464507");

/// G1_GENERATOR_Y =
/// 1339506544944476473020471379941921221584933875938349620426543736416511423956333506472724655353366534992391756441569
pub const G1_GENERATOR_Y: Fq = MontFp!("1339506544944476473020471379941921221584933875938349620426543736416511423956333506472724655353366534992391756441569");

/// BETA is a non-trivial cubic root of unity in Fq.
pub const BETA: Fq = MontFp!("793479390729215512621379701633421447060886740281060493010456487427281649075476305620758731620350");

pub fn endomorphism(p: &Affine<Config>) -> Affine<Config> {
    // Endomorphism of the points on the curve.
    // endomorphism_p(x,y) = (BETA * x, y)
    // where BETA is a non-trivial cubic root of unity in Fq.
    let mut res = (*p).clone();
    res.x *= BETA;
    res
}

#[cfg(test)]
mod mul_tests {
    use super::*;
    use ark_ec::CurveGroup;
    use ark_ff::{BigInteger, UniformRand};
    type RefFq = ark_bls12_381::Fq;
    type RefAffine = ark_bls12_381::G1Affine;

    fn to_ref(p: G1Affine) -> RefAffine {
        let conv = |x: Fq| {
            let mut limbs = [0u64; 6];
            limbs.copy_from_slice(&x.into_bigint().0[..6]);
            RefFq::from_bigint(ark_ff::BigInt(limbs)).unwrap()
        };
        if p.infinity {
            RefAffine::identity()
        } else {
            RefAffine::new_unchecked(conv(p.x), conv(p.y))
        }
    }

    #[test]
    fn scalar_multiplication_matches_the_reference() {
        use ark_std::test_rng;
        let mut rng = test_rng();
        let r = ark_bls12_381::Fr::MODULUS;
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
        ];
        for _ in 0..20 {
            scalars.push(ark_bls12_381::Fr::rand(&mut rng).into_bigint().0);
        }
        let mut points = vec![G1Affine::identity(), G1Affine::generator()];
        for _ in 0..5 {
            let k = ark_bls12_381::Fr::rand(&mut rng).into_bigint();
            points.push(G1Affine::generator().mul_bigint(k).into_affine());
        }
        for p in &points {
            let reference = to_ref(*p);
            for scalar in &scalars {
                let expected =
                    ark_bls12_381::g1::Config::mul_affine(&reference, scalar).into_affine();
                let ours = Config::mul_affine(p, scalar).into_affine();
                assert_eq!(to_ref(ours), expected);
            }
        }
        // the two-point multiplication agrees with two single ones
        for (i, p) in points.iter().enumerate() {
            let q = points[(i + 3) % points.len()];
            for pair in scalars.windows(2) {
                let scalar =
                    |limbs: [u64; 4]| Fr::from_bigint(<Fr as PrimeField>::BigInt::new(limbs));
                let (Some(k), Some(l)) = (scalar(pair[0]), scalar(pair[1])) else {
                    continue;
                };
                let expected = p.into_group() * k + q.into_group() * l;
                let ours = super::mul_two(&p.into_group(), k, &q.into_group(), l);
                assert_eq!(ours.into_affine(), expected.into_affine());
            }
        }
    }
}

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
