//! Pairing checks without the final exponentiation: the "residue witness" of
//!
//! - A. Novakovic, L. Eagen, "On Proving Pairings", IACR ePrint 2024/640,
//!   <https://eprint.iacr.org/2024/640> (Section 3: the check, Section 4: the BN254 instance,
//!   Algorithm 4: the cube root with `3^n | p^k - 1`),
//! - gnark's in-circuit implementation of it for BN254 and BLS12-381
//!   (`std/algebra/emulated/sw_bn254/{pairing,hints}.go` and `sw_bls12381/{pairing,hints}.go`,
//!   <https://github.com/Consensys/gnark>), which this module follows, including the BLS12-381
//!   instance that gnark credits to a personal communication with Novakovic.
//!
//! # The check
//!
//! Let `h = (p^12 - 1) / r`. A product `f` of Miller functions (of pairs in the prime-order
//! groups) is the pairing identity iff `f^h = 1`, i.e. iff `f` is an `r`-th residue in `Fp12^*`
//! (`f^h = 1 <=> f = g^r` for some `g`, as the multiplicative group is cyclic of order
//! `r h`). The final exponentiation computes `f^h`; the residue check instead verifies
//!
//! `f s = c^λ`
//!
//! for a witness `c` (given as `d = c^-1`, and for BN254 also `c`) and a "scaling factor"
//! `s` supplied by the prover, with a fixed `λ` divisible by `r`, and `s` restricted to the
//! subfield `Fp6`.
//!
//! # Why the check is sound
//!
//! If `f s = c^λ` holds for *any* `c` in `Fp12^*` and *any* `s` in `Fp6^*` then `f` is an
//! `r`-th residue:
//! - `c^λ` is an `r`-th residue, as `r | λ`: `c^λ = (c^(λ/r))^r`;
//! - `s` is an `r`-th residue, as every element of `Fp6^*` is: its order divides `p^6 - 1`,
//!   and `p^6 - 1` divides `h`, because `r` divides `p^6 + 1` (`r | Φ_12(p) = p^4 - p^2 + 1`
//!   and `p^6 + 1 = (p^2 + 1)(p^4 - p^2 + 1)`), so `h = (p^6 - 1)(p^6 + 1)/r` is a multiple
//!   of `p^6 - 1` and `s^h = 1`;
//! - hence `f = c^λ s^-1` is an `r`-th residue, i.e. `f^h = 1`.
//!
//! No property of `c`, `s` beyond their membership in `Fp12^*` and `Fp6^*` is used: a prover
//! cannot make a non-identity `f` pass. Membership in `Fp6` is enforced by construction (the
//! verifier builds `s` from six base field elements as an `Fp12` element with a zero `c1`),
//! and `c` may be anything.
//!
//! # What the check does not prove
//!
//! A failed check proves nothing: the witness comes from the prover, and a wrong or withheld
//! one fails the check for an identity `f` as well. A verifier whose output must not be the
//! prover's choice (the EVM precompiles: their result is part of the state transition) treats
//! "passed" as the only trusted answer and settles a failed check with the exact final
//! exponentiation (`final_exponentiation_with_inverse` with a checked inversion hint). An
//! honest prover never pays for that on an identity, and a dishonest one only slows its own
//! proof down. This is the difference from a plain pairing-equality proof, where a failed
//! check is simply a rejected proof.
//!
//! # Why an honest prover can always pass (completeness)
//!
//! Write `h = h_λ h'` where `h'` is the largest divisor of `h` coprime to `λ` (`h_λ` collects
//! the prime powers of `h` that `λ` shares: `3^3 · POLY` for BLS12-381, `3^3` for BN254). For
//! an identity `f` (`f^h = 1`) the prover finds `s` with `(f s)^h' = 1`: the elements of order
//! dividing `h_λ` live in `Fp6` for both curves (`h_λ | p^6 - 1`), and `s` is the inverse of
//! the `h_λ`-part of `f` (a power of `f`, computed with the exponents below). Then
//! `c = (f s)^(λ^-1 mod h')` satisfies `c^λ = (f s)^(1 + k h') = f s`. For BN254, `λ = 3 r m`
//! with `gcd(m, h) = 1` and `3^3 || h`: the `r`-th and `m`-th roots are exponentiations
//! (`r^-1 mod h`, `m^-1 mod h`) and the cube root needs the scaling by a 27-th root of unity
//! `ω` (a cubic non-residue) that makes `f ω^i` a cube, plus a Tonelli-Shanks step (Alg. 4).
//!
//! # Why `c^λ` is almost free
//!
//! `λ` is `p - u` for BLS12-381 (`u` the seed, negative) and `6x + 2 + p - p^2 + p^3` for
//! BN254 (`x` the seed): its low part is the Miller loop count, and the loop, started with `d`
//! instead of `1` in its accumulator and multiplied by `d` (or `c`) at the non-zero digits of
//! the loop count, computes `d^loop f` by itself (it runs the double-and-add of the exponent;
//! the squarings are the loop's own, the extra multiplications are one per non-zero digit).
//! The `p`-powers are Frobenius maps. The verifier's work beyond the loop is three
//! multiplications for BN254 and two for BLS12-381, instead of the ~4-5k field multiplications
//! of the final exponentiation.
//!
//! Prover side: [`bn254::witness`], [`bls12_381::witness`] (arkworks arithmetic, host only in
//! practice). Verifier side: `multi_miller_loop_with_initial` of the pairing implementations,
//! then [`bn254::check`], [`bls12_381::check`].

