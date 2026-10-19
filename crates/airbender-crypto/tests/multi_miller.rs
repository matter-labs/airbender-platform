//! The multi-pair Miller loops through the public API in the forward field representation
//! (arkworks, default features), where every element is reduced: equal values are equal limbs,
//! so these tests check bit identity with the Miller loop of each pair on its own (one
//! single-pair call per pair, multiplied), and agreement with arkworks.
// the forward representation only (the delegated one has other limb counts and conversions)
#![cfg(not(any(
    feature = "proving",
    all(target_arch = "riscv32", feature = "bigint_ops")
)))]

use airbender_crypto::ark_ec::pairing::{MillerLoopOutput, Pairing};
use airbender_crypto::ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use airbender_crypto::ark_ff::{AdditiveGroup, Field, One, PrimeField, UniformRand};
use ark_std::rand::Rng;
use ark_std::test_rng;

/// Pairs of `n` points at most, and the positions of the pairs with a point at infinity: none,
/// one at the start, the middle or the end, all, every other one, a first chunk's worth
fn infinity_patterns(n: usize) -> Vec<Vec<bool>> {
    let mut candidates = vec![vec![false; n]];
    if n > 0 {
        for index in [0, n / 2, n - 1] {
            let mut zero = vec![false; n];
            zero[index] = true;
            candidates.push(zero);
        }
        candidates.push(vec![true; n]);
        candidates.push((0..n).map(|i| i % 2 == 1).collect());
        candidates.push((0..n).map(|i| i < 4).collect());
    }
    let mut patterns = vec![];
    for zero in candidates {
        if !patterns.contains(&zero) {
            patterns.push(zero);
        }
    }
    patterns
}

mod bls12_381 {
    use super::*;
    use airbender_crypto::ark_ec::bls12::Bls12Config;
    use airbender_crypto::bls12_381::curves::{Bls12_381, Config, G2PreparedNoAlloc};
    use airbender_crypto::bls12_381::{
        Fq, Fq12, Fq2, Fq6, Fr, G1Affine, G1Projective, G2Affine, G2Projective,
    };

    /// The Montgomery and the canonical limbs of the twelve coefficients
    fn limbs(f: &Fq12) -> Vec<[u64; 6]> {
        let fq6 = |a: &Fq6| [a.c0, a.c1, a.c2];
        [fq6(&f.c0), fq6(&f.c1)]
            .into_iter()
            .flatten()
            .flat_map(|a| [a.c0, a.c1])
            .flat_map(|a| [a.0 .0, a.into_bigint().0])
            .collect()
    }

    fn assert_identical(new: &Fq12, expected: &Fq12, what: &str) {
        assert_eq!(new, expected, "{what}");
        assert_eq!(limbs(new), limbs(expected), "{what}");
    }

    fn conjugated(f: &Fq12) -> Fq12 {
        let mut f = *f;
        f.conjugate_in_place();
        f
    }

    fn random_g1(rng: &mut impl Rng) -> G1Affine {
        (G1Projective::generator() * Fr::rand(rng)).into_affine()
    }

    fn random_g2(rng: &mut impl Rng) -> G2Affine {
        (G2Projective::generator() * Fr::rand(rng)).into_affine()
    }

    /// `n` pairs, `Q` a curve point or a prepared point with arbitrary coefficients (normalized
    /// or not), `P` a curve point or an arbitrary one, some at infinity
    fn pairs(zero: &[bool], rng: &mut impl Rng) -> (Vec<G1Affine>, Vec<G2PreparedNoAlloc>) {
        zero.iter()
            .map(|&zero| {
                let p = match rng.gen_range(0..4) {
                    0 => G1Affine::new_unchecked(Fq::rand(rng), Fq::rand(rng)),
                    _ => random_g1(rng),
                };
                let q = match rng.gen_range(0..4) {
                    0 => G2PreparedNoAlloc {
                        ell_coeffs: core::array::from_fn(|_| {
                            (Fq2::rand(rng), Fq2::rand(rng), Fq2::rand(rng))
                        }),
                        infinity: false,
                        normalized: rng.gen(),
                    },
                    _ => random_g2(rng).into(),
                };
                match (zero, rng.gen_range(0..3)) {
                    (false, _) => (p, q),
                    (true, 0) => (G1Affine::identity(), q),
                    (true, 1) => (p, G2Affine::identity().into()),
                    (true, _) => (G1Affine::identity(), G2Affine::identity().into()),
                }
            })
            .unzip()
    }

