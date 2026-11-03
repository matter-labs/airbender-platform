#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
use crate::ark_ff::MontFp;
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
#[cfg(test)]
use ark_ec::AffineRepr;
use ark_ec::{
    models::{short_weierstrass::SWCurveConfig, CurveConfig},
    scalar_mul::glv::GLVConfig,
    short_weierstrass::{Affine, Projective},
};
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
use ark_ff::BigInt as ScalarBigInt;
#[cfg(test)]
use ark_ff::Field;
use ark_ff::{AdditiveGroup, PrimeField, Zero};

use crate::bn254::fields::{Fq, Fq2, Fr};
pub type G2Affine = Affine<Config>;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Config;

impl CurveConfig for Config {
    type BaseField = Fq2;
    type ScalarField = Fr;

    /// COFACTOR = (36 * X^4) + (36 * X^3) + (30 * X^2) + 6*X + 1
    /// 21888242871839275222246405745257275088844257914179612981679871602714643921549
    #[rustfmt::skip]
    const COFACTOR: &'static [u64] = &[
        0x345f2299c0f9fa8d,
        0x06ceecda572a2489,
        0xb85045b68181585e,
        0x30644e72e131a029,
    ];

    /// COFACTOR_INV = COFACTOR^{-1} mod r
    // 10944121435919637613327163357776759465618812564592884533313067514031822496649,
    // built from limbs so that it types as either scalar field implementation
    const COFACTOR_INV: Fr = Fr::from_sign_and_limbs(
        true,
        &[
            3348495167459320713,
            3559417044256155545,
            3894081688100485083,
            1743499133401485333,
        ],
    );
}

impl SWCurveConfig for Config {
    /// COEFF_A = [0, 0]
    const COEFF_A: Fq2 = Fq2::ZERO;

    /// COEFF_B = 3/(u+9)
    /// (19485874751759354771024239261021720505790618469301721065564631296452457478373, 266929791119991161246907387137283842545076965332900288569378510910307636690)
    const COEFF_B: Fq2 = Fq2::new(
        MontFp!("19485874751759354771024239261021720505790618469301721065564631296452457478373"),
        MontFp!("266929791119991161246907387137283842545076965332900288569378510910307636690"),
    );

    /// AFFINE_GENERATOR_COEFFS = (G2_GENERATOR_X, G2_GENERATOR_Y)
    const GENERATOR: G2Affine = G2Affine::new_unchecked(G2_GENERATOR_X, G2_GENERATOR_Y);

    #[inline(always)]
    fn mul_by_a(_: Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }

    fn is_in_correct_subgroup_assuming_on_curve(point: &G2Affine) -> bool {
        super::g2_subgroup::is_in_subgroup(point)
    }
}

impl GLVConfig for Config {
    const ENDO_COEFFS: &'static [Self::BaseField] = &[Fq2::new(
        MontFp!("21888242871839275220042445260109153167277707414472061641714758635765020556616"),
        Fq::ZERO,
    )];

    // 4407920970296243842393367215006156084916469457145843978461,
    // built from limbs so that it types as either scalar field implementation
    const LAMBDA: Self::ScalarField = Fr::from_sign_and_limbs(
        true,
        &[
            10022737222657937629,
            6628244256572943015,
            12953715498635827032,
            0,
        ],
    );

    const SCALAR_DECOMP_COEFFS: [(bool, <Self::ScalarField as PrimeField>::BigInt); 4] = [
        (
            false,
            ScalarBigInt([857057580304901387, 8020209761171036669, 0, 0]),
        ), // 147946756881789319010696353538189108491
        (false, ScalarBigInt([9931322734385697763, 0, 0, 0])), // 9931322734385697763
        (true, ScalarBigInt([9931322734385697763, 0, 0, 0])),  // 9931322734385697763
        (
            false,
            ScalarBigInt([9372478919628755240, 8020209761171036668, 0, 0]),
        ), // 147946756881789319000765030803803410728
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
}

pub const G2_GENERATOR_X: Fq2 = Fq2::new(G2_GENERATOR_X_C0, G2_GENERATOR_X_C1);
pub const G2_GENERATOR_Y: Fq2 = Fq2::new(G2_GENERATOR_Y_C0, G2_GENERATOR_Y_C1);

/// G2_GENERATOR_X_C0 =
/// 10857046999023057135944570762232829481370756359578518086990519993285655852781
pub const G2_GENERATOR_X_C0: Fq =
    MontFp!("10857046999023057135944570762232829481370756359578518086990519993285655852781");

/// G2_GENERATOR_X_C1 =
/// 11559732032986387107991004021392285783925812861821192530917403151452391805634
pub const G2_GENERATOR_X_C1: Fq =
    MontFp!("11559732032986387107991004021392285783925812861821192530917403151452391805634");

/// G2_GENERATOR_Y_C0 =
/// 8495653923123431417604973247489272438418190587263600148770280649306958101930
pub const G2_GENERATOR_Y_C0: Fq =
    MontFp!("8495653923123431417604973247489272438418190587263600148770280649306958101930");

/// G2_GENERATOR_Y_C1 =
/// 4082367875863433681332203403145435568316851327593401208105741076214120093531
pub const G2_GENERATOR_Y_C1: Fq =
    MontFp!("4082367875863433681332203403145435568316851327593401208105741076214120093531");

// PSI_X = (u+9)^((p-1)/3) = TWIST_MUL_BY_Q_X
pub(super) const P_POWER_ENDOMORPHISM_COEFF_0: Fq2 = Fq2::new(
    MontFp!("21575463638280843010398324269430826099269044274347216827212613867836435027261"),
    MontFp!("10307601595873709700152284273816112264069230130616436755625194854815875713954"),
);

// PSI_Y = (u+9)^((p-1)/2) = TWIST_MUL_BY_Q_Y
pub(super) const P_POWER_ENDOMORPHISM_COEFF_1: Fq2 = Fq2::new(
    MontFp!("2821565182194536844548159561693502659359617185244120367078079554186484126554"),
    MontFp!("3505843767911556378687030309984248845540243509899259641013678093033130930403"),
);

// Integer representation of 6x^2 = t - 1
#[cfg(test)]
const SIX_X_SQUARED: [u64; 2] = [17887900258952609094, 8020209761171036667];

/// The subgroup check from section 4.3 of https://eprint.iacr.org/2022/352.pdf, `[p]P = [6X^2]P`,
/// kept as the reference of the in-place one in `g2_subgroup`
#[cfg(test)]
pub(crate) fn is_in_subgroup_reference(point: &G2Affine) -> bool {
    let x_times_point = point.mul_bigint(SIX_X_SQUARED);
    let p_times_point = p_power_endomorphism(point);
    x_times_point.eq(&p_times_point)
}

/// psi(P) is the untwist-Frobenius-twist endomorphism on E'(Fq2)
#[cfg(test)]
fn p_power_endomorphism(p: &Affine<Config>) -> Affine<Config> {
    // Maps (x,y) -> (x^p * (u+9)^((p-1)/3), y^p * (u+9)^((p-1)/2))

    let mut res = *p;
    res.x.frobenius_map_in_place(1);
    res.y.frobenius_map_in_place(1);

    res.x *= P_POWER_ENDOMORPHISM_COEFF_0;
    res.y *= P_POWER_ENDOMORPHISM_COEFF_1;

    res
}