pub mod bn254 {
    //! `λ = 6x + 2 + p - p² + p³ = 3 r m` (Section 4 of the paper; `λ = 0 mod r` because
    //! `p = 6x + 2 + ... mod r` for BN curves) with `gcd(m, h) = 1`, and `h = 27 l` with
    //! `3 ∤ l`. The scaling factor is a power of a 27-th root of unity `ω` in `Fp6` (a cubic
    //! non-residue, as `27 | h`), the one of `1, ω, ω²` that makes `f ω^i` a cubic residue;
    //! then `c = (f ω^i)^(1 / (r m))` (exponentiations by `r^-1 mod h`, `m^-1 mod h`) followed
    //! by a cube root (a Tonelli-Shanks variant, Alg. 4 of the paper), so that
    //! `c^(3 r m) = f ω^i`. The same construction as gnark's `finalExpWitness` for BN254.
    use crate::bn254::{Fq, Fq12, Fq2, Fq6};
    use ark_ff::{Field, One, Zero};

    /// The Miller loop count `6x + 2`, the low part of `λ`
    pub const SIX_X_PLUS_2: &[u64] = &[0x9d797039be763ba8, 0x1];
    /// `λ = 6x + 2 + p - p^2 + p^3`
    pub const LAMBDA: &[u64] = &[
        0x57f83d3135ae0c55,
        0xfaf174e7e839ac91,
        0xb136a4fdd951c142,
        0xeb9b42c6f8f8e030,
        0x788e401a57040c54,
        0xbf0519bc6772f06e,
        0xab87216b02105ec9,
        0xdd45cf150c7e2d75,
        0xc2421c372dee2ddc,
        0x68148fd2e5e487f1,
        0x331ec15183177faf,
        0x1baaa710b0759ad,
    ];

