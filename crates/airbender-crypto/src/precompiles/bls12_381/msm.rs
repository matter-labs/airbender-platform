//! BLS12-381 G1 and G2 multi-scalar multiplication (EIP-2537).

use super::*;
use crate::ark_ec::CurveGroup;
use crate::ark_ff::AdditiveGroup;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints as curve_hints;
use alloc::vec::Vec;

#[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
use super::super::unavailable as curve_hints;

/// The size of the EIP-2537 encoding of a pair of the G1 MSM: a G1 point and a scalar
pub const G1_MSM_PAIR_LEN: usize = SCALAR_SERIALIZATION_LEN + G1_SERIALIZATION_LEN;
/// The size of the EIP-2537 encoding of a pair of the G2 MSM: a G2 point and a scalar
pub const G2_MSM_PAIR_LEN: usize = SCALAR_SERIALIZATION_LEN + G2_SERIALIZATION_LEN;

fn log2(x: usize) -> u32 {
    if x == 0 {
        0
    } else if x.is_power_of_two() {
        1usize.leading_zeros() - x.leading_zeros()
    } else {
        0usize.leading_zeros() - x.leading_zeros()
    }
}

fn ln_without_floats(a: usize) -> usize {
    // log2(a) * ln(2)
    (log2(a) * 69 / 100) as usize
}

/// Pippenger multi-scalar multiplication; `bases` and `bigints` have the same length.
#[allow(clippy::bool_comparison)]
fn msm<G: CurveGroup, A: core::alloc::Allocator + Clone>(
    bases: &[G::Affine],
    mut bigints: Vec<<Fr as PrimeField>::BigInt, A>,
    allocator: A,
) -> G {
    assert_eq!(bases.len(), bigints.len());
    let size = bases.len();

    const NUM_BITS: usize = 256;

    // Special-case for small inputs: double-and-add over all bits.
    if size < 4 {
        let mut acc = G::ZERO;
        for bit in (0..NUM_BITS).rev() {
            let word_idx = bit / 64;
            let bit_idx = bit % 64;

            for (base, bigint) in bases.iter().zip(bigints.iter()) {
                if bigint.0[word_idx] & 1 << bit_idx > 0 {
                    acc += base;
                }
            }
            if bit > 0 {
                acc.double_in_place();
            }
        }

        return acc;
    }

    let c = if size < 32 {
        3
    } else {
        ln_without_floats(size) + 2
    };
    assert!(c < 64);

    let zero = G::zero();
    let mut window_start = 0;
    let num_windows = NUM_BITS.div_ceil(c);

    let mut reusable_buckets = Vec::with_capacity_in((1 << c) - 1, allocator.clone());
    reusable_buckets.resize((1 << c) - 1, zero);

    let mut window_sums = Vec::with_capacity_in(num_windows, allocator);
    window_sums.resize(num_windows, zero);

    let lowest_bits_mask = (1u64 << c) - 1;

    #[allow(clippy::needless_range_loop)]
    for window_idx in 0..num_windows {
        let last_window = window_idx == num_windows - 1;

        for (base, bigint) in bases.iter().zip(bigints.iter_mut()) {
            // Extract lowest c bits as the bucket index.
            let scalar: u64 = bigint.as_ref()[0] & lowest_bits_mask;

            use core::ops::ShrAssign;
            bigint.shr_assign(c as u32);

            if scalar != 0 {
                reusable_buckets[(scalar - 1) as usize] += base;
            }
        }

        // now sum over buckets
        let mut tmp = zero;
        let mut window_result = zero;
        for el in reusable_buckets.iter_mut().rev() {
            tmp += &*el;
            window_result += &tmp;
            if last_window == false {
                *el = zero;
            }
        }
        window_sums[window_idx] = window_result;

        window_start += c;
    }

    assert!(window_start >= 256);

    let lowest = *window_sums.first().unwrap();

    lowest
        + window_sums[1..]
            .iter()
            .rev()
            .fold(zero, |mut total, sum_i| {
                total += sum_i;
                for _ in 0..c {
                    total.double_in_place();
                }
                total
            })
}

