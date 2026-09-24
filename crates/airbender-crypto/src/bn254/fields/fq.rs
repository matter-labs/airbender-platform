#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
compile_error!("feature `bigint_ops` must be activated for RISC-V target");

pub type Fq = Fp256<MontBackend<FqConfig, 4>>;
use crate::ark_ff_delegation::{BigInt, BigIntMacro, Fp, Fp256, MontBackend, MontConfig};
use crate::bigint_delegation::{u256, DelegatedModParams, DelegatedMontParams};
use ark_ff::{AdditiveGroup, Zero};

type B = BigInt<4>;
type F = Fp<MontBackend<FqConfig, 4usize>, 4usize>;

static MODULUS_CONSTANT: B =
    BigIntMacro!("21888242871839275222246405745257275088696311157297823662689037894645226208583");
// it's - MODULUS^-1 mod 2^256
static MONT_REDUCTION_CONSTANT: B =
    BigIntMacro!("111032442853175714102588374283752698368366046808579839647964533820976443843465");
// 2 p, the reduction constant of the redundant representation
static DOUBLE_MODULUS_CONSTANT: B =
    BigIntMacro!("43776485743678550444492811490514550177392622314595647325378075789290452417166");

// a^-1 = a ^ (p - 2)
const INVERSION_POW: B = BigInt([
    4332616871279656263u64 - 2,
    10917124144477883021u64,
    13281191951274694749u64,
    3486998266802970665u64,
]);

#[derive(Default)]
struct FqParams;

// the redundant representation needs 4 modulus < 2^256 (the top limb below 2^62)
const _: () = assert!(FqConfig::MODULUS.0[3] >> 62 == 0);

impl DelegatedModParams<4> for FqParams {
    const MODULUS_BITSIZE: usize = 254;
    const REDUNDANT: bool = true;

    fn modulus() -> &'static BigInt<4> {
        &MODULUS_CONSTANT
    }

    fn double_modulus() -> &'static BigInt<4> {
        &DOUBLE_MODULUS_CONSTANT
    }
}

impl DelegatedMontParams<4> for FqParams {
    fn reduction_const() -> &'static BigInt<4> {
        &MONT_REDUCTION_CONSTANT
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FqConfig;

// NOTE: even though we pretend to be u64 everywhere, on LE machine (and our RISC-V 32IM is such) we do not care
// for purposes of our precompile calls

impl MontConfig<4usize> for FqConfig {
    const MODULUS: B = BigInt([
        4332616871279656263u64,
        10917124144477883021u64,
        13281191951274694749u64,
        3486998266802970665u64,
    ]);
    const GENERATOR: F = {
        let (is_positive, limbs) = (true, [3u64]);
        Fp::from_sign_and_limbs(is_positive, &limbs)
    };
    const TWO_ADIC_ROOT_OF_UNITY: F = {
        let (is_positive, limbs) = (
            true,
            [
                4332616871279656262u64,
                10917124144477883021u64,
                13281191951274694749u64,
                3486998266802970665u64,
            ],
        );
        Fp::from_sign_and_limbs(is_positive, &limbs)
    };
    #[inline(always)]
    fn add_assign(a: &mut F, b: &F) {
        unsafe {
            u256::add_mod_assign::<FqParams>(&mut a.0, &b.0);
        }
    }
    #[inline(always)]
    fn sub_assign(a: &mut F, b: &F) {
        unsafe {
            u256::sub_mod_assign::<FqParams>(&mut a.0, &b.0);
        }
    }
    #[inline(always)]
    fn double_in_place(a: &mut F) {
        unsafe {
            u256::double_mod_assign::<FqParams>(&mut a.0);
        }
    }
    /// Sets `a = -a`.
    #[inline(always)]
    fn neg_in_place(a: &mut F) {
        unsafe {
            u256::neg_mod_assign::<FqParams>(&mut a.0);
        }
    }
    #[inline(always)]
    fn mul_assign(a: &mut F, b: &F) {
        unsafe {
            u256::mul_assign_montgomery::<FqParams>(&mut a.0, &b.0);
        }
    }