    /// `(p^12 - 1) / 3`: `f^EXP1 = 1` iff `f` is a cubic residue
    pub const EXP1: &[u64] = &[
        0xeb46f64643825060,
        0xc09504ce57838ff3,
        0xb6973b1dfda111a7,
        0x9e6a6b1d46fc408c,
        0x745bdaf039c199f6,
        0xe9f65a41395df713,
        0x4dcd3d267739953c,
        0x9f49699c7d2e3b27,
        0xb189f37c0ecd514e,
        0x55aa926463b3f1ad,
        0x6030fad438f67304,
        0x1dc6e7821edb8a5c,
        0x3fabe2a396c821ee,
        0xce442caa65704817,
        0xac5266c00ed4ded7,
        0x53aa9ef14c0ae51f,
        0x133df7ebbc224e97,
        0x88ce9faea263de92,
        0x8c4be6bdd2b88017,
        0x628d5a19e9c247d9,
        0xa93bf3094d3d5518,
        0x3939f77b19cd8e05,
        0x3c85c4759d907006,
        0xf47559371ceb7cb4,
        0x9868d7443cc60fe8,
        0x591589f02cf8ecb7,
        0x680fa342f7100bba,
        0xb44b431aae371e85,
        0x99625bea8196289d,
        0xa38d36e079b35749,
        0x8d38b7277eb44ec,
        0xb5de835af494b061,
        0x370bd1df6206ad8e,
        0xf755226d1fb5139f,
        0xedafa93168993756,
        0x5b43e8559e471ed9,
        0xe84ed08d6375382d,
        0x9b99a5c06b47a88a,
        0x19e45304da068978,
        0x12aff3b863cdce2f,
        0xb0178e622c3aaf92,
        0x19e6b3b6373de8df,
        0xeb4cec3eff8e12f1,
        0xc3fc152a73114859,
        0xd516d062f8015f36,
        0x6440dd3153897c68,
        0x73924a5d67a5d259,
        0x2fae42e49,
    ];
    /// `r^-1 mod h`
    pub const R_INV: &[u64] = &[
        0xa11b194379daaca1,
        0xa5f0d2dcad831751,
        0x2b410d5c2e47c1b4,
        0x384a4274ed212efc,
        0x70aac1f263098d89,
        0xf5fd95b7c6784e,
        0x7ebfe5d9a66b49bb,
        0x1c4d0568f9a146bb,
        0xc647b55ab5455a1f,
        0x315372b248a33b03,
        0xaa284fca9651ce25,
        0x1f401dc214ffb6d3,
        0x65cfacd3e5294c64,
        0xcce86cb5c9a967d9,
        0xd457e4e53e766cdf,
        0xc3c33ad27ef8286d,
        0xb0c4571cbda9e9b9,
        0x26024e519498470,
        0xb517826e8d0a7ea3,
        0xfe0b42fd3f9e4cb6,
        0xdf7db6f0f880457b,
        0x182418ed6397d5da,
        0x4dbb3c5f77a5e53d,
        0xd8252918fdce3f1e,
        0xfd9bd55b098443f5,
        0xf8a2e6743a1e2509,
        0xfe45c6ba38a2a7d2,
        0x379e03a9a5e228c5,
        0xc190b44c4eaa05ef,
        0xe326d9499c2238ca,
        0x68e0b9719edade8,
        0x51acfb2c2b91395f,
        0x23575e4b046e4551,
        0x28206b1dd3e111f2,
        0xee7b16d7001a28fd,
        0xcc9f2cef8aa8b968,
        0x82165fad421d6efa,
        0xe0d2744ba5f78583,
        0xa5fa4275e36247c2,
        0x847f3dae4767238b,
        0x4c2fca934b0d67ba,
        0xf17857b96814f37b,
        0x79815b691a6a294e,
        0x2a71a42aee,
    ];
    /// `m^-1 mod h`, `m = λ / (3 r)`
    pub const M_INV: &[u64] = &[
        0xf7721c7aff2f56b7,
        0xa41c1525e62392af,
        0xf08969a9108c577f,
        0x843248b70bc58b9b,
        0x72d7a403a1e679c,
        0x88c5e5a473bf4dc0,
        0x51c8b0e8579cc523,
        0xb459b999d3ac8a7d,
        0x18fa924d8034a528,
        0xc55e1e6afd4f17a6,
        0xb3667969b57a1a59,
        0x38188d68645ec977,
        0x4ce226be79e0093e,
        0x93f8674a5444d2ec,
        0x111b573624f772f3,
        0x349070d501f354a4,
        0xff1e01c9b766a835,
        0x4d90962b1fc6ee8f,
        0xb6fe4cd038239172,
        0xe82a55d5a62a0447,
        0x2ddd7fea7f5d2359,
        0xeebee1afc60f5209,
        0x8a10190cff16e8e4,
        0x9d001574cb7631fa,
        0xd0f5585b4ca70cc,
        0x496f56776b19ac89,
        0x91f3035f8757e549,
        0xfd3d06751c3e97a3,
        0xe51a66ac9a83c506,
        0xdf0ea52e1e84aa13,
        0xce20f6fcdca29467,
        0x8591195a1879549f,
        0x5261846036719b2e,
        0xdec7495884c546a9,
        0x2d6eb7558005a1ee,
        0xe534278298cad674,
        0xa29b3e0abf45fbcb,
        0xbd4341f0e9cc796,
        0xf544c6143b686a58,
        0x5d32122cd9a4aa34,
        0xadc0eee93555b6d4,
        0x715424f40cb25186,
        0x8dd1549b7e60deaa,
        0x186f601e0,
    ];
    /// `(s + 1) / 3` with `p^12 - 1 = 27 s`, the exponent of the cube root candidate
    pub const EXP2: &[u64] = &[
        0x7102a0d331e861cb,
        0x1a187b6ff0473e38,
        0xcddfacdb2f51d13f,
        0x483cd48f4e7b1ed5,
        0xd4e6f5255778f2bd,
        0x83ecae026a6bc6c7,
        0x911a907caf15187d,
        0xe9747f2bb8c8d2c8,
        0x69354deab370302,
        0x61fcd603b7d741d7,
        0xcaac7b1157716c8e,
        0xb540417698d8b945,
        0x6aa78d2280d80141,
        0x4a028665203390e4,
        0x6eadb7f42679a970,
        0x586e9d9728be087c,
        0xedbfecbce10ac08a,
        0x1807a7196e4f8cfb,
        0xdf452e78ceea638f,
        0xb7cc58aba05c8766,
        0xb0ef41e3e66a915f,
        0x3b02259c43537709,
        0x31a623b881185000,
        0x4b6ca47d4cec46fd,
        0xd63cc59a3b23c7b3,
        0x7513c2bd0b25a9f3,
        0xdded9dc01c1d09ea,
        0xcdc9e60a783aee2a,
        0x9d62752e9c80d218,
        0x944799bc7649033b,
        0xed5d2b1733d94e67,
        0x19b2e86baa3e6558,
        0xe5982437ae4c1964,
        0xffadd1de1d9e6905,
        0x253f6514cafc3174,
        0x58b6a9ca483b85e2,
        0xe2ad95f2460dd2ac,
        0x8a80f32d0d746e89,
        0xe483b739118e76de,
        0x4c8b41ea6282e1b5,
        0x81c7fbcabf448b3e,
        0x4ccfa7d757611b96,
        0x2528c66125e8d14b,
        0x6612d16063139a62,
        0xd87c1aae55098844,
        0x628724a303180e16,
        0x33b015b79b8ae1dd,
        0x1c41570c,
    ];

