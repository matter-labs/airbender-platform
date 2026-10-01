use ark_ec::{
    bls12,
    bls12::Bls12Config,
    hashing::curve_maps::wb::{IsogenyMap, WBConfig},
    models::CurveConfig,
    scalar_mul::glv::GLVConfig,
    short_weierstrass::{Affine, Projective, SWCurveConfig},
    AffineRepr,
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

use super::g1_swu_iso;
use crate::{
    bls12_381::{
        util::{
            read_g1_compressed, read_g1_uncompressed, serialize_fq, EncodingFlags,
            G1_SERIALIZED_SIZE,
        },
        Fq, Fr,
    },
    extension_tower::CopyAssign,
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
        // Algorithm from Section 6 of https://eprint.iacr.org/2021/1130: P is in G1 iff
        // endomorphism(P) == -[X^2]P.
        //
        // [X^2]P = [X]([X]P) by two double-and-adds over the bits of X on the Jacobian group
        // law, which handles every case (doubling, P + (-P), the point at infinity): the
        // multiples are exact for any point of the curve, in G1 or not. The GLV multiplication
        // computes the same two multiples (X decomposes as (X, 0)), but normalizes [X]P to
        // affine for the second one: a field inversion.
        //
        // The early-out of Section 6 (reject [X]P == P for P != O) is not needed: [X]P == P
        // gives -[X^2]P == -P, and -P == endomorphism(P) = (BETA x, y) needs x = y = 0, which
        // is not a point of the curve (and X - 1 is coprime to the group order, so [X]P == P
        // only for O).
        if p.infinity {
            return true;
        }
        let (_, x_squared_times_p) = x_and_x_squared_times(p);
        -x_squared_times_p == endomorphism(p)
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

/// `k P + l Q` in affine coordinates, with the field divisions of `divider` (see
/// `crate::affine_glv`)
pub fn mul_two_affine_with_divider<D: crate::affine_glv::Divider<Fq>>(
    p: &G1Affine,
    k: Fr,
    q: &G1Affine,
    l: Fr,
    divider: &mut D,
) -> G1Affine {
    crate::affine_glv::glv_mul_two_affine::<Config, D>(p, k, q, l, divider)
}

/// `a + b` in affine coordinates, with the field division of `divider`
pub fn add_affine_with_divider<D: crate::affine_glv::Divider<Fq>>(
    a: &G1Affine,
    b: &G1Affine,
    divider: &mut D,
) -> G1Affine {
    crate::affine_glv::add_affine::<Config, D>(a, b, divider)
}

/// The subgroup membership test of `is_in_correct_subgroup_assuming_on_curve` in affine
/// coordinates, with the field divisions of `divider`: `[x]P` and `[x²]P` by double-and-add on
/// the 64-bit seed
pub fn is_in_subgroup_with_divider<D: crate::affine_glv::Divider<Fq>>(
    p: &G1Affine,
    divider: &mut D,
) -> bool {
    const _: () = assert!(crate::bls12_381::curves::Config::X.len() == 1);
    let x = crate::bls12_381::curves::Config::X[0];
    // Algorithm from Section 6 of https://eprint.iacr.org/2021/1130: `σ(P) == -[x²]P`, with the
    // early out `[x]P == P` (for `P` not the point at infinity) of that section
    let x_times_p = crate::affine_glv::mul_u64_affine::<Config, D>(p, x, divider);
    if !p.infinity && x_times_p == *p {
        return false;
    }
    let x_squared_times_p = crate::affine_glv::mul_u64_affine::<Config, D>(&x_times_p, x, divider);
    endomorphism(p) == -x_squared_times_p
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

/// `([X]P, [X²]P)` for the absolute value `X` of the curve parameter: two double-and-adds over
/// the bits of `X` on the Jacobian group law, mixed additions of `P` for the first, full
/// additions of the (not normalized) `[X]P` for the second. Exact for any point of the curve,
/// in G1 or not: the scalar is not reduced and nothing is normalized. Generic over the curve
/// configuration so that the tests can also run it on the arkworks field.
#[inline(always)]
fn x_and_x_squared_times<C: SWCurveConfig>(p: &Affine<C>) -> (Projective<C>, Projective<C>)
where
    C::BaseField: CopyAssign,
{
    use crate::jacobian::Jacobian;
    use core::mem::MaybeUninit;
    const X_LIMBS: &[u64] = <crate::bls12_381::curves::Config as Bls12Config>::X;
    const _: () = assert!(X_LIMBS.len() == 1);
    const X: u64 = X_LIMBS[0];
    // the most significant bit of X (63), which starts both chains
    const TOP: u32 = u64::BITS - 1 - X.leading_zeros();

    // [X]P, mixed additions of the affine P
    let mut x_p_slot = MaybeUninit::uninit();
    let x_p = Jacobian::<C>::init_infinity(&mut x_p_slot);
    x_p.add_assign_affine(p);
    for i in (0..TOP).rev() {
        x_p.double_in_place();
        if (X >> i) & 1 == 1 {
            x_p.add_assign_affine(p);
        }
    }

    // [X]([X]P), full additions of [X]P
    let mut x2_p_slot = MaybeUninit::uninit();
    let x2_p = Jacobian::<C>::init_copy(&mut x2_p_slot, x_p);
    for i in (0..TOP).rev() {
        x2_p.double_in_place();
        if (X >> i) & 1 == 1 {
            x2_p.add_assign(x_p);
        }
    }

    (x_p.to_projective(), x2_p.to_projective())
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

    /// Scalars below 2^256 at the edges of the reduction modulo the group order `r` and of the
    /// decomposition, against the plain double-and-add on the arkworks field, from points that
    /// are not multiplied out by GLV
    #[test]
    fn scalar_multiplication_of_edge_scalars_matches_the_reference() {
        use ark_ec::scalar_mul::sw_double_and_add_affine;
        use ark_std::rand::Rng;
        use num_bigint::BigUint;
        let mut rng = ark_std::test_rng();
        let r: BigUint = ark_bls12_381::Fr::MODULUS.into();
        let x = BigUint::from(crate::bls12_381::curves::Config::X[0]);
        let one = BigUint::from(1u32);
        let mut scalars = vec![
            &one << 255u32,
            &r * 2u32,
            &r * 2u32 + 1u32,
            (&r - 1u32) / 2u32,
            (&r + 1u32) / 2u32,
            x.clone(),
        ];
        // full-width (about half of them at least r) and 50-bit
        scalars.extend((0..16).map(|_| BigUint::from_bytes_le(&rng.gen::<[u8; 32]>())));
        scalars.extend((0..4).map(|_| BigUint::from(rng.gen::<u64>() >> 14)));
        // the decomposition rounds `s (X² - 1) / r` to the nearest integer: the scalars on both
        // sides of the rounding points `j + 1/2` for `j` in `[0, X² - 2]`, and the same plus r
        let n22 = &x * &x - 1u32;
        for j in [BigUint::from(0u32), one.clone(), &n22 / 2u32, &n22 - 1u32] {
            let s = (j * 2u32 + 1u32) * &r / (&n22 * 2u32);
            scalars.extend([s.clone(), &s + 1u32, &s + &r, &s + 1u32 + &r]);
        }
        let limbs = |s: &BigUint| {
            assert!(s.bits() <= 256);
            let mut limbs = [0u64; 4];
            for (limb, digit) in limbs.iter_mut().zip(s.iter_u64_digits()) {
                *limb = digit;
            }
            limbs
        };

        let mut points = vec![G1Affine::identity(), G1Affine::generator()];
        for _ in 0..4 {
            let k = ark_bls12_381::Fr::rand(&mut rng).into_bigint();
            points.push(sw_double_and_add_affine(&G1Affine::generator(), k).into_affine());
        }
        for p in &points {
            let reference = to_ref(*p);
            for s in &scalars {
                let s = limbs(s);
                let expected = sw_double_and_add_affine(&reference, s).into_affine();
                let ours = Config::mul_affine(p, &s).into_affine();
                assert_eq!(to_ref(ours), expected, "{p:?} * {s:?}");
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

    /// The affine test with divisions agrees with the projective one: on points of the
    /// subgroup, on points of the curve outside it, and on the point at infinity
    #[test]
    fn subgroup_test_with_divider_matches() {
        use super::{is_in_subgroup_with_divider, Config, Fq, G1Affine};
        use crate::affine_glv::InvertingDivider;
        use ark_ec::short_weierstrass::SWCurveConfig;
        use ark_ec::AffineRepr;
        use ark_ff::{PrimeField, UniformRand};
        // random values through the reference field: `Fq::rand` of the 512-bit representation
        // rejects almost every sample
        let from_ref = |x: ark_bls12_381::Fq| {
            let mut limbs = [0u64; 8];
            limbs[..6].copy_from_slice(&x.into_bigint().0);
            Fq::from_bigint(crate::BigInt(limbs)).unwrap()
        };
        let mut rng = ark_std::test_rng();
        let mut checked_outside = 0;
        for i in 0..12 {
            let p = if i < 3 {
                let p = ark_bls12_381::G1Affine::rand(&mut rng);
                G1Affine::new_unchecked(from_ref(p.x), from_ref(p.y))
            } else {
                // a point of the curve from a random x, in the subgroup with probability 1/h
                let x = from_ref(ark_bls12_381::Fq::rand(&mut rng));
                match G1Affine::get_point_from_x_unchecked(x, i % 2 == 0) {
                    Some(p) => p,
                    None => continue,
                }
            };
            let expected = Config::is_in_correct_subgroup_assuming_on_curve(&p);
            checked_outside += usize::from(!expected);
            assert_eq!(
                is_in_subgroup_with_divider(&p, &mut InvertingDivider),
                expected
            );
        }
        assert!(checked_outside > 0);
        assert!(is_in_subgroup_with_divider(
            &G1Affine::identity(),
            &mut InvertingDivider
        ));
    }
}

/// The subgroup check in the delegated field representation of the `cfg(test)` build (the
/// forward one: `tests/bls12_381_g1_subgroup.rs`) against its previous implementation (kept
/// here verbatim), upstream arkworks and the ground truth `[r]P == O`, and both multiples of
/// `x_and_x_squared_times` (in the delegated and in the arkworks field) against exact ones
#[cfg(test)]
mod subgroup_tests {
    use super::*;
    use ark_ec::{CurveGroup, PrimeGroup};
    use ark_ff::{Field, UniformRand};
    use ark_std::rand::{rngs::StdRng, Rng, SeedableRng};
    use ark_std::test_rng;
    use core::ops::Neg;
    use num_bigint::BigUint;

    type RefFq = ark_bls12_381::Fq;
    type RefFr = ark_bls12_381::Fr;
    type RefAffine = ark_bls12_381::G1Affine;
    type RefProjective = ark_bls12_381::G1Projective;

    /// The primes of `m = (X + 1) / 3`: the cofactor is `h = 3 m²`, and the points of order
    /// dividing it form `Z/m × Z/3m` (there are no points of order `ℓ²`)
    const PRIMES_OF_M: [u64; 4] = [11, 10177, 859267, 52437899];

    /// The body of `is_in_correct_subgroup_assuming_on_curve` before the double-and-adds (GLV
    /// multiplications, an inversion, the early-out), verbatim
    fn subgroup_check_before(p: &G1Affine) -> bool {
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

    fn x_abs() -> BigUint {
        BigUint::from(crate::bls12_381::curves::Config::X[0])
    }

    fn r() -> BigUint {
        RefFr::MODULUS.into()
    }

    /// The cofactor `h = (X + 1)² / 3`: `#E(Fq) = h r`
    fn h() -> BigUint {
        (x_abs() + 1u32).pow(2) / 3u32
    }

    /// `[k]P` by a plain double-and-add on arkworks' projective group law, which handles every
    /// case: exact for any point of the curve (`k` is not reduced modulo `r`, no GLV)
    fn mul_exact(p: &RefAffine, k: &BigUint) -> RefProjective {
        let mut acc = RefProjective::zero();
        for i in (0..k.bits()).rev() {
            acc.double_in_place();
            if k.bit(i) {
                acc += p;
            }
        }
        acc
    }

    /// The ground truth
    fn in_g1(p: &RefAffine) -> bool {
        mul_exact(p, &r()).is_zero()
    }

    fn fq(x: RefFq) -> Fq {
        // from the canonical limbs: `From<BigUint>` goes through `from_le_bytes_mod_order`,
        // whose chunks are shorter than the debug-asserted input of the delegated
        // representation's byte reader
        let src = x.into_bigint().0;
        let mut limbs = <Fq as PrimeField>::BigInt::default();
        limbs.0[..src.len()].copy_from_slice(&src);
        Fq::from_bigint(limbs).expect("a canonical value")
    }

    fn ref_fq(x: Fq) -> RefFq {
        RefFq::from(BigUint::from(x))
    }

    /// The same coordinates (any, the flag kept)
    fn from_ref(p: &RefAffine) -> G1Affine {
        G1Affine {
            x: fq(p.x),
            y: fq(p.y),
            infinity: p.infinity,
        }
    }

    fn to_ref(p: &G1Affine) -> RefAffine {
        RefAffine {
            x: ref_fq(p.x),
            y: ref_fq(p.y),
            infinity: p.infinity,
        }
    }

    fn to_ref_projective(p: &G1Projective) -> RefProjective {
        RefProjective::new_unchecked(ref_fq(p.x), ref_fq(p.y), ref_fq(p.z))
    }

    /// A random point of the curve (in G1 with probability 1/h)
    fn random_point(rng: &mut impl Rng) -> RefAffine {
        for _ in 0..64 {
            if let Some(p) = RefAffine::get_point_from_x_unchecked(RefFq::rand(rng), rng.gen()) {
                return p;
            }
        }
        panic!("no point of the curve at 64 random x");
    }

    fn random_g1(rng: &mut impl Rng) -> RefAffine {
        (RefProjective::generator() * RefFr::rand(rng)).into_affine()
    }

    /// A point of order `order`: `[multiple]R` for a random point `R`, resampled while it is O
    fn torsion_point(rng: &mut impl Rng, multiple: &BigUint, order: u64) -> RefAffine {
        for _ in 0..64 {
            let t = mul_exact(&random_point(rng), multiple).into_affine();
            if !t.is_zero() {
                assert!(mul_exact(&t, &BigUint::from(order)).is_zero());
                return t;
            }
        }
        panic!("[{multiple}]R is O for 64 random R");
    }

    /// Points of order dividing the cofactor, none O: two `[r]R` (order dividing `X + 1`), one
    /// of order 3 (`(0, ±2)`) and, for every prime `ℓ | m`, two of order `ℓ` (generically
    /// independent) and their sum (unless O)
    fn cofactor_points(rng: &mut impl Rng) -> Vec<RefAffine> {
        let (r, h) = (r(), h());
        let mut points = vec![];
        for _ in 0..2 {
            let t = mul_exact(&random_point(rng), &r).into_affine();
            assert!(!t.is_zero() && mul_exact(&t, &(x_abs() + 1u32)).is_zero());
            points.push(t);
        }
        let t = torsion_point(rng, &(&r * &h / 3u32), 3);
        assert!(t.x.is_zero());
        points.push(t);
        for l in PRIMES_OF_M {
            let multiple = &r * &h / BigUint::from(l * l);
            let (a, b) = (
                torsion_point(rng, &multiple, l),
                torsion_point(rng, &multiple, l),
            );
            points.extend([a, b]);
            let sum = (a + b).into_affine();
            if !sum.is_zero() {
                points.push(sum);
            }
        }
        points
    }

    /// The new check against the old one and, for a point of the curve, against upstream
    /// arkworks and the ground truth; both multiples against the exact ones (from the
    /// delegated field and from the arkworks field); the result
    fn check(p: &G1Affine) -> bool {
        let new = p.is_in_correct_subgroup_assuming_on_curve();
        assert_eq!(new, subgroup_check_before(p), "old != new at {p:?}");
        let reference = to_ref(p);
        if reference.is_on_curve() {
            assert_eq!(
                new,
                reference.is_in_correct_subgroup_assuming_on_curve(),
                "arkworks at {p:?}"
            );
            assert_eq!(new, in_g1(&reference), "[r]P == O at {p:?}");
        }
        // off the curve too: no formula uses b, so both are the group law of the curve
        // y² = x³ + b' through the point
        let expected = (
            mul_exact(&reference, &x_abs()),
            mul_exact(&reference, &(x_abs() * x_abs())),
        );
        let (x_p, x2_p) = x_and_x_squared_times(p);
        let multiples = (to_ref_projective(&x_p), to_ref_projective(&x2_p));
        assert_eq!(multiples, expected, "multiples at {p:?}");
        let multiples = x_and_x_squared_times(&reference);
        assert_eq!(multiples, expected, "multiples (arkworks field) at {p:?}");
        new
    }

    /// The other representative of the same residue below `2p` (the delegated field keeps any)
    fn other_representative(a: Fq) -> Fq {
        let modulus = BigUint::from(<Fq as PrimeField>::MODULUS);
        let limbs = BigUint::from(a.0);
        let other = match limbs < modulus {
            true => limbs + &modulus,
            false => limbs - &modulus,
        };
        assert!(other < &modulus * 2u32);
        let b = Fq::new_unchecked(other.try_into().unwrap());
        assert!(b.0 != a.0 && b == a);
        b
    }

    #[test]
    fn curve_constants_and_exact_multiples() {
        let mut rng = test_rng();
        let (x, r, h) = (x_abs(), r(), h());
        let q: BigUint = RefFq::MODULUS.into();
        // #E(Fq) = q + 1 - t with the trace t = 1 - X
        assert_eq!(&h * &r, q + &x);
        let cofactor = Config::COFACTOR
            .iter()
            .rev()
            .fold(BigUint::from(0u32), |acc, limb| (acc << 64u32) + *limb);
        assert_eq!(h, cofactor);
        let m = PRIMES_OF_M
            .iter()
            .fold(BigUint::from(1u32), |acc, l| acc * *l);
        assert_eq!(&m * 3u32, &x + 1u32);
        assert_eq!(&m * &m * 3u32, h);
        // the structure of the part of order dividing the cofactor: exponent X + 1, no points
        // of order ℓ²
        for _ in 0..4 {
            let p = random_point(&mut rng);
            assert!(!in_g1(&p));
            assert!(mul_exact(&p, &(&r * (&x + 1u32))).is_zero());
            for l in PRIMES_OF_M {
                assert!(mul_exact(&p, &(&r * &h / BigUint::from(l))).is_zero());
            }
        }
        // exact multiples are not reduced modulo r: (0, 2) has order 3 and r = 1 (mod 3),
        // while arkworks' projective multiplication (GLV) reduces the scalar
        let t = RefAffine::new_unchecked(RefFq::ZERO, RefFq::from(2u32));
        assert!(t.is_on_curve());
        assert_eq!(mul_exact(&t, &r).into_affine(), t);
        assert!(mul_exact(&t, &BigUint::from(3u32)).is_zero());
        assert!(t.into_group().mul_bigint(RefFr::MODULUS).is_zero());
    }

    /// The old check (both GLV multiplications) is exact outside G1 because X decomposes as
    /// (X, 0): no endomorphism is involved
    #[test]
    fn x_decomposes_as_x_and_zero() {
        let x = Fr::from(crate::bls12_381::curves::Config::X[0]);
        let ((positive, k1), (_, k2)) = Config::scalar_decomposition(x);
        assert!(positive && k1 == x && k2.is_zero());
    }

    #[test]
    fn subgroup_points() {
        let mut rng = test_rng();
        let g = RefAffine::generator();
        let mut points = vec![
            G1Affine::identity(),
            G1Affine {
                x: Fq::ONE,
                y: Fq::ONE,
                infinity: true,
            },
            G1Affine::generator(),
            -G1Affine::generator(),
        ];
        for k in [BigUint::from(1u32), BigUint::from(2u32), r() - 1u32] {
            points.push(from_ref(&mul_exact(&g, &k).into_affine()));
        }
        for _ in 0..16 {
            points.push(from_ref(&random_g1(&mut rng)));
        }
        // cleared cofactors: [h]R and [k h]R
        for _ in 0..4 {
            let p = random_point(&mut rng);
            let k: BigUint = RefFr::rand(&mut rng).into();
            for multiple in [h(), k * h()] {
                points.push(from_ref(&mul_exact(&p, &multiple).into_affine()));
            }
        }
        for p in &points {
            assert!(check(p), "{p:?}");
        }
    }

    #[test]
    fn points_outside_the_subgroup() {
        let mut rng = test_rng();
        let two = RefFq::from(2u32);
        let order_3 = [
            RefAffine::new_unchecked(RefFq::ZERO, two),
            RefAffine::new_unchecked(RefFq::ZERO, -two),
        ];
        let cofactor_points = cofactor_points(&mut rng);
        // not in G1 by construction
        let mut points = order_3.to_vec();
        points.extend(&cofactor_points);
        for t in order_3.iter().chain(&cofactor_points) {
            points.push((random_g1(&mut rng) + t).into_affine());
        }
        for _ in 0..32 {
            let p = random_point(&mut rng);
            points.extend([p, -p]);
        }
        for p in &points {
            assert!(!p.is_zero() && p.is_on_curve());
            assert!(!check(&from_ref(p)), "{p:?}");
        }
        // random multiples of points of the cofactor part and of random points, labelled by
        // the ground truth only
        for i in 0..8 {
            let k: BigUint = RefFr::rand(&mut rng).into();
            let t = cofactor_points[i % cofactor_points.len()];
            let p = random_point(&mut rng);
            for q in [
                mul_exact(&t, &k),
                mul_exact(&p, &k),
                mul_exact(&p, &(&k * r())),
            ] {
                check(&from_ref(&q.into_affine()));
            }
        }
    }

    /// Outside the contract of the check (`deserialize_with_mode` without compression calls it
    /// on points off the curve): the old and the new check are compared, and the multiples
    #[test]
    fn points_off_the_curve() {
        let mut rng = test_rng();
        let f = |a: u64| RefFq::from(a);
        // incl. points of the cusp y² = x³, whose non-singular points are a group
        let mut points = vec![
            (f(0), f(0)),
            (f(0), f(1)),
            (f(1), f(0)),
            (f(1), f(1)),
            (f(1), -f(1)),
            (f(4), f(8)),
            (f(4), -f(8)),
        ];
        for _ in 0..16 {
            points.push((RefFq::rand(&mut rng), RefFq::rand(&mut rng)));
        }
        for _ in 0..4 {
            points.push((RefFq::rand(&mut rng), RefFq::ZERO));
            let t = RefFq::rand(&mut rng);
            let (x, y) = (t.square(), t.square() * t);
            points.extend([(x, y), (x, -y)]);
        }
        for (x, y) in points {
            let p = RefAffine::new_unchecked(x, y);
            assert!(!p.is_on_curve());
            check(&from_ref(&p));
        }
    }

    /// Every coordinate is any representative below `2p` in the delegated field: the other
    /// representative of `x`, `y` or both gives the same answer
    #[test]
    fn redundant_representatives() {
        let mut rng = test_rng();
        let g = RefAffine::generator();
        let two = RefFq::from(2u32);
        let cofactor_points = cofactor_points(&mut rng);
        let mut points = vec![
            g,
            -g,
            RefAffine::new_unchecked(RefFq::ZERO, two),
            RefAffine::new_unchecked(RefFq::ZERO, -two),
            random_point(&mut rng),
            (random_g1(&mut rng) + cofactor_points[3]).into_affine(),
            // off the curve, y = 0 (then encoded as p): a doubling gives Z = 2YZ = p
            RefAffine::new_unchecked(RefFq::rand(&mut rng), RefFq::ZERO),
            RefAffine::new_unchecked(RefFq::ZERO, RefFq::ZERO),
        ];
        points.extend(&cofactor_points);
        for p in &points {
            let p = from_ref(p);
            let expected = check(&p);
            let (x, y) = (other_representative(p.x), other_representative(p.y));
            for (x, y) in [(x, p.y), (p.x, y), (x, y)] {
                let q = G1Affine {
                    x,
                    y,
                    infinity: false,
                };
                assert_eq!(check(&q), expected, "{p:?}");
            }
        }
        // the point at infinity with coordinates 0, zero as p, junk
        let zero_as_p = Fq::new_unchecked(<Fq as PrimeField>::MODULUS);
        for (x, y) in [
            (Fq::ZERO, Fq::ZERO),
            (zero_as_p, zero_as_p),
            (Fq::ONE, other_representative(Fq::ONE)),
            (zero_as_p, Fq::ONE),
        ] {
            assert!(check(&G1Affine {
                x,
                y,
                infinity: true
            }));
        }
    }

    #[test]
    fn random_sweep() {
        let mut rng = StdRng::seed_from_u64(39);
        let cofactor_points = cofactor_points(&mut rng);
        for i in 0..96 {
            check(&from_ref(&random_point(&mut rng)));
            let g = random_g1(&mut rng);
            assert!(check(&from_ref(&g)));
            let t = cofactor_points[i % cofactor_points.len()];
            assert!(!check(&from_ref(&(g + t).into_affine())));
        }
    }
}