    #[inline(always)]
    fn square_in_place(a: &mut F) {
        unsafe {
            u256::square_assign_montgomery::<FqParams>(&mut a.0);
        }
    }

    // We will override to also use 256-digit approach here
    #[inline(always)]
    fn into_bigint(mut a: Fp<MontBackend<Self, 4>, 4>) -> BigInt<4> {
        // a multiplication by 1, whose result (below 2p in the redundant representation) is
        // then made canonical
        unsafe {
            u256::mul_assign_montgomery::<FqParams>(&mut a.0, &BigInt::one());
            u256::reduce_to_canonical::<FqParams>(&mut a.0);
        }

        a.0
    }

    /// Modular: the redundant representation has two representatives of every element
    #[inline(always)]
    fn eq(a: &F, b: &F) -> bool {
        unsafe { u256::eq_mod::<FqParams>(&a.0, &b.0) }
    }

    #[inline(always)]
    fn is_zero(a: &F) -> bool {
        unsafe { u256::is_zero_mod::<FqParams>(&a.0) }
    }

    /// Fermat: a^(p - 2). A few hundred delegated Montgomery operations, which is cheaper
    /// than the binary GCD (`__gcd_inverse`) whose word-by-word halving runs in software.
    #[inline(always)]
    fn inverse(a: &Fp<MontBackend<Self, 4>, 4>) -> Option<Fp<MontBackend<Self, 4>, 4>> {
        if a.is_zero() {
            return None;
        }
        Some(crate::ark_ff_delegation::pow_window4(a, &INVERSION_POW.0))
    }

    /// In place, with delegated copies: no by-value moves of the operands
    #[inline(always)]
    fn sum_of_products<const M: usize>(a: &[F; M], b: &[F; M]) -> F {
        let mut sum = F::ZERO;
        let mut product = F::ZERO;
        for i in 0..M {
            u256::copy_assign(&mut product.0, &a[i].0);
            unsafe {
                u256::mul_assign_montgomery::<FqParams>(&mut product.0, &b[i].0);
                u256::add_mod_assign::<FqParams>(&mut sum.0, &product.0);
            }
        }
        sum
    }
}

#[cfg(test)]
fn __gcd_inverse(a: &F) -> Option<F> {
    if a.is_zero() {
        return None;
    }
    // Guajardo Kumar Paar Pelzl
    // Efficient Software-Implementation of Finite Fields with Applications to
    // Cryptography
    // Algorithm 16 (BEA for Inversion in Fp)

    use ark_ff::BigInteger;
    use ark_ff::PrimeField;

    let mut u = a.0;
    let mut v = F::MODULUS;
    let mut b = Fp::new_unchecked(F::R2); // Avoids unnecessary reduction step.
    let mut c = Fp::zero();
    let modulus = F::MODULUS;

    while !u256::is_one(&u) && !u256::is_one(&v) {
        while u.is_even() {
            u.div2();

            if b.0.is_even() {
                b.0.div2();
            } else {
                let _carry = u256::add_assign(&mut b.0, &modulus);
                b.0.div2();
                // if !Self::MODULUS_HAS_SPARE_BIT && carry {
                //     (b.0).0[N - 1] |= 1 << 63;
                // }
            }
        }

        while v.is_even() {
            v.div2();

            if c.0.is_even() {
                c.0.div2();
            } else {
                let _carry = u256::add_assign(&mut c.0, &modulus);
                c.0.div2();
                // if !Self::MODULUS_HAS_SPARE_BIT && carry {
                //     (c.0).0[N - 1] |= 1 << 63;
                // }
            }
        }

        // if v < u {
        if v.lt(&u) {
            u256::sub_assign(&mut u, &v);
            b -= &c;
        } else {
            u256::sub_assign(&mut v, &u);
            c -= &b;
        }
    }

    if u256::is_one(&u) {
        Some(b)
    } else {
        Some(c)
    }
}

#[cfg(test)]
mod test {
    use super::Fq;
    use ark_ff::{Field, One, UniformRand, Zero};