    /// A 27-th root of unity in `Fp6` (`(0, c010, c011, 0, 0, 0)` in the tower), necessarily a
    /// cubic non-residue: `w^((p^12 - 1) / 27)` for the generator `w` of `Fp12 = Fp[w] / (w^12
    /// - 18 w^6 + 82)`, mapped to the tower (from gnark)
    pub fn root_27th_of_unity() -> Fq12 {
        use ark_ff::PrimeField;
        let fq = |limbs: [u64; 4]| {
            let mut repr = <Fq as PrimeField>::BigInt::default();
            repr.as_mut()[..4].copy_from_slice(&limbs);
            Fq::from_bigint(repr).expect("a canonical constant")
        };
        // 9483667112135124394372960210728142145589475128897916459350428495526310884707
        let c010 = fq([
            0xbf9094793132b563,
            0x992478d2aeef5cde,
            0x862867ef12a24eb7,
            0x14f790bbd583653f,
        ]);
        // 4534159768373982659291990808346042891252278737770656686799127720849666919525
        let c011 = fq([
            0x4c0093fe44aacc65,
            0x44c8873b8927d16d,
            0x4f84e31d49aece1f,
            0xa063e5502b196f6,
        ]);
        Fq12::new(
            Fq6::new(Fq2::zero(), Fq2::new(c010, c011), Fq2::zero()),
            Fq6::zero(),
        )
    }

    /// The witness `(c, d = c^-1, s)` of the Miller loop output `f` (the product over all
    /// pairs, as `multi_miller_loop` computes it): `f s = c^λ`. `None` if `f` is not the
    /// pairing identity (then no witness exists: every step below is bounded and checked, so
    /// a non-identity is detected rather than looped on). The loop needs both `d` (for the `1`
    /// digits) and `c` (for the `-1` digits of the NAF loop count).
    pub fn witness(f: &Fq12) -> Option<(Fq12, Fq12, Fq6)> {
        let omega = root_27th_of_unity();
        // the power of ω that makes f ω^i a cubic residue
        let mut scaled = *f;
        let mut scaling = Fq12::one();
        let mut is_cube = false;
        for _ in 0..3 {
            if scaled.pow(EXP1).is_one() {
                is_cube = true;
                break;
            }
            scaled *= &omega;
            scaling *= &omega;
        }
        if !is_cube {
            return None;
        }
        // (r m)-th root
        let root = scaled.pow(R_INV).pow(M_INV);
        // cube root, Tonelli-Shanks with 3^3 | p^12 - 1 (Alg. 4 of the paper, as in gnark):
        // x^3 / root has order 3^t, t <= 3, and the candidate is corrected by ω^EXP2 until
        // t = 0
        let root_inv = root.inverse()?;
        let mut x = root.pow(EXP2);
        let omega_exp2 = omega.pow(EXP2);
        let order = |x: &Fq12| -> Option<u32> {
            let mut x3 = x.square() * x * root_inv;
            let mut t = 0;
            while !x3.is_one() {
                if t == 3 {
                    return None;
                }
                t += 1;
                x3 = x3.square() * x3;
            }
            Some(t)
        };
        // the candidate is off by a 27-th root of unity: at most 26 steps through the coset
        let mut found = false;
        for _ in 0..27 {
            if order(&x)? == 0 {
                found = true;
                break;
            }
            x *= &omega_exp2;
        }
        if !found {
            return None;
        }
        let c = x;
        if !scaling.c1.is_zero() {
            return None;
        }
        // f s = c^λ must hold; it does for an identity, and never for anything else
        let s = scaling.c0;
        if *f * Fq12::new(s, Fq6::zero()) != c.pow(LAMBDA) {
            return None;
        }
        Some((c, c.inverse()?, s))
    }

    /// The check on `l`, the Miller loop output with the accumulator started at `d = c^-1`
    /// (`l = d^(6x+2) f`, see `Bn254::multi_miller_loop_with_initial`): `f s = c^λ` with
    /// `λ = 6x + 2 + p - p^2 + p^3` is `f s = d^-(6x+2) d^-p d^(p^2) d^-(p^3)`, i.e. without an
    /// inversion `l s d^p d^(p^3) = d^(p^2)`. Sound for any `d` in `Fp12^*` and `s` in `Fp6^*`
    /// (see the module documentation); `s` is in `Fp6` by construction.
    pub fn check(l: &Fq12, d: &Fq12, s: &Fq6) -> bool {
        let mut left = *l * Fq12::new(*s, Fq6::zero());
        let mut t = *d;
        t.frobenius_map_in_place(1);
        left *= &t;
        let mut t = *d;
        t.frobenius_map_in_place(3);
        left *= &t;
        let mut right = *d;
        right.frobenius_map_in_place(2);
        left == right
    }
}

pub mod bls12_381 {
    //! `λ = p - u` (`u < 0` the seed): as `p = (u - 1)^2 r / 3 + u`, `λ = 3 POLY^2 r` with
    //! `POLY = (1 - u) / 3` (a 63-bit integer), so `r | λ`. `h = 27 · POLY · FEF` with `FEF`
    //! coprime to `λ` (checked offline, together with `27 POLY | p^6 - 1`: the roots of unity
    //! of order dividing `27 POLY` lie in `Fp6`). The prover's scaling factor `s` cancels the
    //! parts of `f` of order dividing `POLY` and `27`, so that `f s` has order dividing `FEF`,
    //! and then `c = (f s)^(λ^-1 mod FEF)` gives `c^λ = f s`. The same construction as gnark's
    //! `finalExpWitness` for BLS12-381.
    use crate::bls12_381::{Fq12, Fq6};
    use ark_ff::{Field, One, Zero};