/// The G1 MSM of `input`: each pair is a G1 point and a big-endian 256-bit scalar (not reduced).
/// Fails unless every point is canonical, on the curve and in the subgroup, checked pair by pair
/// while parsing. `input` must not be empty (with an oracle, an empty input panics).
///
/// With an oracle, the subgroup checks take checked hinted divisions; one or two pairs use a
/// joint affine double-and-add with hinted divisions, more the projective `msm` and a hinted
/// inverse. A wrong hint panics. Points, scalars and buckets go into vectors of `allocator`.
pub fn bls12_381_g1_msm<O: MemoryOracle, A: core::alloc::Allocator + Clone>(
    input: &[[u8; G1_MSM_PAIR_LEN]],
    allocator: A,
    mut oracle: Option<&mut O>,
) -> Result<G1Affine, Bls12Error> {
    let num_pairs = input.len();
    let mut scalars = Vec::with_capacity_in(num_pairs, allocator.clone());
    let mut points = Vec::with_capacity_in(num_pairs, allocator.clone());

    // arkworks' MSM allocates internally, so parse here and run `msm` with `allocator`
    for pair_encoding in input.iter() {
        let point = parse_g1_with_subgroup_check_with_oracle(
            pair_encoding[0..G1_SERIALIZATION_LEN].try_into().unwrap(),
            oracle.as_deref_mut(),
        )?;
        let scalar = parse_integer(
            pair_encoding[G1_SERIALIZATION_LEN..(G1_SERIALIZATION_LEN + SCALAR_SERIALIZATION_LEN)]
                .try_into()
                .unwrap(),
        );
        points.push(point);
        scalars.push(scalar);
    }

    // Reduction modulo r is valid only after every point passed its subgroup check.
    // For one or two pairs affine GLV shares the doublings and checks each slope hint.
    let result = if let Some(oracle) = oracle.as_deref_mut().filter(|_| num_pairs <= 2) {
        let scalar = |s: <Fr as PrimeField>::BigInt| g1::scalar_from_limbs(s.as_ref());
        let k = scalar(scalars[0]);
        let (q, l) = if num_pairs == 2 {
            (points[1], scalar(scalars[1]))
        } else {
            (G1Affine::identity(), Fr::ZERO)
        };
        drop(scalars);
        g1::mul_two_affine_with_divider(
            &points[0],
            k,
            &q,
            l,
            &mut curve_hints::Bls12381FqDivider::new(oracle),
        )
    } else {
        let result: G1Projective = msm(&points, scalars, allocator);
        match oracle {
            Some(oracle) => g1::into_affine_with_inverse(&result, |z| {
                curve_hints::bls12_381_fq_inverse(oracle, z)
            }),
            None => result.into_affine(),
        }
    };

    Ok(result)
}

