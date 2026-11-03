#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
compile_error!("feature `bigint_ops` must be activated for RISC-V target");

// partially reused cargo expand of derived FqConfig with multiplication updated

// Prime modulus is 4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787

// NOTE: we operate with 256-bit "limbs", so Montgomery representation is 512 bits

#[derive(Default)]
struct FqParams;

impl DelegatedModParams<8> for FqParams {
    const MODULUS_BITSIZE: usize = 381;
    // elements are any representative below `2 modulus`: a multiplication then needs no final
    // subtraction (`2^512 > 4 modulus`), see `u512::mul_assign_montgomery`
    const REDUNDANT: bool = true;

    fn modulus() -> &'static BigInt<8> {
        &MODULUS_CONSTANT
    }

    fn double_modulus() -> &'static BigInt<8> {
        &DOUBLE_MODULUS_CONSTANT
    }
}

impl DelegatedMontParams<8> for FqParams {
    fn reduction_const() -> &'static BigInt<4> {
        &MONT_REDUCTION_CONSTANT
    }
}

impl u512::DelegatedLazyParams for FqParams {
    fn four_modulus_squared() -> &'static BigInt<12> {
        &FOUR_MODULUS_SQUARED
    }

    fn eight_modulus_squared() -> &'static BigInt<12> {
        &EIGHT_MODULUS_SQUARED
    }
}