    /// The product of the single-pair Miller functions (not conjugated), `initial` given to the
    /// first pair without a point at infinity: the Miller loop of each pair on its own, as before
    /// the pairs shared an accumulator (a single pair is one chunk, whose loop is the old
    /// per-pair loop operation for operation, up to the exact `1 f` and the skipped `1^2`),
    /// multiplied in order starting from `1`
    fn per_pair(initial: Option<&Fq12>, g1: &[G1Affine], g2: &[G2PreparedNoAlloc]) -> Fq12 {
        let mut initial = initial;
        let mut f = Fq12::one();
        for (p, q) in g1.iter().zip(g2) {
            if p.is_zero() || q.is_zero() {
                continue;
            }
            f *= match initial.take() {
                Some(d) => Bls12_381::multi_miller_loop_with_initial(d, [*p], [q]),
                None => conjugated(&Bls12_381::multi_miller_loop_prepared([*p], [q])),
            };
        }
        match initial {
            Some(d) => d.pow(Config::X),
            None => f,
        }
    }

    #[test]
    fn entry_points_match_per_pair_loops() {
        let mut rng = test_rng();
        for n in 0..=9 {
            for zero in infinity_patterns(n) {
                let (g1, g2) = pairs(&zero, &mut rng);
                let a = || g1.iter().copied();
                let expected = conjugated(&per_pair(None, &g1, &g2));
                let f = Bls12_381::multi_miller_loop_prepared(a(), &g2);
                assert_identical(&f, &expected, "prepared");
                let f = Bls12_381::multi_miller_loop_prepared(a(), g2.iter().cloned());
                assert_identical(&f, &expected, "prepared, owned");
                let f = Bls12_381::multi_miller_loop(a(), g2.iter().cloned()).0;
                assert_identical(&f, &expected, "Pairing");
                for d in [Fq12::rand(&mut rng), Fq12::ZERO, Fq12::one()] {
                    let expected = per_pair(Some(&d), &g1, &g2);
                    let l = Bls12_381::multi_miller_loop_with_initial(&d, a(), &g2);
                    assert_identical(&l, &expected, "with initial");
                }
            }
        }
    }

    #[test]
    fn curve_points_match_per_pair_loops() {
        let mut rng = test_rng();
        for n in 0..=9 {
            for zero in infinity_patterns(n) {
                let mut g1: Vec<_> = (0..n).map(|_| random_g1(&mut rng)).collect();
                let mut g2: Vec<_> = (0..n).map(|_| random_g2(&mut rng)).collect();
                for (i, _) in zero.iter().enumerate().filter(|(_, zero)| **zero) {
                    match i % 2 {
                        0 => g1[i] = G1Affine::identity(),
                        _ => g2[i] = G2Affine::identity(),
                    }
                }
                let mut expected = Fq12::one();
                for (p, q) in g1.iter().zip(&g2) {
                    expected *= Bls12_381::multi_miller_loop([p], [q]).0;
                }
                let f = Bls12_381::multi_miller_loop(&g1, &g2).0;
                assert_identical(&f, &expected, "Pairing");
            }
        }
    }

    fn to_ark_fq2(a: &Fq2) -> ark_bls12_381::Fq2 {
        ark_bls12_381::Fq2::new(a.c0, a.c1)
    }

    fn to_ark_fq12(a: &Fq12) -> ark_bls12_381::Fq12 {
        let fq6 = |a: &Fq6| {
            ark_bls12_381::Fq6::new(to_ark_fq2(&a.c0), to_ark_fq2(&a.c1), to_ark_fq2(&a.c2))
        };
        ark_bls12_381::Fq12::new(fq6(&a.c0), fq6(&a.c1))
    }

    fn to_ark_g1(p: &G1Affine) -> ark_bls12_381::G1Affine {
        match p.infinity {
            true => ark_bls12_381::G1Affine::identity(),
            false => ark_bls12_381::G1Affine::new(p.x, p.y),
        }
    }