    #[test]
    fn test_mul_properties() {
        const ITERATIONS: usize = 1000;
        use ark_std::test_rng;
        let mut rng = test_rng();
        let zero = Fq::zero();
        let one = Fq::one();
        assert_eq!(one.inverse().unwrap(), one, "One inverse failed");
        assert!(one.is_one(), "One is not one");

        assert!(Fq::ONE.is_one(), "One constant is not one");
        assert_eq!(Fq::ONE, one, "One constant is incorrect");

        for _ in 0..ITERATIONS {
            // Associativity
            let a = Fq::rand(&mut rng);
            let b = Fq::rand(&mut rng);
            let c = Fq::rand(&mut rng);
            assert_eq!((a * b) * c, a * (b * c), "Associativity failed");

            // Commutativity
            assert_eq!(a * b, b * a, "Commutativity failed");

            // Identity
            assert_eq!(one * a, a, "Identity mul failed");
            assert_eq!(one * b, b, "Identity mul failed");
            assert_eq!(one * c, c, "Identity mul failed");

            assert_eq!(zero * a, zero, "Mul by zero failed");
            assert_eq!(zero * b, zero, "Mul by zero failed");
            assert_eq!(zero * c, zero, "Mul by zero failed");

            // Inverses
            assert_eq!(a * a.inverse().unwrap(), one, "Mul by inverse failed");
            assert_eq!(b * b.inverse().unwrap(), one, "Mul by inverse failed");
            assert_eq!(c * c.inverse().unwrap(), one, "Mul by inverse failed");

            // Associativity and commutativity simultaneously
            let t0 = (a * b) * c;
            let t1 = (a * c) * b;
            let t2 = (b * c) * a;
            assert_eq!(t0, t1, "Associativity + commutativity failed");
            assert_eq!(t1, t2, "Associativity + commutativity failed");

            // Squaring
            assert_eq!(a * a, a.square(), "Squaring failed");
            assert_eq!(b * b, b.square(), "Squaring failed");
            assert_eq!(c * c, c.square(), "Squaring failed");

            // Distributivity
            assert_eq!(a * (b + c), a * b + a * c, "Distributivity failed");
            assert_eq!(b * (a + c), b * a + b * c, "Distributivity failed");
            assert_eq!(c * (a + b), c * a + c * b, "Distributivity failed");
            assert_eq!(
                (a + b).square(),
                a.square() + b.square() + a * ark_ff::AdditiveGroup::double(&b),
                "Distributivity for square failed"
            );
            assert_eq!(
                (b + c).square(),
                c.square() + b.square() + c * ark_ff::AdditiveGroup::double(&b),
                "Distributivity for square failed"
            );
            assert_eq!(
                (c + a).square(),
                a.square() + c.square() + a * ark_ff::AdditiveGroup::double(&c),
                "Distributivity for square failed"
            );
        }
    }

    /// Products whose low limb is zero (the carry-only path of the reduction) and the
    /// extreme residues, against the reference implementation
    #[test]
    fn test_mul_edge_cases() {
        use ark_ff::{Field, One, PrimeField};
        type RefFq = ark_bn254::Fq;
        let from_ref = |r: RefFq| {
            Fq::from_bigint(crate::ark_ff_delegation::BigInt(r.into_bigint().0)).unwrap()
        };
        let p_minus_1 = -RefFq::one();
        let two_128 = RefFq::from(2u64).pow([128u64]);
        // the Montgomery form of `2^256^-1` is 1: a zero low limb
        let r_inv = RefFq::from(2u64).pow([256u64]).inverse().unwrap();
        let values = [
            RefFq::zero(),
            RefFq::one(),
            p_minus_1,
            two_128,
            r_inv,
            RefFq::from(2u64),
            two_128 * two_128,
            p_minus_1 * r_inv,
            two_128 * r_inv,
        ];
        for &ra in values.iter() {
            for &rb in values.iter() {
                let mut a = from_ref(ra);
                let b = from_ref(rb);
                a *= &b;
                assert_eq!(a.into_bigint().0, (ra * rb).into_bigint().0, "{ra} * {rb}");
                let mut sq = from_ref(ra);
                sq.square_in_place();
                assert_eq!(sq.into_bigint().0, ra.square().into_bigint().0);
            }
        }
    }