/// `(a0 + a1 u) *= (b0 + b1 u)` with the reduction deferred, see `u512::fp2_mul_assign_lazy`
#[inline(always)]
pub(crate) fn fq2_mul_assign_lazy(a0: &mut F, a1: &mut F, b0: &F, b1: &F) {
    unsafe { u512::fp2_mul_assign_lazy::<FqParams>(&mut a0.0, &mut a1.0, &b0.0, &b1.0) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FqConfig;

const NUM_LIMBS: usize = 8usize;

pub type Fq = Fp512<MontBackend<FqConfig, NUM_LIMBS>>;

use crate::ark_ff_delegation::{BigInt, BigIntMacro, Fp, Fp512, MontBackend, MontConfig};
use crate::bigint_delegation::{u512, DelegatedModParams, DelegatedMontParams};
use ark_ff::{AdditiveGroup, Field, PrimeField, Zero};

type B = BigInt<NUM_LIMBS>;
type F = Fp<MontBackend<FqConfig, NUM_LIMBS>, NUM_LIMBS>;

static MODULUS_CONSTANT: BigInt<8> = BigIntMacro!("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
static DOUBLE_MODULUS_CONSTANT: BigInt<8> = BigIntMacro!("8004819110443334786835579651471808313113765639878015770664116272248063300981675728885375258258031328075788545119574");
static FOUR_MODULUS_SQUARED: BigInt<12> = BigIntMacro!("64077128990918821647774994577275890470780480397322210639449538147943906993965428573602624317208773719899817542160682922124415538158827623744800809684145743244372052137087251032065631877985326645674164094711099599885703973957941476");
static EIGHT_MODULUS_SQUARED: BigInt<12> = BigIntMacro!("128154257981837643295549989154551780941560960794644421278899076295887813987930857147205248634417547439799635084321365844248831076317655247489601619368291486488744104274174502064131263755970653291348328189422199199771407947915882952");
// it's - MODULUS^-1 mod 2^256
static MONT_REDUCTION_CONSTANT: BigInt<4> =
    BigIntMacro!("11726191667098586211898467594267748916577995138249226639719947807923487178749");

// a^-1 = a ^ (p - 2)
const INVERSION_POW: [u64; 6] = [
    13402431016077863595u64 - 2,
    2210141511517208575u64,
    7435674573564081700u64,
    7239337960414712511u64,
    5412103778470702295u64,
    1873798617647539866u64,
];

// NOTE: even though we pretend to be u64 everywhere, on LE machine (and our RISC-V 32IM is such) we do not care
// for purposes of our precompile calls

impl MontConfig<NUM_LIMBS> for FqConfig {
    const MODULUS: B = BigInt([
        13402431016077863595u64,
        2210141511517208575u64,
        7435674573564081700u64,
        7239337960414712511u64,
        5412103778470702295u64,
        1873798617647539866u64,
        0,
        0,
    ]);

    // we also need to override into_bigint to properly perform
    // conversion
    fn into_bigint(mut a: Fp<MontBackend<Self, NUM_LIMBS>, NUM_LIMBS>) -> BigInt<NUM_LIMBS> {
        // a multiplication with 1 literal, then the canonical representative
        unsafe {
            u512::mul_assign_montgomery::<FqParams>(&mut a.0, &BigInt::one());
            u512::reduce_to_canonical::<FqParams>(&mut a.0);
        }
        a.0
    }

    #[inline(always)]
    fn eq(a: &F, b: &F) -> bool {
        unsafe { u512::eq_mod::<FqParams>(&a.0, &b.0) }
    }

    #[inline(always)]
    fn is_zero(a: &F) -> bool {
        unsafe { u512::is_zero_mod::<FqParams>(&a.0) }
    }

    const GENERATOR: F = {
        let (is_positive, limbs) = (true, [2u64]);
        Fp::from_sign_and_limbs(is_positive, &limbs)
    };
    const TWO_ADIC_ROOT_OF_UNITY: F = {
        let (is_positive, limbs) = (
            true,
            [
                13402431016077863594u64,
                2210141511517208575u64,
                7435674573564081700u64,
                7239337960414712511u64,
                5412103778470702295u64,
                1873798617647539866u64,
            ],
        );
        Fp::from_sign_and_limbs(is_positive, &limbs)
    };
    const SMALL_SUBGROUP_BASE: Option<u32> = Some(3u32);
    const SMALL_SUBGROUP_BASE_ADICITY: Option<u32> = Some(2u32);
    const LARGE_SUBGROUP_ROOT_OF_UNITY: Option<F> = Some({
        let (is_positive, limbs) = (
            true,
            [
                5896295325348737640u64,
                5503863413011229930u64,
                11466573396089897971u64,
                17103254516989687468u64,
                7243505556163372831u64,
                1399342764408159943u64,
            ],
        );
        Fp::from_sign_and_limbs(is_positive, &limbs)
    });

    #[inline(always)]
    fn add_assign(a: &mut F, b: &F) {
        unsafe {
            u512::add_mod_assign::<FqParams>(&mut a.0, &b.0);
        }
    }
    #[inline(always)]
    fn sub_assign(a: &mut F, b: &F) {
        unsafe {
            u512::sub_mod_assign::<FqParams>(&mut a.0, &b.0);
        }
    }

    #[inline(always)]
    fn double_in_place(a: &mut F) {
        unsafe {
            u512::double_mod_assign::<FqParams>(&mut a.0);
        }
    }
    /// Sets `a = -a`.
    #[inline(always)]
    fn neg_in_place(a: &mut F) {
        unsafe {
            u512::neg_mod_assign::<FqParams>(&mut a.0);
        }
    }

    #[inline(always)]
    fn mul_assign(a: &mut F, b: &F) {
        unsafe {
            u512::mul_assign_montgomery::<FqParams>(&mut a.0, &b.0);
        }
    }

    #[inline(always)]
    fn square_in_place(a: &mut F) {
        unsafe {
            u512::square_assign_montgomery::<FqParams>(&mut a.0);
        }
    }

    fn inverse(
        a: &Fp<MontBackend<Self, NUM_LIMBS>, NUM_LIMBS>,
    ) -> Option<Fp<MontBackend<Self, NUM_LIMBS>, NUM_LIMBS>> {
        if a.is_zero() {
            return None;
        }

        let inverse = a.pow(INVERSION_POW);

        Some(inverse)
    }

    /// In place: the by-value `a[i] * b[i]` of the default moves 64-byte values through
    /// `memcpy` calls; here the product is built in one buffer with delegated copies.
    #[inline(always)]
    fn sum_of_products<const M: usize>(a: &[F; M], b: &[F; M]) -> F {
        let mut sum = F::ZERO;
        let mut product = F::ZERO;
        for i in 0..M {
            u512::copy_assign(&mut product.0, &a[i].0);
            unsafe {
                u512::mul_assign_montgomery::<FqParams>(&mut product.0, &b[i].0);
                u512::add_mod_assign::<FqParams>(&mut sum.0, &product.0);
            }
        }
        sum
    }
}

#[cfg(test)]
mod test {
    use super::{BigInt, Fq, FqConfig, MontConfig, B};
    use ark_ff::{Field, One, UniformRand, Zero};

    #[test]
    fn fq2_lazy_multiplication_matches_arkworks() {
        use crate::bls12_381::fields::fq2::Fq2;
        use crate::extension_tower::fp2_mul_assign;
        use ark_std::test_rng;
        type RefFq2 = ark_bls12_381::Fq2;
        // (sampling the delegated element directly rejects almost every 512-bit draw)
        let convert = |x: ark_bls12_381::Fq| {
            let mut t = BigInt::zero();
            t.0[..6].copy_from_slice(&x.into_bigint().0);
            Fq::from_bigint(t).unwrap()
        };
        let to_ours = |x: RefFq2| Fq2::new(convert(x.c0), convert(x.c1));
        let mut rng = test_rng();
        for _ in 0..2000 {
            let a = RefFq2::rand(&mut rng);
            let b = RefFq2::rand(&mut rng);
            let mut ours = to_ours(a);
            fp2_mul_assign(&mut ours, &to_ours(b));
            assert_eq!(ours, to_ours(a * b));
            // a product of products: the operands are then in the redundant range
            let mut twice = ours;
            fp2_mul_assign(&mut twice, &ours);
            assert_eq!(twice, to_ours(a * b * (a * b)));
        }
    }

    #[test]
    fn test_mul_compare() {
        const ITERATIONS: usize = 100000;
        let one_bigint = BigInt::one();
        let t = Fq::from_bigint(one_bigint).unwrap();
        assert_eq!(t.0, FqConfig::R);

        use ark_std::test_rng;
        let mut rng = test_rng();

        type RefFq = ark_bls12_381::Fq;

        for i in 0..ITERATIONS {
            let ref_a = RefFq::rand(&mut rng);
            let ref_b = RefFq::rand(&mut rng);

            let mut t = BigInt::zero();
            t.0[..6].copy_from_slice(&ref_a.into_bigint().0);
            let a = Fq::from_bigint(t).unwrap();
            let mut t = BigInt::zero();
            t.0[..6].copy_from_slice(&ref_b.into_bigint().0);
            let b = Fq::from_bigint(t).unwrap();

            assert_eq!(ref_a.into_bigint().0[..6], a.into_bigint().0[..6]);
            assert_eq!(ref_b.into_bigint().0[..6], b.into_bigint().0[..6]);

            assert_eq!(
                (ref_a * ref_b).into_bigint().0[..6],
                (a * b).into_bigint().0[..6],
                "failed at iteration {}",
                i,
            );
        }
    }

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

        type RefFq = ark_bls12_381::Fq;

        for _ in 0..ITERATIONS {
            // Associativity
            let ref_a = RefFq::rand(&mut rng);
            let ref_b = RefFq::rand(&mut rng);
            let ref_c = RefFq::rand(&mut rng);

            let a = convert_fq(ref_a);
            let b = convert_fq(ref_b);
            let c = convert_fq(ref_c);
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

    // NOTE: those tests are backported as we need to init static and run single thread
    // instead of full arkwords test suite. This coverage is ok as our base math is just
    // very small

    pub const ITERATIONS: usize = 100;
    use crate::bls12_381::curves::Bls12_381;
    use ark_bls12_381::Bls12_381 as Bls12_381_Ref;
    use ark_bls12_381::Fq as FqRef;
    use ark_bls12_381::Fq2 as Fq2Ref;
    use ark_bls12_381::Fq6 as Fq6Ref;
    use ark_ec::{pairing::*, CurveGroup, PrimeGroup};
    use ark_ff::{CyclotomicMultSubgroup, PrimeField};
    use ark_std::test_rng;

    fn convert_fq(src: FqRef) -> Fq {
        let mut t = B::zero();
        t.0[..6].copy_from_slice(&src.into_bigint().0);

        Fq::from_bigint(t).unwrap()
    }

    fn convert_fq2(src: Fq2Ref) -> super::super::Fq2 {
        super::super::Fq2 {
            c0: convert_fq(src.c0),
            c1: convert_fq(src.c1),
        }
    }

    fn convert_g1(src: <Bls12_381_Ref as Pairing>::G1) -> <Bls12_381 as Pairing>::G1 {
        crate::bls12_381::G1Projective {
            x: convert_fq(src.x),
            y: convert_fq(src.y),
            z: convert_fq(src.z),
        }
    }

    fn convert_g2(src: <Bls12_381_Ref as Pairing>::G2) -> <Bls12_381 as Pairing>::G2 {
        crate::bls12_381::G2Projective {
            x: convert_fq2(src.x),
            y: convert_fq2(src.y),
            z: convert_fq2(src.z),
        }
    }

    fn convert_g1_affine(
        src: <Bls12_381_Ref as Pairing>::G1Affine,
    ) -> <Bls12_381 as Pairing>::G1Affine {
        crate::bls12_381::G1Affine {
            x: convert_fq(src.x),
            y: convert_fq(src.y),
            infinity: src.infinity,
        }
    }

    fn convert_g2_affine(
        src: <Bls12_381_Ref as Pairing>::G2Affine,
    ) -> <Bls12_381 as Pairing>::G2Affine {
        crate::bls12_381::G2Affine {
            x: convert_fq2(src.x),
            y: convert_fq2(src.y),
            infinity: src.infinity,
        }
    }

    fn convert_fq6(src: Fq6Ref) -> crate::bls12_381::Fq6 {
        crate::bls12_381::Fq6 {
            c0: convert_fq2(src.c0),
            c1: convert_fq2(src.c1),
            c2: convert_fq2(src.c2),
        }
    }

    fn convert_fq12(
        src: <Bls12_381_Ref as Pairing>::TargetField,
    ) -> <Bls12_381 as Pairing>::TargetField {
        crate::bls12_381::Fq12 {
            c0: convert_fq6(src.c0),
            c1: convert_fq6(src.c1),
        }
    }

    #[test]
    fn test_bilinearity() {
        for _ in 0..100 {
            let mut rng = test_rng();
            let a: <Bls12_381_Ref as Pairing>::G1 = UniformRand::rand(&mut rng);
            let b: <Bls12_381_Ref as Pairing>::G2 = UniformRand::rand(&mut rng);
            let s: <Bls12_381 as Pairing>::ScalarField = UniformRand::rand(&mut rng);

            let a = convert_g1(a);
            let b = convert_g2(b);

            let sa = a * s;
            let sb = b * s;

            let ans1 = <Bls12_381>::pairing(sa, b);
            let ans2 = <Bls12_381>::pairing(a, sb);
            let ans3 = <Bls12_381>::pairing(a, b) * s;

            assert_eq!(ans1, ans2);
            assert_eq!(ans2, ans3);

            assert_ne!(ans1, PairingOutput::zero());
            assert_ne!(ans2, PairingOutput::zero());
            assert_ne!(ans3, PairingOutput::zero());
            let group_order = <<Bls12_381 as Pairing>::ScalarField>::characteristic();

            assert_eq!(ans1.mul_bigint(group_order), PairingOutput::zero());
            assert_eq!(ans2.mul_bigint(group_order), PairingOutput::zero());
            assert_eq!(ans3.mul_bigint(group_order), PairingOutput::zero());
        }
    }

    #[test]
    fn test_multi_pairing() {
        for _ in 0..ITERATIONS {
            let rng = &mut test_rng();

            let a = <Bls12_381_Ref as Pairing>::G1::rand(rng).into_affine();
            let b = <Bls12_381_Ref as Pairing>::G2::rand(rng).into_affine();
            let c = <Bls12_381_Ref as Pairing>::G1::rand(rng).into_affine();
            let d = <Bls12_381_Ref as Pairing>::G2::rand(rng).into_affine();

            let a = convert_g1_affine(a);
            let b = convert_g2_affine(b);
            let c = convert_g1_affine(c);
            let d = convert_g2_affine(d);

            let ans1 = <Bls12_381>::pairing(a, b) + &<Bls12_381>::pairing(c, d);
            let ans2 = <Bls12_381>::multi_pairing(&[a, c], &[b, d]);
            assert_eq!(ans1, ans2);
        }
    }

    #[test]
    fn test_final_exp() {
        for _ in 0..ITERATIONS {
            let rng = &mut test_rng();
            let fp_ext = <Bls12_381_Ref as Pairing>::TargetField::rand(rng);
            let fp_ext = convert_fq12(fp_ext);
            let gt = <Bls12_381 as Pairing>::final_exponentiation(MillerLoopOutput(fp_ext))
                .unwrap()
                .0;
            let r = <Bls12_381 as Pairing>::ScalarField::MODULUS;
            assert!(gt.cyclotomic_exp(r).is_one());
        }
    }

    /// Products whose low limb is zero (the carry-only path of the reduction) and the
    /// extreme residues, against the reference implementation
    #[test]
    fn test_mul_edge_cases() {
        use ark_ff::PrimeField;
        type RefFq = ark_bls12_381::Fq;
        let from_ref = |r: RefFq| {
            let mut t = BigInt::zero();
            t.0[..6].copy_from_slice(&r.into_bigint().0);
            Fq::from_bigint(t).unwrap()
        };
        let p_minus_1 = -RefFq::one();
        let two_256 = RefFq::from(2u64).pow([256u64]);
        let two_128 = RefFq::from(2u64).pow([128u64]);
        // the Montgomery form of `2^512^-1` is 1: a zero low limb
        let r_inv = RefFq::from(2u64).pow([512u64]).inverse().unwrap();
        let values = [
            RefFq::zero(),
            RefFq::one(),
            p_minus_1,
            two_256,
            two_128,
            r_inv,
            RefFq::from(2u64),
            two_256 * two_128,
            two_256 * r_inv,
            p_minus_1 * r_inv,
        ];
        for &ra in values.iter() {
            for &rb in values.iter() {
                let mut a = from_ref(ra);
                let b = from_ref(rb);
                a *= &b;
                assert_eq!(
                    a.into_bigint().0[..6],
                    (ra * rb).into_bigint().0[..6],
                    "{ra} * {rb}"
                );
                let mut sq = from_ref(ra);
                sq.square_in_place();
                assert_eq!(sq.into_bigint().0[..6], ra.square().into_bigint().0[..6]);
            }
        }
    }

    #[test]
    fn test_montgomery_mul_requires_final_reduction_fq() {
        use super::FqParams;
        use crate::bigint_delegation::u512;
        // Montgomery-form inputs chosen to force the raw Montgomery product
        // above the modulus if final reduction is omitted.
        let mut a = BigInt([
            13402431016077863594,
            2210141511517208575,
            7435674573564081700,
            7239337960414712511,
            5412103778470702295,
            1873798617647539866,
            0,
            0,
        ]); // p - 1 (valid Montgomery residue)

        let b = BigInt([
            2558544279233641998,
            15670776001706131633,
            16499115467214941661,
            5658596800012892911,
            5792713988013659675,
            1672441503323099093,
            0,
            0,
        ]);

        assert!(a < super::MODULUS_CONSTANT);
        assert!(b < super::MODULUS_CONSTANT);

        unsafe {
            u512::mul_assign_montgomery::<FqParams>(&mut a, &b);
        }

        // the representation is redundant: the product is some representative below twice
        // the modulus, and canonical after the reduction
        assert!(
            a < super::DOUBLE_MODULUS_CONSTANT,
            "montgomery mul result above twice the modulus: {:?}",
            a
        );
        unsafe {
            u512::reduce_to_canonical::<FqParams>(&mut a);
        }
        assert!(a < super::MODULUS_CONSTANT);
    }
}

// Helper functions for EIP-2537 precompile serialization
// These are defined here to avoid ICE when calling into_bigint from external crates
#[inline(never)]
pub fn fq_to_eip2537_bytes(el: Fq) -> [u8; 48] {
    let mut result = [0u8; 48];
    let bigint = el.into_bigint();
    let words = bigint.as_ref();
    // Take only the first 6 words (48 bytes total) in big-endian
    for (i, word) in words.iter().take(6).enumerate() {
        let bytes = word.to_be_bytes();
        let start = (5 - i) * 8; // Reverse order for big-endian
        result[start..start + 8].copy_from_slice(&bytes);
    }
    result
}

#[inline(never)]
pub fn fq_from_eip2537_bytes(input: &[u8; 64]) -> Option<Fq> {
    if input[..16].iter().all(|el| *el == 0) == false {
        return None;
    }
    // account for potentially variable representations
    let mut repr = <Fq as ark_ff::PrimeField>::BigInt::zero();
    let repr_slice = repr.as_mut();
    for (dst, src) in repr_slice.iter_mut().zip(input[16..].chunks_exact(8).rev()) {
        *dst = u64::from_be_bytes(src.try_into().unwrap());
    }
    // from_bigint returns None if repr >= MODULUS
    Fq::from_bigint(repr)
}

#[inline(never)]
pub fn write_fq_to_buffer(el: Fq, buffer: &mut [u8]) {
    // First 16 bytes are padding
    buffer[..16].fill(0);
    // Next 48 bytes are the field element
    let fq_bytes = fq_to_eip2537_bytes(el);
    buffer[16..64].copy_from_slice(&fq_bytes);
}
