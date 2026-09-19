#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
compile_error!("feature `bigint_ops` must be activated for RISC-V target");

// Scalar field of bn254 on the bigint delegation: the GLV decomposition of a scalar
// multiplication runs its Montgomery arithmetic here.

use crate::ark_ff_delegation::{BigInt, BigIntMacro, Fp, Fp256, MontBackend, MontConfig};
use crate::bigint_delegation::{u256, DelegatedModParams, DelegatedMontParams};
use ark_ff::{AdditiveGroup, Zero};

// -MODULUS^-1 mod 2^256
static MONT_REDUCTION_CONSTANT: BigInt<4> =
    BigIntMacro!("52454480824480482120356829342366457550537710351690908576382634413609933864959");
static MODULUS: BigInt<4> = FrConfig::MODULUS;

/// a^-1 = a^(r - 2)
const INVERSION_POW: BigInt<4> = BigInt([
    4891460686036598785u64 - 2,
    2896914383306846353u64,
    13281191951274694749u64,
    3486998266802970665u64,
]);

#[derive(Default, Debug)]
pub struct FrParams;

impl DelegatedModParams<4> for FrParams {
    const MODULUS_BITSIZE: usize = 254;
    fn modulus() -> &'static BigInt<4> {
        &MODULUS
    }
}

impl DelegatedMontParams<4> for FrParams {
    fn reduction_const() -> &'static BigInt<4> {
        &MONT_REDUCTION_CONSTANT
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrConfig;

pub type Fr = Fp256<MontBackend<FrConfig, 4>>;

impl MontConfig<4> for FrConfig {
    const MODULUS: BigInt<4> = BigIntMacro!(
        "21888242871839275222246405745257275088548364400416034343698204186575808495617"
    );
    const GENERATOR: Fr = {
        let (is_positive, limbs) = (true, [5u64]);
        Fp::from_sign_and_limbs(is_positive, &limbs)
    };
    const TWO_ADIC_ROOT_OF_UNITY: Fr = {
        let (is_positive, limbs) = (
            true,
            [
                11229192882073836016,
                4624371214017703636,
                63235024940837564,
                3043318377369730693,
            ],
        );
        Fp::from_sign_and_limbs(is_positive, &limbs)
    };
    const SMALL_SUBGROUP_BASE: Option<u32> = Some(3u32);
    const SMALL_SUBGROUP_BASE_ADICITY: Option<u32> = Some(2);
    const LARGE_SUBGROUP_ROOT_OF_UNITY: Option<Fr> = Some({
        let (is_positive, limbs) = (
            true,
            [
                10639863269868064110,
                6020083959115413713,
                15196548748307230377,
                1274670453483637722,
            ],
        );
        Fr::from_sign_and_limbs(is_positive, &limbs)
    });

    fn into_bigint(mut a: Fr) -> BigInt<4> {
        unsafe {
            u256::mul_assign_montgomery::<FrParams>(&mut a.0, &BigInt::one());
        }
        a.0
    }

    #[inline(always)]
    fn add_assign(a: &mut Fr, b: &Fr) {
        unsafe {
            u256::add_mod_assign::<FrParams>(&mut a.0, &b.0);
        }
    }

    #[inline(always)]
    fn sub_assign(a: &mut Fr, b: &Fr) {
        unsafe {
            u256::sub_mod_assign::<FrParams>(&mut a.0, &b.0);
        }
    }

    #[inline(always)]
    fn double_in_place(a: &mut Fr) {
        unsafe {
            u256::double_mod_assign::<FrParams>(&mut a.0);
        }
    }

    #[inline(always)]
    fn neg_in_place(a: &mut Fr) {
        unsafe {
            u256::neg_mod_assign::<FrParams>(&mut a.0);
        }
    }

    #[inline(always)]
    fn mul_assign(a: &mut Fr, b: &Fr) {
        unsafe {
            u256::mul_assign_montgomery::<FrParams>(&mut a.0, &b.0);
        }
    }

    #[inline(always)]
    fn square_in_place(a: &mut Fr) {
        unsafe {
            u256::square_assign_montgomery::<FrParams>(&mut a.0);
        }
    }