    #[test]
    fn test_mul_correctness() {
        use std::str::FromStr;

        type RefFq = ark_bn254::Fq;

        let a = Fq::from_str("-1").unwrap();
        let ref_a = RefFq::from_str("-1").unwrap();

        assert_eq!(a.0 .0, ref_a.0 .0);
    }

    // NOTE: those tests are backported as we need to init static and run single thread
    // instead of full arkwords test suite. This coverage is ok as our base math is just
    // very small

    pub const ITERATIONS: usize = 100;
    use crate::bn254::curves::Bn254;
    use ark_ec::{pairing::*, CurveGroup, PrimeGroup};
    use ark_ff::{CyclotomicMultSubgroup, PrimeField};
    use ark_std::test_rng;

    #[test]
    fn test_bilinearity() {
        for _ in 0..ITERATIONS {
            let mut rng = test_rng();
            let a: <Bn254 as Pairing>::G1 = UniformRand::rand(&mut rng);
            let b: <Bn254 as Pairing>::G2 = UniformRand::rand(&mut rng);
            let s: <Bn254 as Pairing>::ScalarField = UniformRand::rand(&mut rng);

            let sa = a * s;
            let sb = b * s;

            let ans1 = <Bn254>::pairing(sa, b);
            let ans2 = <Bn254>::pairing(a, sb);
            let ans3 = <Bn254>::pairing(a, b) * s;

            assert_eq!(ans1, ans2);
            assert_eq!(ans2, ans3);

            assert_ne!(ans1, PairingOutput::zero());
            assert_ne!(ans2, PairingOutput::zero());
            assert_ne!(ans3, PairingOutput::zero());
            let group_order = <<Bn254 as Pairing>::ScalarField>::characteristic();

            assert_eq!(ans1.mul_bigint(group_order), PairingOutput::zero());
            assert_eq!(ans2.mul_bigint(group_order), PairingOutput::zero());
            assert_eq!(ans3.mul_bigint(group_order), PairingOutput::zero());
        }
    }

    #[test]
    fn test_multi_pairing() {
        for _ in 0..ITERATIONS {
            let rng = &mut test_rng();

            let a = <Bn254 as Pairing>::G1::rand(rng).into_affine();
            let b = <Bn254 as Pairing>::G2::rand(rng).into_affine();
            let c = <Bn254 as Pairing>::G1::rand(rng).into_affine();
            let d = <Bn254 as Pairing>::G2::rand(rng).into_affine();
            let ans1 = <Bn254>::pairing(a, b) + &<Bn254>::pairing(c, d);
            let ans2 = <Bn254>::multi_pairing(&[a, c], &[b, d]);
            assert_eq!(ans1, ans2);
        }
    }

    #[test]
    fn test_final_exp() {
        for _ in 0..ITERATIONS {
            let rng = &mut test_rng();
            let fp_ext = <Bn254 as Pairing>::TargetField::rand(rng);
            let gt = <Bn254 as Pairing>::final_exponentiation(MillerLoopOutput(fp_ext))
                .unwrap()
                .0;
            let r = <Bn254 as Pairing>::ScalarField::MODULUS;
            assert!(gt.cyclotomic_exp(r).is_one());
        }
    }
}

#[cfg(test)]
mod redundant_representation_tests {
    use super::*;
    use ark_ff::{Field, One, PrimeField, UniformRand};