    /// `27 FEF`
    pub const E27F: &[u64] = &[
        0x1096726fbab05f30,
        0x321ff61f21fd7812,
        0x2523b2d3d6819cff,
        0x15ed8bb794286672,
        0x140a18b448f0b31d,
        0x6c4b07f128c43298,
        0xe083e4a3dbc2cbb,
        0x5f8b72a054921010,
        0xf4e6d2e21767bd0e,
        0x6beb55720212f0e3,
        0xb9988b3db5fb7b9d,
        0xc0737c1e17b89511,
        0x658593664b88450a,
        0x427ce3f7b73683cd,
        0x51fae88e1e3c057a,
        0x7ee35190408f52a4,
        0x77f54deddd50267,
        0x82c1574ece29c483,
        0xa03c32f56f193726,
        0x6a5e631f639160f,
        0x6031a51a78b1911a,
        0x870116d70b5e848f,
        0xb8792f625e35108,
        0xa33efa7447198c10,
        0x6d8d6fb12f699721,
        0xe1118f0fd018653f,
        0x3d29e65f84506d0d,
        0x250ab8ed7c42f265,
        0x25c7254c550afb22,
        0x382bea267a4342b1,
        0x13c5b5cdd0aef59c,
        0xb5668ca737d04a19,
        0xc675032cbb28f3a7,
        0x7a104b1cc61409f4,
        0x92c8edc8634b555c,
        0xa5d6dcba6631a1d5,
        0x26d798b8d26fd9d3,
        0xf18e81fdda344984,
        0x5f440f80c2c81309,
        0x10552f5e2c73f520,
        0x171219f94913b05a,
        0x9f230502fe488faf,
        0x89219dae9fc4c3d9,
        0x94a85c4ecef1aece,
        0x10c1f741e2da9137,
        0xe959e476df255af0,
        0x7e8df723673c98c0,
        0x5354765daa3038a2,
        0x76a5efff63342428,
        0x8ba8bb23d8126714,
        0x769c56ca8132a6ba,
        0x3af978bc554efc95,
        0x2ef219cbe26cd01d,
        0x9bc57ac353c87bd7,
        0x45914bfad0be3423,
        0xe3a142ea466ab63a,
        0x6d5d92b1b34b0d56,
        0x7c8b71832a557b93,
        0x9973c676250dc3d6,
        0x11c5c3b349edc2a6,
        0x935a81b0dda6a39d,
        0x31204d6cfcf9a4c,
        0xf8635e4e65ee8f57,
        0xf8d8d17ce2a72e13,
        0x90b280cd1c401824,
        0x4fa23def17d5c3fa,
        0xab73b05,
    ];
    /// `-(27 FEF)^-1 mod POLY`
    pub const E27F_NEG_INV_MOD_POLY: &[u64] = &[0x210e612f684c025f];
    /// `POLY FEF`
    pub const EPF: &[u64] = &[
        0x65f406e0ca975030,
        0xa29776bf70d5c6ee,
        0x90bde366423754e8,
        0xd5dbd580a8ee47c3,
        0x908a926c09de3429,
        0x569bb5e398c9f363,
        0x29c6c23b859a6430,
        0x3f612af78cc293a9,
        0x1d6c24e9712439b5,
        0xff06cc504fc4198f,
        0x2f0c93ab68742106,
        0x33b3c87d54d4be9e,
        0xbaaa23e42e91dbbf,
        0xd214dac9d23f353,
        0x737d0253108af8c8,
        0x6e6637fc6ed25ba5,
        0x7f4516a5eb8a4ce2,
        0x1cc63e082e898928,
        0x1febffb0b7d647c,
        0x3530812a86929402,
        0x6818d1530c650ab1,
        0xdc6f9b9a5b686b89,
        0x4878f3cf4bb795e0,
        0xe4953dacaed465cd,
        0x390a64e2ed791de7,
        0x451a0f3d194224be,
        0x28edb985179e7230,
        0x1f2ac9b2087d0ec8,
        0xb83473198a94a92d,
        0xd60002d966b75adb,
        0xefde123b3687126d,
        0x264d06697b6b24fc,
        0xe162442fc539b02b,
        0xeeeff9238fdc68bb,
        0x610358ff3ad7b760,
        0x501586d30113cb81,
        0xdac226f066875744,
        0xf0a746b1fdf06856,
        0xafece8d217f5e91f,
        0xc3327d08dca713a3,
        0x2854aeddfe2397da,
        0xcce5f334b17494d2,
        0x69ab50df71593242,
        0x3c3366057937497c,
        0xc7b7def1bf168ee4,
        0xaaf1c6eda843bc5d,
        0xb1f2f042ceb5bdb3,
        0x26e40367c9ef1593,
        0xbf983586b9eaef36,
        0x4a29be21ec07fceb,
        0x2447b9d76c5fcddd,
        0x693ac26afc21b5be,
        0xb139a0be60e78547,
        0x5f18ebbc8868ccd6,
        0x521dec236446b60a,
        0x5445064fba90efa5,
        0xf78e9f70397f6784,
        0xfdf14b344bd2be2f,
        0xa174ad2ccd45f1e6,
        0xc4c3f554f5b25e2b,
        0x27b80ec2d7fec40f,
        0x4a1cb633cd73bff8,
        0xf763b913d7836a74,
        0xfacace05f8df4c9d,
        0x9f93d52847028e82,
        0xf93cd1a19354aab9,
        0x249f1dd7672cf352,
        0x1bc836,
    ];
    /// `-(POLY FEF)^-1 mod 3`
    pub const EPF_NEG_INV_MOD_3: &[u64] = &[0x2];
    /// `-(POLY FEF)^-1 mod 9`
    pub const EPF_NEG_INV_MOD_9: &[u64] = &[0x8];
    /// `-(POLY FEF)^-1 mod 27`
    pub const EPF_NEG_INV_MOD_27: &[u64] = &[0x11];
    /// `λ = p - u`
    pub const LAMBDA: &[u64] = &[
        0x8c0000000000aaab,
        0x1eabfffeb1540000,
        0x6730d2a0f6b0f624,
        0x64774b84f38512bf,
        0x4b1ba7b6434bacd7,
        0x1a0111ea397fe69a,
    ];
    /// `λ^-1 mod FEF`
    pub const LAMBDA_INV_MOD_FEF: &[u64] = &[
        0xb6e3fefaaf6c23,
        0xba50f7521d495337,
        0x8fa40e48bef9c692,
        0xd0cabc76f961074d,
        0x805168994fad7512,
        0x6aa1c8c9776871c3,
        0x2a68a4a67f55a88e,
        0x9b798d4030a925b3,
        0xbd017f6f55031685,
        0x5941c1d1cc14bb00,
        0x8cab8b6b547ac1b1,
        0xe8ba0fa4956270d9,
        0x402e2ea408490f83,
        0xe18f6141e75e6151,
        0x46944b9ccc52a999,
        0x2d1a56bba3476aeb,
        0x2884afb4610957be,
        0xc7f1ca9e35adf3a,
        0x25a8b21e58a3059d,
        0x23e0efdaa215a695,
        0xbe22bd36eae8284d,
        0x4941f4877fe7756e,
        0xed8552dabf485b53,
        0x3d6f68b235ee04c1,
        0xfba66f18e3f07980,
        0x9f0afd39b4ffa61a,
        0x770409ca39863bc3,
        0x1ec1adbd1d7fd6b0,
        0x27919600415dabdb,
        0xf2e39dc4a2ef8538,
        0x2c1761964cb5c1f1,
        0x7cc854792a8a9c52,
        0x10e35f0ec2b3fc7a,
        0xaf9e06161c9a4afc,
        0x6835191eddcb197f,
        0x851646957b8a14f5,
        0x23fb3f871de4d8e5,
        0xb41c4306d335cb6,
        0x3c5be4d379339f3,
        0x7dc912d6e44a7fab,
        0x6d087dcd3618e42,
        0x64e6c398c568675b,
        0x5893e10ca8e48731,
        0xe9d43d3b7ad9af62,
        0x67bcb0d7498e2482,
        0xacaab294577b5f17,
        0xf82b66fd5a502089,
        0x7ffac9d4f6359297,
        0x2c27b262ebbdcb0c,
        0xf1bd69c020b89006,
        0x987f26d6b7de5b55,
        0x7a07bc5b60e1ed38,
        0x362f49e4dd57cf6c,
        0xd5d230c454517eb4,
        0x7b93fc9ccc5cd0d8,
        0x89def3e07ff5e35a,
        0xa70dcd395814cc0c,
        0xb18f69bd487a02af,
        0x66c5b338d11e220,
        0xa8bba4f42f4d8974,
        0x75e528ac2412d477,
        0xd46fda1e16fb9588,
        0xfc4e91d7468d4790,
        0x8a72692b8d18d58e,
        0x22142ac1801c949d,
        0xcb940d75d40a0772,
        0x4ea48c,
    ];