    fn to_ark_g2(q: &G2Affine) -> ark_bls12_381::G2Affine {
        match q.infinity {
            true => ark_bls12_381::G2Affine::identity(),
            false => ark_bls12_381::G2Affine::new(to_ark_fq2(&q.x), to_ark_fq2(&q.y)),
        }
    }

    /// Every entry point against arkworks; whether the pairing is one
    fn check_against_arkworks(g1: &[G1Affine], g2: &[G2Affine], rng: &mut impl Rng) -> bool {
        let ark_g1: Vec<_> = g1.iter().map(to_ark_g1).collect();
        let ark_g2: Vec<_> = g2.iter().map(to_ark_g2).collect();
        let expected = ark_bls12_381::Bls12_381::multi_miller_loop(&ark_g1, &ark_g2).0;
        let f = Bls12_381::multi_miller_loop(g1, g2).0;
        assert_eq!(to_ark_fq12(&f), expected);
        let prepared: Vec<_> = g2.iter().map(G2PreparedNoAlloc::from).collect();
        let borrowed = Bls12_381::multi_miller_loop_prepared(g1, &prepared);
        assert_eq!(to_ark_fq12(&borrowed), expected);
        let expected = ark_bls12_381::Bls12_381::multi_pairing(&ark_g1, &ark_g2).0;
        assert_eq!(to_ark_fq12(&Bls12_381::multi_pairing(g1, g2).0), expected);
        let gt = Bls12_381::final_exponentiation(MillerLoopOutput(borrowed))
            .unwrap()
            .0;
        assert_eq!(to_ark_fq12(&gt), expected);
        let d = Fq12::rand(rng);
        let l = Bls12_381::multi_miller_loop_with_initial(&d, g1, &prepared);
        assert_eq!(l, d.pow(Config::X) * conjugated(&borrowed));
        gt.is_one()
    }

    #[test]
    fn matches_arkworks() {
        let mut rng = test_rng();
        for n in 0..=9 {
            for zero in infinity_patterns(n) {
                let mut g1: Vec<_> = (0..n).map(|_| random_g1(&mut rng)).collect();
                let mut g2: Vec<_> = (0..n).map(|_| random_g2(&mut rng)).collect();
                for (i, _) in zero.iter().enumerate().filter(|(_, zero)| **zero) {
                    match i % 3 {
                        0 => g1[i] = G1Affine::identity(),
                        1 => g2[i] = G2Affine::identity(),
                        _ => (g1[i], g2[i]) = (G1Affine::identity(), G2Affine::identity()),
                    }
                }
                let is_one = check_against_arkworks(&g1, &g2, &mut rng);
                assert_eq!(is_one, zero.iter().all(|zero| *zero));
            }
        }
    }

    #[test]
    fn identity_products_match_arkworks() {
        let mut rng = test_rng();
        for m in 1..=4 {
            // `e(P, Q) e(-P, Q) = 1`, with a pair at infinity among them, and `e(sP, Q) = e(P, sQ)`
            let mut pairs = vec![(G1Affine::identity(), random_g2(&mut rng))];
            for _ in 0..m {
                let (p, q) = (random_g1(&mut rng), random_g2(&mut rng));
                pairs.push((p, q));
                pairs.insert(rng.gen_range(0..pairs.len()), (-p, q));
            }
            let (p, q, s) = (random_g1(&mut rng), random_g2(&mut rng), Fr::rand(&mut rng));
            pairs.push(((p * s).into_affine(), q));
            pairs.push((p, -(q * s).into_affine()));
            let (g1, g2): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
            assert!(check_against_arkworks(&g1, &g2, &mut rng));
        }
    }

    #[test]
    #[should_panic(expected = "Caller must check input lengths")]
    fn more_g1_points_panic() {
        let mut rng = test_rng();
        let q = G2PreparedNoAlloc::from(random_g2(&mut rng));
        let d = Fq12::rand(&mut rng);
        let _ = Bls12_381::multi_miller_loop_with_initial(&d, [random_g1(&mut rng); 6], [&q; 5]);
    }

    #[test]
    #[should_panic(expected = "Caller must check input lengths")]
    fn more_g2_points_panic() {
        let mut rng = test_rng();
        let _ = Bls12_381::multi_miller_loop([random_g1(&mut rng); 5], [random_g2(&mut rng); 6]);
    }
}