    /// Fermat: a^(r - 2), a few hundred delegated Montgomery operations
    fn inverse(a: &Fr) -> Option<Fr> {
        if a.is_zero() {
            return None;
        }
        Some(crate::ark_ff_delegation::pow_window4(a, &INVERSION_POW.0))
    }

    fn sum_of_products<const M: usize>(a: &[Fr; M], b: &[Fr; M]) -> Fr {
        let mut sum = Fr::ZERO;
        let mut product = Fr::ZERO;
        for i in 0..M {
            u256::copy_assign(&mut product.0, &a[i].0);
            unsafe {
                u256::mul_assign_montgomery::<FrParams>(&mut product.0, &b[i].0);
                u256::add_mod_assign::<FrParams>(&mut sum.0, &product.0);
            }
        }
        sum
    }
}

#[cfg(test)]
mod test {
    use super::{BigInt, Fr, FrConfig, MontConfig};
    use ark_ff::{FftField, Field, MontConfig as ArkMontConfig, PrimeField, UniformRand, Zero};

    type RefFr = ark_bn254::Fr;
    type RefConfig = ark_bn254::FrConfig;

    fn from_ref(x: RefFr) -> Fr {
        Fr::from_bigint(BigInt(x.into_bigint().0)).unwrap()
    }

    #[test]
    fn constants_match_the_reference() {
        assert_eq!(FrConfig::MODULUS.0, RefFr::MODULUS.0);
        assert_eq!(FrConfig::R.0, <RefConfig as ArkMontConfig<4>>::R.0);
        assert_eq!(FrConfig::R2.0, <RefConfig as ArkMontConfig<4>>::R2.0);
        assert_eq!(
            FrConfig::GENERATOR.into_bigint().0,
            <RefFr as FftField>::GENERATOR.into_bigint().0
        );
        assert_eq!(
            FrConfig::TWO_ADIC_ROOT_OF_UNITY.into_bigint().0,
            <RefFr as FftField>::TWO_ADIC_ROOT_OF_UNITY.into_bigint().0
        );
        assert_eq!(
            FrConfig::LARGE_SUBGROUP_ROOT_OF_UNITY
                .unwrap()
                .into_bigint()
                .0,
            <RefFr as FftField>::LARGE_SUBGROUP_ROOT_OF_UNITY
                .unwrap()
                .into_bigint()
                .0
        );
        assert_eq!(Fr::from_bigint(BigInt::from(1u64)).unwrap().0, FrConfig::R);
    }

    #[test]
    fn arithmetic_matches_the_reference() {
        use ark_std::test_rng;
        let mut rng = test_rng();
        for i in 0..20000 {
            let ref_a = RefFr::rand(&mut rng);
            let ref_b = RefFr::rand(&mut rng);
            let (a, b) = (from_ref(ref_a), from_ref(ref_b));
            assert_eq!(a.into_bigint().0, ref_a.into_bigint().0);
            assert_eq!(
                (a * b).into_bigint().0,
                (ref_a * ref_b).into_bigint().0,
                "mul {i}"
            );
            assert_eq!(
                (a + b).into_bigint().0,
                (ref_a + ref_b).into_bigint().0,
                "add {i}"
            );
            assert_eq!(
                (a - b).into_bigint().0,
                (ref_a - ref_b).into_bigint().0,
                "sub {i}"
            );
            assert_eq!(
                a.square().into_bigint().0,
                ref_a.square().into_bigint().0,
                "sq {i}"
            );
            assert_eq!((-a).into_bigint().0, (-ref_a).into_bigint().0, "neg {i}");
            if i < 200 {
                assert_eq!(
                    a.inverse().unwrap().into_bigint().0,
                    ref_a.inverse().unwrap().into_bigint().0,
                    "inv {i}"
                );
            }
        }
        assert_eq!(Fr::zero().inverse(), None);
        // reduction of wide byte strings, as the GLV decomposition does it
        for _ in 0..2000 {
            let bytes: [u8; 64] = core::array::from_fn(|_| ark_std::rand::Rng::gen(&mut rng));
            assert_eq!(
                Fr::from_le_bytes_mod_order(&bytes).into_bigint().0,
                RefFr::from_le_bytes_mod_order(&bytes).into_bigint().0
            );
        }
    }
}