    /// The witness `(d = c^-1, s)` of `f`, the product of the Miller functions `f_|u|` over all
    /// pairs (without the conjugation that stands for the negative seed: it changes `f` by an
    /// `r`-th residue only). `f s = c^λ`. `None` if `f` is not the pairing identity (then no
    /// witness exists: every step below is bounded and checked, so a non-identity is detected
    /// rather than looped on).
    pub fn witness(f: &Fq12) -> Option<(Fq12, Fq6)> {
        // 1. the POLY-th root part: root = f^(27 FEF) has order dividing POLY
        let root = f.pow(E27F);
        let root_poly_inverse = if root.is_one() {
            Fq12::one()
        } else {
            root.pow(E27F_NEG_INV_MOD_POLY)
        };
        // 2. the 27-th root part: root = f^(POLY FEF) has order 3^k, k <= 3
        let root = f.pow(EPF);
        let root_27th_inverse = if root.is_one() {
            Fq12::one()
        } else {
            let mut order = 0;
            let mut r = root;
            while !r.is_one() {
                if order == 3 {
                    return None;
                }
                order += 1;
                r = r.square() * r;
            }
            let exponent = match order {
                1 => EPF_NEG_INV_MOD_3,
                2 => EPF_NEG_INV_MOD_9,
                _ => EPF_NEG_INV_MOD_27,
            };
            root.pow(exponent)
        };
        let scaling = root_poly_inverse * root_27th_inverse;
        if !scaling.c1.is_zero() {
            return None;
        }
        // 3. f s has order dividing FEF, coprime to λ
        let scaled = *f * scaling;
        let c = scaled.pow(LAMBDA_INV_MOD_FEF);
        // f s = c^λ must hold; it does for an identity, and never for anything else
        if c.pow(LAMBDA) != scaled {
            return None;
        }
        Some((c.inverse()?, scaling.c0))
    }