/// The G2 MSM of `input`, as [`bls12_381_g1_msm`], except that an empty input gives the point at
/// infinity without hints, and with an oracle any number of pairs takes the projective `msm` and
/// a hinted inverse.
pub fn bls12_381_g2_msm<O: MemoryOracle, A: core::alloc::Allocator + Clone>(
    input: &[[u8; G2_MSM_PAIR_LEN]],
    allocator: A,
    mut oracle: Option<&mut O>,
) -> Result<G2Affine, Bls12Error> {
    let num_pairs = input.len();

    let mut scalars = Vec::with_capacity_in(num_pairs, allocator.clone());
    let mut points = Vec::with_capacity_in(num_pairs, allocator.clone());

    for pair_encoding in input.iter() {
        let point = parse_g2_with_subgroup_check_with_oracle(
            pair_encoding[0..G2_SERIALIZATION_LEN].try_into().unwrap(),
            oracle.as_deref_mut(),
        )?;
        let scalar = parse_integer(
            pair_encoding[G2_SERIALIZATION_LEN..(G2_SERIALIZATION_LEN + SCALAR_SERIALIZATION_LEN)]
                .try_into()
                .unwrap(),
        );
        points.push(point);
        scalars.push(scalar);
    }

    let result: G2Projective = msm(&points, scalars, allocator);

    let result = match oracle {
        Some(oracle) => {
            g2::into_affine_with_inverse(&result, |z| curve_hints::bls12_381_fq2_inverse(oracle, z))
        }
        None => result.into_affine(),
    };

    Ok(result)
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::super::tests as bls_tests;
    use super::*;
    use crate::ark_ec::scalar_mul::sw_double_and_add_affine;
    use crate::hints::{record_hints, NoHints};
    use num_bigint::BigUint;
    use std::alloc::Global;
    use std::vec;
    use std::vec::Vec;

    type Scalar = <Fr as PrimeField>::BigInt;

    /// The absolute value of the curve parameter
    const X: u64 = 0xd201000000010000;

    /// splitmix64, for deterministic test values
    struct TestRng(u64);

    impl TestRng {
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }

        /// A random integer below `2^bits`, for `bits <= 256`
        fn below_power_of_two(&mut self, bits: u32) -> BigUint {
            let bytes: Vec<u8> = (0..4).flat_map(|_| self.next_u64().to_le_bytes()).collect();
            BigUint::from_bytes_le(&bytes) % (BigUint::from(1u32) << bits)
        }
    }

    fn group_order() -> BigUint {
        <Fr as PrimeField>::MODULUS.into()
    }

    fn scalar(n: &BigUint) -> Scalar {
        assert!(n.bits() <= 256);
        let mut limbs = [0u64; 4];
        for (limb, digit) in limbs.iter_mut().zip(n.iter_u64_digits()) {
            *limb = digit;
        }
        crate::BigInt::new(limbs)
    }

    /// How many random scalars of each kind `test_scalars` takes; few in a debug build, where a
    /// call with an oracle takes about a second
    const RANDOM_SCALARS: [usize; 3] = if cfg!(debug_assertions) {
        [2, 1, 1]
    } else {
        [24, 8, 8]
    };

    /// Scalars around the multiples of the group order `r`, `(r ± 1) / 2`, `X`, the extremes of
    /// 256 bits, and random ones: full-width, 50-bit, and below `2^254 < r`
    fn test_scalars(rng: &mut TestRng) -> Vec<Scalar> {
        let r = group_order();
        let one = BigUint::from(1u32);
        let mut scalars: Vec<BigUint> = vec![
            BigUint::from(0u32),
            one.clone(),
            BigUint::from(2u32),
            BigUint::from(3u32),
            &r - 1u32,
            r.clone(),
            &r + 1u32,
            &r * 2u32,
            &r * 2u32 + 1u32,
            &one << 255u32,
            (&one << 256u32) - 1u32,
            BigUint::from(X),
            (&r - 1u32) / 2u32,
            (&r + 1u32) / 2u32,
        ];
        let [full, small, below] = RANDOM_SCALARS;
        scalars.extend((0..full).map(|_| rng.below_power_of_two(256)));
        scalars.extend((0..small).map(|_| rng.below_power_of_two(50)));
        scalars.extend((0..below).map(|_| rng.below_power_of_two(254)));
        scalars.iter().map(scalar).collect()
    }

    /// The point at infinity, the generator, and `count` random points of G1, multiplied out by
    /// a plain double-and-add (not by the GLV multiplication under test)
    fn test_points(rng: &mut TestRng, count: usize) -> Vec<G1Affine> {
        let mut points = vec![G1Affine::identity(), G1Affine::generator()];
        for _ in 0..count {
            let k = scalar(&rng.below_power_of_two(255));
            points.push(sw_double_and_add_affine(&G1Affine::generator(), k).into_affine());
        }
        points
    }

    fn encode_pair(point: &[u8; G1_SERIALIZATION_LEN], s: &Scalar) -> Vec<u8> {
        let mut input = point.to_vec();
        for limb in s.0.iter().rev() {
            input.extend_from_slice(&limb.to_be_bytes());
        }
        input
    }

    /// The encoded G1 MSM of `input`, checked to be the same with the hints of an honest oracle
    fn execute(input: &[u8]) -> Result<[u8; G1_SERIALIZATION_LEN], Bls12Error> {
        let (input, []) = input.as_chunks::<G1_MSM_PAIR_LEN>() else {
            panic!("whole pairs")
        };
        let (hinted, _) = record_hints(|oracle| bls12_381_g1_msm(input, Global, Some(oracle)));
        let result = bls12_381_g1_msm::<NoHints, _>(input, Global, None);
        assert_eq!(result, hinted);
        result.map(bls_tests::write_g1)
    }

    #[test]
    fn errors() {
        use Bls12Error::{InvalidG1Point, NotInSubgroup};
        let s = scalar(&BigUint::from(7u32));
        let generator = bls_tests::write_g1(G1Affine::generator());
        // (1, 1), and 1 != 1 + 4
        let mut not_on_curve = [0u8; G1_SERIALIZATION_LEN];
        not_on_curve[FIELD_ELEMENT_SERIALIZATION_LEN - 1] = 1;
        not_on_curve[G1_SERIALIZATION_LEN - 1] = 1;
        // x + q for the field modulus q
        let x = BigUint::from_bytes_be(&generator[16..FIELD_ELEMENT_SERIALIZATION_LEN])
            + BigUint::from(<Fq as PrimeField>::MODULUS);
        let mut not_canonical = generator;
        not_canonical[16..FIELD_ELEMENT_SERIALIZATION_LEN].copy_from_slice(&x.to_bytes_be());
        let order_three = bls_tests::write_g1(bls_tests::order_three());
        for (point, expected) in [
            (not_on_curve, InvalidG1Point),
            (not_canonical, InvalidG1Point),
            (order_three, NotInSubgroup),
        ] {
            assert_eq!(execute(&encode_pair(&point, &s)), Err(expected));
        }
        let mut g2_not_on_curve = bls_tests::write_g2(G2Affine::generator());
        g2_not_on_curve[G2_SERIALIZATION_LEN - 1] ^= 1;
        let input = [g2_not_on_curve.as_slice(), &[1; SCALAR_SERIALIZATION_LEN]].concat();
        let input = input.as_chunks::<G2_MSM_PAIR_LEN>().0;
        let (hinted, _) = record_hints(|oracle| bls12_381_g2_msm(input, Global, Some(oracle)));
        assert_eq!(hinted, Err(Bls12Error::InvalidG2Point));
        assert_eq!(
            bls12_381_g2_msm::<NoHints, _>(input, Global, None),
            Err(Bls12Error::InvalidG2Point)
        );
    }

    /// 33 pairs, past the 32 where the buckets take a wider window
    #[test]
    fn wide_window_matches_double_and_add() {
        let mut rng = TestRng(3);
        let mut input = Vec::new();
        let mut expected = G1Projective::zero();
        for p in test_points(&mut rng, 31) {
            let s = scalar(&rng.below_power_of_two(256));
            input.extend(encode_pair(&bls_tests::write_g1(p), &s));
            expected += sw_double_and_add_affine(&p, s);
        }
        assert_eq!(
            execute(&input),
            Ok(bls_tests::write_g1(expected.into_affine()))
        );
    }

    /// One pair (with an oracle, the affine GLV chain), and the same with a second pair at
    /// infinity, against a plain double-and-add
    #[test]
    fn single_pair_matches_double_and_add() {
        let mut rng = TestRng(2);
        let scalars = test_scalars(&mut rng);
        // one random point in a debug build (see `RANDOM_SCALARS`)
        let points = test_points(&mut rng, if cfg!(debug_assertions) { 1 } else { 2 });
        let t = scalar(&rng.below_power_of_two(256));
        let infinity_pair = encode_pair(&[0u8; G1_SERIALIZATION_LEN], &t);
        for p in points {
            for s in &scalars {
                let input = encode_pair(&bls_tests::write_g1(p), s);
                let expected = bls_tests::write_g1(sw_double_and_add_affine(&p, s).into_affine());
                assert_eq!(execute(&input), Ok(expected), "{p:?} * {s}");
                let two_pairs = [input, infinity_pair.clone()].concat();
                assert_eq!(execute(&two_pairs), Ok(expected), "{p:?} * {s}");
            }
        }
    }

    mod advice_matches_no_advice {
        use super::*;
        use crate::precompiles::{HintedCases, HINT_HEAVY_CASES};
        use proptest::test_runner::Config as ProptestConfig;
        use proptest::{prop_assert_eq, proptest};

        /// `k` times the generator, or by `mode` the point at infinity, a point off the subgroup,
        /// or an encoding that is mostly not a point of the curve
        fn g1_pair(k: u64, scalar: &[u8; 32], mode: u8, bytes: &[u8]) -> Vec<u8> {
            let mut point = (G1Affine::generator() * Fr::from(k)).into_affine();
            match mode {
                0 => point = G1Affine::identity(),
                1 => point = bls_tests::g1_off_subgroup(bytes),
                _ => {}
            }
            let mut encoded = bls_tests::write_g1(point);
            if mode == 2 {
                encoded[G1_SERIALIZATION_LEN - 1] ^= 1;
            }
            [encoded.as_slice(), scalar].concat()
        }

        fn g2_pair(k: u64, scalar: &[u8; 32], mode: u8, bytes: &[u8]) -> Vec<u8> {
            let mut point = (G2Affine::generator() * Fr::from(k)).into_affine();
            match mode {
                0 => point = G2Affine::identity(),
                1 => point = bls_tests::g2_off_subgroup(bytes),
                _ => {}
            }
            let mut encoded = bls_tests::write_g2(point);
            if mode == 2 {
                encoded[G2_SERIALIZATION_LEN - 1] ^= 1;
            }
            [encoded.as_slice(), scalar].concat()
        }

        /// One to five pairs, through the affine chains (up to two G1 pairs, with an oracle) and
        /// the buckets (from four), and one of them maybe at infinity or rejected
        #[test]
        fn msm() {
            let (g1_cases, g2_cases) = (HintedCases::default(), HintedCases::default());
            proptest!(ProptestConfig::with_cases(HINT_HEAVY_CASES), |(ks: [u64; 5], scalars: [[u8; 32]; 5], len in 1usize..=5, mode in 0u8..12, at: usize, bytes: [u8; 96])| {
                let at = at % len;
                let mode = |i: usize| if i == at { mode } else { u8::MAX };
                let input: Vec<u8> = (0..len)
                    .flat_map(|i| g1_pair(ks[i], &scalars[i], mode(i), &bytes[..48]))
                    .collect();
                let input = input.as_chunks::<G1_MSM_PAIR_LEN>().0;
                let hinted = g1_cases.record(|oracle| bls12_381_g1_msm(input, Global, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_g1_msm(input, Global, None::<&mut NoHints>));

                let input: Vec<u8> = (0..len)
                    .flat_map(|i| g2_pair(ks[i], &scalars[i], mode(i), &bytes))
                    .collect();
                let input = input.as_chunks::<G2_MSM_PAIR_LEN>().0;
                let hinted = g2_cases.record(|oracle| bls12_381_g2_msm(input, Global, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_g2_msm(input, Global, None::<&mut NoHints>));
            });
            g1_cases.check();
            g2_cases.check();
        }
    }

    #[test]
    fn recorded_tape_replays() {
        let g1_pairs: Vec<u8> = (1..=4u64)
            .flat_map(|k| {
                let point = (G1Affine::generator() * Fr::from(k)).into_affine();
                [bls_tests::write_g1(point).as_slice(), &[0x5a; 32]].concat()
            })
            .collect();
        // the affine chains of one and two pairs, and the buckets of four
        for n in [1, 2, 4] {
            let input = &g1_pairs.as_chunks::<G1_MSM_PAIR_LEN>().0[..n];
            let result =
                assert_tape_replays!(|oracle| bls12_381_g1_msm(input, Global, Some(oracle)));
            assert_eq!(
                result,
                bls12_381_g1_msm(input, Global, None::<&mut NoHints>)
            );
        }
        let g2 = G2Affine::generator();
        let input = [bls_tests::write_g2(g2).as_slice(), &[0x5a; 32]].concat();
        let input = input.as_chunks::<G2_MSM_PAIR_LEN>().0;
        let result = assert_tape_replays!(|oracle| bls12_381_g2_msm(input, Global, Some(oracle)));
        assert_eq!(
            result,
            bls12_381_g2_msm(input, Global, None::<&mut NoHints>)
        );
    }
}