    fn to_ref(a: &F) -> ark_bn254::Fq {
        ark_bn254::Fq::from_bigint(ark_ff::BigInt(a.into_bigint().0)).unwrap()
    }

    /// The other representative of `a`: its limbs plus the modulus (the arithmetic must
    /// accept it and every observation must not tell it apart)
    fn shifted(a: &F) -> F {
        let mut limbs = a.0;
        u256::add_assign(&mut limbs, FqParams::modulus());
        F::new_unchecked(limbs)
    }

    #[test]
    fn representatives_are_equal_and_reduce_the_same() {
        let mut rng = ark_std::test_rng();
        for _ in 0..50 {
            let a = F::rand(&mut rng);
            let b = F::rand(&mut rng);
            let a2 = shifted(&a);
            assert_ne!(a.0, a2.0);
            assert_eq!(a, a2);
            assert_eq!(a.into_bigint(), a2.into_bigint());
            assert!(!a2.is_zero() || a.is_zero());
            // every operation gives the same element from either representative
            let ops: [fn(F, F) -> F; 4] = [|x, y| x + y, |x, y| x - y, |x, y| x * y, |x, y| y - x];
            for op in ops {
                let expected = op(a, b);
                assert_eq!(op(a2, b), expected);
                assert_eq!(op(a2, shifted(&b)), expected);
                assert_eq!(to_ref(&op(a2, shifted(&b))), {
                    let (ra, rb) = (to_ref(&a), to_ref(&b));
                    match ops.iter().position(|f| *f as usize == op as usize).unwrap() {
                        0 => ra + rb,
                        1 => ra - rb,
                        2 => ra * rb,
                        _ => rb - ra,
                    }
                });
            }
            assert_eq!(-a2, -a);
            assert_eq!(a2.double(), a.double());
            assert_eq!(a2.square(), a.square());
            assert_eq!(a2.inverse(), a.inverse());
            assert_eq!(
                to_ref(&a2.inverse().unwrap()),
                to_ref(&a).inverse().unwrap()
            );
        }
    }

    #[test]
    fn zero_and_one_have_two_representatives() {
        let zero_as_modulus = F::new_unchecked(*FqParams::modulus());
        assert!(zero_as_modulus.is_zero());
        assert_eq!(zero_as_modulus, F::ZERO);
        assert_eq!(zero_as_modulus.into_bigint(), BigInt::zero());
        assert!((-zero_as_modulus).is_zero());
        let one2 = shifted(&F::ONE);
        assert!(one2.is_one());
        assert_eq!(one2 * F::from(7u64), F::from(7u64));
        assert!(!shifted(&F::from(2u64)).is_one());
        assert!(!shifted(&F::from(2u64)).is_zero());
    }

    #[test]
    fn non_canonical_integers_are_rejected() {
        assert!(F::from_bigint(*FqParams::modulus()).is_none());
        let mut above = *FqParams::modulus();
        u256::add_assign(&mut above, &BigInt::one());
        assert!(F::from_bigint(above).is_none());
        assert!(F::from_bigint(BigInt::zero()).unwrap().is_zero());
        let mut below = *FqParams::modulus();
        u256::sub_assign(&mut below, &BigInt::one());
        assert_eq!(F::from_bigint(below).unwrap(), -F::ONE);
    }

    #[test]
    fn results_stay_below_twice_the_modulus() {
        let mut rng = ark_std::test_rng();
        let below = |x: &F| u256::lt(&x.0, FqParams::double_modulus());
        for _ in 0..50 {
            let a = shifted(&F::rand(&mut rng));
            let b = shifted(&F::rand(&mut rng));
            for x in [a + b, a - b, a * b, -a, a.double(), a.square()] {
                assert!(below(&x));
            }
        }
    }
}