    /// The check on `l`, the Miller loop output with the accumulator started at `d = c^-1`
    /// (`l = d^|u| f` without the conjugation of the negative seed, see
    /// `Bls12_381::multi_miller_loop_with_initial`): `f s = c^λ` with `λ = p - u = p + |u|` is
    /// `l s d^p = 1`. Sound for any `d` in `Fp12^*` and `s` in `Fp6^*` (see the module
    /// documentation); `s` is in `Fp6` by construction. The conjugation is not needed because
    /// it changes `f` by `f^(p^6 - 1)`, an `r`-th residue, which does not change whether `f`
    /// is one.
    pub fn check(l: &Fq12, d: &Fq12, s: &Fq6) -> bool {
        let mut t = *d;
        t.frobenius_map_in_place(1);
        (*l * Fq12::new(*s, Fq6::zero()) * t).is_one()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::pairing::Pairing;
    use ark_ec::{CurveGroup, PrimeGroup};
    use ark_ff::{Field, One, Zero};

    /// `λ` of bn254 and of bls12-381 (as the modules' constants), for the direct check
    /// `f s = c^λ`
    const BN254_LAMBDA: &[u64] = &[
        0x57f83d3135ae0c55,
        0xfaf174e7e839ac91,
        0xb136a4fdd951c142,
        0xeb9b42c6f8f8e030,
        0x788e401a57040c54,
        0xbf0519bc6772f06e,
        0xab87216b02105ec9,
        0xdd45cf150c7e2d75,
        0xc2421c372dee2ddc,
        0x68148fd2e5e487f1,
        0x331ec15183177faf,
        0x1baaa710b0759ad,
    ];
    const BLS12_381_LAMBDA: &[u64] = &[
        0x8c0000000000aaab,
        0x1eabfffeb1540000,
        0x6730d2a0f6b0f624,
        0x64774b84f38512bf,
        0x4b1ba7b6434bacd7,
        0x1a0111ea397fe69a,
    ];

    fn bn254_points(k: u64) -> (crate::bn254::G1Affine, crate::bn254::G2Affine) {
        let p =
            (crate::bn254::G1Projective::generator() * crate::bn254::Fr::from(k + 3)).into_affine();
        let q = (crate::bn254::G2Projective::generator() * crate::bn254::Fr::from(2 * k + 5))
            .into_affine();
        (p, q)
    }

    #[test]
    fn bn254_root_of_unity() {
        let w = bn254::root_27th_of_unity();
        assert!(w.pow([27u64]).is_one());
        assert!(!w.pow([9u64]).is_one());
        // a cubic non-residue
        assert!(!w.pow(bn254::EXP1).is_one());
    }

    #[test]
    fn bn254_identity_passes_and_matches_lambda() {
        use crate::bn254::curves::Bn254;
        for k in 0..3 {
            let (p, q) = bn254_points(k);
            let pairs_g1 = [p, -p];
            let pairs_g2 = [q, q];
            let f = Bn254::multi_miller_loop(pairs_g1, pairs_g2).0;
            assert!(
                Bn254::final_exponentiation(ark_ec::pairing::MillerLoopOutput(f))
                    .unwrap()
                    .0
                    .is_one()
            );
            let (c, d, s) = bn254::witness(&f).expect("an identity has a witness");
            assert!((c * d).is_one());
            // f s = c^λ
            assert_eq!(
                f * crate::bn254::Fq12::new(s, crate::bn254::Fq6::zero()),
                c.pow(BN254_LAMBDA)
            );
            let l = Bn254::multi_miller_loop_with_initial(
                &d,
                &c,
                pairs_g1,
                pairs_g2
                    .iter()
                    .map(|q| crate::bn254::curves::G2PreparedNoAlloc::from(*q)),
            );
            assert_eq!(l, d.pow(bn254::SIX_X_PLUS_2) * f);
            assert!(bn254::check(&l, &d, &s));
            // a wrong witness fails
            assert!(!bn254::check(&l, &(d * c * c), &s));
        }
    }

    #[test]
    fn bn254_non_identity_fails() {
        use crate::bn254::curves::Bn254;
        for k in 0..3 {
            let (p, q) = bn254_points(k);
            let f = Bn254::multi_miller_loop([p], [q]).0;
            assert!(
                !Bn254::final_exponentiation(ark_ec::pairing::MillerLoopOutput(f))
                    .unwrap()
                    .0
                    .is_one()
            );
            assert!(
                bn254::witness(&f).is_none(),
                "no witness for a non-identity"
            );
            // nor does any other value pass the check
            let (c, d, s) = (f, f.inverse().unwrap(), f.c0);
            let l = Bn254::multi_miller_loop_with_initial(
                &d,
                &c,
                [p],
                [crate::bn254::curves::G2PreparedNoAlloc::from(q)],
            );
            assert!(!bn254::check(&l, &d, &s));
        }
    }

    fn bls_points(k: u64) -> (crate::bls12_381::G1Affine, crate::bls12_381::G2Affine) {
        let p = (crate::bls12_381::G1Projective::generator() * crate::bls12_381::Fr::from(k + 3))
            .into_affine();
        let q = (crate::bls12_381::G2Projective::generator()
            * crate::bls12_381::Fr::from(2 * k + 5))
        .into_affine();
        (p, q)
    }

    #[test]
    fn bls12_381_identity_passes_and_matches_lambda() {
        use crate::bls12_381::curves::Bls12_381;
        for k in 0..2 {
            let (p, q) = bls_points(k);
            let pairs_g1 = [p, -p];
            let pairs_g2 = [q, q];
            let conjugated = Bls12_381::multi_miller_loop(pairs_g1, pairs_g2).0;
            assert!(
                Bls12_381::final_exponentiation(ark_ec::pairing::MillerLoopOutput(conjugated))
                    .unwrap()
                    .0
                    .is_one()
            );
            let prepared_g2 = || {
                pairs_g2
                    .iter()
                    .map(|q| crate::bls12_381::curves::G2PreparedNoAlloc::from(*q))
            };
            let f = Bls12_381::multi_miller_loop_with_initial(
                &crate::bls12_381::Fq12::one(),
                pairs_g1,
                prepared_g2(),
            );
            let mut f_conj = f;
            f_conj.conjugate_in_place();
            assert_eq!(f_conj, conjugated);
            let (d, s) = bls12_381::witness(&f).expect("an identity has a witness");
            let c = d.inverse().unwrap();
            assert_eq!(
                f * crate::bls12_381::Fq12::new(s, crate::bls12_381::Fq6::zero()),
                c.pow(BLS12_381_LAMBDA)
            );
            let l = Bls12_381::multi_miller_loop_with_initial(&d, pairs_g1, prepared_g2());
            assert_eq!(
                l,
                d.pow(<crate::bls12_381::curves::Config as ark_ec::bls12::Bls12Config>::X) * f
            );
            assert!(bls12_381::check(&l, &d, &s));
            assert!(!bls12_381::check(&l, &(d * c * c), &s));
        }
    }

    #[test]
    fn bls12_381_non_identity_fails() {
        use crate::bls12_381::curves::Bls12_381;
        for k in 0..2 {
            let (p, q) = bls_points(k);
            let q = crate::bls12_381::curves::G2PreparedNoAlloc::from(q);
            let f = Bls12_381::multi_miller_loop_with_initial(
                &crate::bls12_381::Fq12::one(),
                [p],
                [&q],
            );
            assert!(
                bls12_381::witness(&f).is_none(),
                "no witness for a non-identity"
            );
            let (d, s) = (f.inverse().unwrap(), f.c0);
            let l = Bls12_381::multi_miller_loop_with_initial(&d, [p], [&q]);
            assert!(!bls12_381::check(&l, &d, &s));
        }
    }

    #[test]
    fn bn254_witness_steps() {
        use crate::bn254::curves::Bn254;
        const M: &[u64] = &[
            0xdabb369b81e4aec7,
            0xe5494e0008c77545,
            0x3f56bb169a1573e6,
            0x9922ad74e68bc1cd,
            0x8c0dc4c4391a51ca,
            0xec5c8e0b12875d08,
            0x1dde2529566d9b5e,
            0x30c96e827699534,
        ];
        const R: &[u64] = &[
            0x43e1f593f0000001,
            0x2833e84879b97091,
            0xb85045b68181585d,
            0x30644e72e131a029,
        ];
        for k in 0..3 {
            let (p, q) = bn254_points(k);
            let f = Bn254::multi_miller_loop([p, -p], [q, q]).0;
            let omega = bn254::root_27th_of_unity();
            let mut y = f;
            let mut w = crate::bn254::Fq12::one();
            for _ in 0..3 {
                if y.pow(bn254::EXP1).is_one() {
                    break;
                }
                y *= &omega;
                w *= &omega;
            }
            assert!(y.pow(bn254::EXP1).is_one(), "f w is a cubic residue");
            let z = y.pow(bn254::R_INV);
            assert_eq!(z.pow(R), y, "r-th root");
            let z2 = z.pow(bn254::M_INV);
            assert_eq!(z2.pow(M), z, "m-th root");
            let (c, d, s) = bn254::witness(&f).expect("an identity has a witness");
            assert!((c * d).is_one());
            assert_eq!(
                crate::bn254::Fq12::new(s, crate::bn254::Fq6::zero()),
                w,
                "same scaling"
            );
            assert_eq!(c.square() * c, z2, "cube root");
            assert_eq!(c.pow(BN254_LAMBDA), y, "lambda {k}");
        }
    }
}
