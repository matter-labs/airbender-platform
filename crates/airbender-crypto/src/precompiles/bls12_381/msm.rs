// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/mod.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/mod.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/errors.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/single_pair.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/multiple_pairs.rs
//! BLS12-381 G1 and G2 multi-scalar multiplication (EIP-2537), from the EIP-2537 encoding of
//! the pairs.

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

/// Pippenger multi-scalar multiplication.
///
/// Invariant: `bases.len() == bigints.len()` (asserted at entry).
/// All indexing below is bounded by `size` which equals both lengths.
// zksync-os's code (`last_window == false`), whose crate allows the lint
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

/// The multi-scalar multiplication of G1 points (EIP-2537) over the pairs of `input`, each the
/// EIP-2537 encoding of a G1 point and a big-endian 256-bit scalar (not reduced): the sum of the
/// points times their scalars. Fails unless every point is canonical, on the curve, and in the
/// subgroup, checked pair by pair while the pairs are parsed (so a later pair fails after the
/// hints of the earlier ones). `input` must not be empty (with an oracle, an empty input
/// panics). The wrapper keeps the gas, the size of the input (a non-zero multiple of 160), and
/// the encoding of the result (zksync-os `write_g1`).
///
/// With an oracle, the subgroup checks take checked hinted divisions, and so does the
/// multiplication: of one or two pairs, a joint affine double-and-add; of more, the buckets in
/// projective coordinates, then a hinted inverse for the affine result. A wrong hint panics.
/// The points and the scalars go into vectors of `allocator`, and so do the buckets.
pub fn bls12_381_g1_msm<O: HintOracle, A: core::alloc::Allocator + Clone>(
    input: &[[u8; G1_MSM_PAIR_LEN]],
    allocator: A,
    mut oracle: Option<&mut O>,
) -> Result<G1Affine, Bls12Error> {
    let num_pairs = input.len();
    let mut scalars = Vec::with_capacity_in(num_pairs, allocator.clone());
    let mut points = Vec::with_capacity_in(num_pairs, allocator.clone());

    // arkworks MSM allocates inside, so we will do it our way, just parse here
    // G1Projective::msm_bigint(bases, bigints)

    // parse to use Peppinger algorithm
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

/// The multi-scalar multiplication of G2 points (EIP-2537) over the pairs of `input`, each the
/// EIP-2537 encoding of a G2 point and a big-endian 256-bit scalar (not reduced), as
/// [`bls12_381_g1_msm`], except that an empty input is not a panic here: it gives the point at
/// infinity, with an oracle or without one, and asks no hint (the wrapper rejects it first). The
/// wrapper keeps the gas, the size of the input (a non-zero multiple of 288), and the encoding of
/// the result (zksync-os `write_g2`).
///
/// With an oracle, the subgroup checks take checked hinted divisions, and the projective result
/// of the buckets a hinted inverse. A wrong hint panics. The points and the scalars go into
/// vectors of `allocator`, and so do the buckets.
pub fn bls12_381_g2_msm<O: HintOracle, A: core::alloc::Allocator + Clone>(
    input: &[[u8; G2_MSM_PAIR_LEN]],
    allocator: A,
    mut oracle: Option<&mut O>,
) -> Result<G2Affine, Bls12Error> {
    let num_pairs = input.len();

    let mut scalars = Vec::with_capacity_in(num_pairs, allocator.clone());
    let mut points = Vec::with_capacity_in(num_pairs, allocator.clone());

    // arkworks MSM allocates inside, so we will do it our way, just parse here
    // G1Projective::msm_bigint(bases, bigints)

    // parse to use Peppinger algorithm
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
    // from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/mod.rs

    use super::super::tests as bls_tests;
    use super::*;
    use crate::ark_ec::scalar_mul::sw_double_and_add_affine;
    use crate::ark_ff::{AdditiveGroup, Zero};
    use crate::hints::NoHints;
    use num_bigint::BigUint;
    use std::alloc::Global;
    use std::vec;
    use std::vec::Vec;

    type Scalar = <Fr as PrimeField>::BigInt;

    /// The absolute value of the curve parameter
    const X: u64 = 0xd201000000010000;

    /// Not a point of the curve: `(1, 1)`, and `1 != 1 + 4`
    const NOT_ON_CURVE: [u8; G1_SERIALIZATION_LEN] = {
        let mut bytes = [0u8; G1_SERIALIZATION_LEN];
        bytes[FIELD_ELEMENT_SERIALIZATION_LEN - 1] = 1;
        bytes[G1_SERIALIZATION_LEN - 1] = 1;
        bytes
    };

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

        fn fq(&mut self) -> Fq {
            let bytes: Vec<u8> = (0..8).flat_map(|_| self.next_u64().to_le_bytes()).collect();
            Fq::from_le_bytes_mod_order(&bytes)
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

    /// How many random scalars of each kind `test_scalars` takes: zksync-os's numbers, which it
    /// runs in release, or a few in a debug build, where a call with an oracle takes a second
    /// (each answer of the recording oracle takes milliseconds on the emulated delegated fields)
    const RANDOM_SCALARS: [usize; 3] = if cfg!(debug_assertions) {
        [2, 1, 1]
    } else {
        [24, 8, 8]
    };

    /// Scalars around the multiples of the group order `r`, `(r ± 1) / 2`, `X`, the extremes of
    /// 256 bits, and random ones: full-width (about half of them at least `r`), 50-bit, and
    /// below `2^254 < r`
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

    fn encode_point(p: G1Affine) -> [u8; G1_SERIALIZATION_LEN] {
        let mut bytes = [0u8; G1_SERIALIZATION_LEN];
        crate::bls12_381::eip2537::serialize_g1_bytes(p, &mut bytes);
        bytes
    }

    fn encode_pair(point: &[u8; G1_SERIALIZATION_LEN], s: &Scalar) -> Vec<u8> {
        let mut input = point.to_vec();
        for limb in s.0.iter().rev() {
            input.extend_from_slice(&limb.to_be_bytes());
        }
        input
    }

    /// zksync-os's `execute`: the endpoint without the oracle (which cannot query) and with the
    /// hints of an honest oracle give the same result, returned encoded (the encoding is the
    /// wrapper's, zksync-os `write_g1`)
    fn execute(input: &[u8]) -> Result<[u8; G1_SERIALIZATION_LEN], Bls12Error> {
        let (input, []) = input.as_chunks::<G1_MSM_PAIR_LEN>() else {
            panic!("whole pairs")
        };
        let hinted = bls12_381_g1_msm(input, Global, Some(&mut bls_tests::honest_oracle()));
        let result = bls12_381_g1_msm::<NoHints, _>(input, Global, None);
        assert_eq!(result, hinted);
        result.map(bls_tests::write_g1)
    }

    /// The pair is rejected with `expected`
    fn assert_rejected(point: &[u8; G1_SERIALIZATION_LEN], s: &Scalar, expected: Bls12Error) {
        assert_eq!(execute(&encode_pair(point, s)), Err(expected));
    }

    // from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/errors.rs
    mod errors {
        use super::*;

        #[test]
        fn single_pair_error_paths_unchanged() {
            use Bls12Error::{InvalidG1Point, NotInSubgroup};
            let mut rng = TestRng(3);
            let s = scalar(&rng.below_power_of_two(256));

            assert_rejected(&NOT_ON_CURVE, &s, InvalidG1Point);

            // a coordinate that is not canonical (x + q for the field modulus q), and one with a
            // non-zero byte in its padding
            let generator = encode_point(G1Affine::generator());
            let x = BigUint::from_bytes_be(&generator[16..FIELD_ELEMENT_SERIALIZATION_LEN])
                + BigUint::from(<Fq as PrimeField>::MODULUS);
            let mut not_canonical = generator;
            not_canonical[16..FIELD_ELEMENT_SERIALIZATION_LEN].copy_from_slice(&x.to_bytes_be());
            assert_rejected(&not_canonical, &s, InvalidG1Point);
            let mut padded = generator;
            padded[FIELD_ELEMENT_SERIALIZATION_LEN] = 1;
            assert_rejected(&padded, &s, InvalidG1Point);

            // points of the curve outside of G1, `[r]P != O` by a double-and-add on the integer `r`
            // (`mul_bigint` would reduce it to 0): the two points of order 3, random points (not
            // cofactor-cleared), and the generator plus a point of order 3
            let order_three = G1Affine::new_unchecked(Fq::ZERO, Fq::from(2u64));
            let mut points = vec![order_three, -order_three];
            while points.len() < 10 {
                let x = rng.fq();
                if let Some(p) = G1Affine::get_point_from_x_unchecked(x, rng.next_u64() & 1 == 1) {
                    points.push(p);
                }
            }
            points.push((G1Affine::generator().into_group() + order_three).into_affine());
            let r = scalar(&group_order());
            for p in points {
                assert!(p.is_on_curve());
                // a random point of the curve is in G1 with probability 1 / cofactor, about 2^-126
                assert!(!sw_double_and_add_affine(&p, r).is_zero(), "{p:?} is in G1");
                assert_rejected(&encode_point(p), &s, NotInSubgroup);
            }
        }
    }

    // from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/multiple_pairs.rs
    mod multiple_pairs {
        use super::*;

        #[test]
        fn affine_two_pair_and_larger_bucket_msm_match_integer_multiples() {
            let g = G1Affine::generator();
            let scalars = [
                Scalar::new([u64::MAX; 4]),
                scalar(&(group_order() + 1u32)),
                scalar(&BigUint::from(0u32)),
            ];
            for n in [2, 3] {
                let points = [g, -g, g];
                let mut input = Vec::new();
                let mut expected = G1Projective::zero();
                for i in 0..n {
                    input.extend(encode_pair(&encode_point(points[i]), &scalars[i]));
                    expected += sw_double_and_add_affine(&points[i], scalars[i].as_ref());
                }
                let output = execute(&input).unwrap();
                assert_eq!(parse_g1(&output).unwrap(), expected.into_affine());
            }
        }

        #[test]
        fn g2_msm_normalization_and_subgroup_hints_preserve_results() {
            let g = G2Affine::generator();
            for points in [vec![g], vec![g, -g], vec![g, g, G2Affine::identity()]] {
                let mut input = Vec::new();
                for point in points {
                    input.extend(bls_tests::write_g2(point));
                    input.extend([0xffu8; SCALAR_SERIALIZATION_LEN]);
                }
                let (input, []) = input.as_chunks::<G2_MSM_PAIR_LEN>() else {
                    panic!("whole pairs")
                };
                let _ = bls_tests::compare(
                    bls12_381_g2_msm::<NoHints, _>(input, Global, None),
                    bls12_381_g2_msm(input, Global, Some(&mut bls_tests::honest_oracle())),
                );
            }
        }
    }

    // from zksync-os e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/single_pair.rs
    mod single_pair {
        use super::*;

        #[test]
        fn single_pair_msm_matches_reference() {
            let mut rng = TestRng(1);
            let scalars = test_scalars(&mut rng);
            for p in test_points(&mut rng, 6) {
                for s in &scalars {
                    let generic = msm::<G1Projective, _>(&[p], vec![*s], Global).into_affine();
                    let reference = sw_double_and_add_affine(&p, s).into_affine();
                    assert_eq!(generic, reference, "msm at {p:?} * {s}");
                    assert_eq!(encode_point(generic), encode_point(reference));
                }
            }
        }

        #[test]
        fn single_pair_precompile_output_matches() {
            let mut rng = TestRng(2);
            let scalars = test_scalars(&mut rng);
            // one random point in a debug build (see `RANDOM_SCALARS`)
            let points = test_points(&mut rng, if cfg!(debug_assertions) { 1 } else { 2 });
            let t = scalar(&rng.below_power_of_two(256));
            let infinity_pair = encode_pair(&[0u8; G1_SERIALIZATION_LEN], &t);
            for p in points {
                for s in &scalars {
                    let input = encode_pair(&encode_point(p), s);
                    let output = execute(&input).unwrap();
                    let expected = encode_point(sw_double_and_add_affine(&p, s).into_affine());
                    assert_eq!(output, expected, "{p:?} * {s}");

                    // the same product through `msm`: a second pair that adds the point at infinity
                    let mut two_pairs = input;
                    two_pairs.extend_from_slice(&infinity_pair);
                    let generic = execute(&two_pairs).unwrap();
                    assert_eq!(output, generic, "{p:?} * {s}");
                }
            }
        }
    }

    // new tests

    mod advice_matches_no_advice {
        use super::*;
        use crate::precompiles::{HintedCases, HINT_HEAVY_CASES};
        use proptest::test_runner::Config as ProptestConfig;
        use proptest::{prop_assert_eq, proptest};

        /// The point of `mode` in place of `k` times the generator: the point at infinity, a
        /// point of the curve off the subgroup (rejected), one that is mostly not a point of the
        /// curve, or the multiple itself
        fn g1_pair(k: u64, scalar: &[u8; 32], mode: u8, bytes: &[u8; 48]) -> Vec<u8> {
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

        fn g2_pair(k: u64, scalar: &[u8; 32], mode: u8, bytes: &[u8; 96]) -> Vec<u8> {
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

        /// One to five pairs, through the affine chains (up to two pairs, with an oracle) and
        /// the buckets (from four, either way), and one of them maybe at infinity or rejected
        #[test]
        fn g1_msm() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(HINT_HEAVY_CASES), |(ks: [u64; 5], scalars: [[u8; 32]; 5], len in 1usize..=5, mode in 0u8..12, at: usize, bytes: [u8; 48])| {
                let at = at % len;
                let input: Vec<u8> = (0..len)
                    .flat_map(|i| g1_pair(ks[i], &scalars[i], if i == at { mode } else { u8::MAX }, &bytes))
                    .collect();
                let (input, []) = input.as_chunks::<G1_MSM_PAIR_LEN>() else {
                    unreachable!("whole pairs")
                };
                let hinted = hinted_cases.record(|oracle| bls12_381_g1_msm(input, Global, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_g1_msm(input, Global, None::<&mut NoHints>));
            });
            hinted_cases.check();
        }

        #[test]
        fn g2_msm() {
            let hinted_cases = HintedCases::default();
            proptest!(ProptestConfig::with_cases(HINT_HEAVY_CASES), |(ks: [u64; 4], scalars: [[u8; 32]; 4], len in 1usize..=4, mode in 0u8..12, at: usize, bytes: [[u8; 48]; 2])| {
                let at = at % len;
                let bytes: [u8; 96] = bytes.concat().try_into().unwrap();
                let input: Vec<u8> = (0..len)
                    .flat_map(|i| g2_pair(ks[i], &scalars[i], if i == at { mode } else { u8::MAX }, &bytes))
                    .collect();
                let (input, []) = input.as_chunks::<G2_MSM_PAIR_LEN>() else {
                    unreachable!("whole pairs")
                };
                let hinted = hinted_cases.record(|oracle| bls12_381_g2_msm(input, Global, Some(oracle)));
                prop_assert_eq!(hinted, bls12_381_g2_msm(input, Global, None::<&mut NoHints>));
            });
            hinted_cases.check();
        }
    }

    /// The guest, which reads its answers off the recorded tape, gets the same result and takes
    /// the whole tape and no more. The bls12-381 elements of test builds have the representation
    /// of the guest, so the tape is also the answers this process reads.
    #[test]
    fn recorded_tape_replays() {
        let mut rng = TestRng(5);
        let pairs: Vec<u8> = test_points(&mut rng, 4)[1..]
            .iter()
            .flat_map(|p| encode_pair(&encode_point(*p), &scalar(&rng.below_power_of_two(256))))
            .collect();
        let pairs = pairs.as_chunks::<G1_MSM_PAIR_LEN>().0;
        // the affine chains of one and two pairs, and the buckets of four
        for n in [1, 2, 4] {
            let input = &pairs[..n];
            let result =
                assert_tape_replays!(|oracle| bls12_381_g1_msm(input, Global, Some(oracle)));
            assert_eq!(
                result,
                bls12_381_g1_msm(input, Global, None::<&mut NoHints>)
            );
        }
        let g2 = G2Affine::generator();
        let input: Vec<u8> = [g2, (g2 + g2).into_affine()]
            .into_iter()
            .flat_map(|point| [bls_tests::write_g2(point).as_slice(), &[0x5a; 32]].concat())
            .collect();
        let input = input.as_chunks::<G2_MSM_PAIR_LEN>().0;
        let result = assert_tape_replays!(|oracle| bls12_381_g2_msm(input, Global, Some(oracle)));
        assert_eq!(
            result,
            bls12_381_g2_msm(input, Global, None::<&mut NoHints>)
        );
    }

    /// The subgroup checks run pair by pair while the pairs are parsed, so a pair off the
    /// subgroup is rejected after the hints of the pairs before it
    #[test]
    fn subgroup_hints_are_asked_while_parsing() {
        let s = scalar(&BigUint::from(7u32));
        let valid = encode_point(G1Affine::generator());
        let off_subgroup = encode_point(bls_tests::g1_off_subgroup(&[7; 48]));
        let input = [encode_pair(&valid, &s), encode_pair(&off_subgroup, &s)].concat();
        let (result, tape) = crate::hints::record_hints(|oracle| {
            bls12_381_g1_msm(input.as_chunks().0, Global, Some(oracle))
        });
        assert_eq!(result, Err(Bls12Error::NotInSubgroup));
        let (_, first) = crate::hints::record_hints(|oracle| {
            parse_g1_with_subgroup_check_with_oracle(&valid, Some(oracle))
        });
        let (_, second) = crate::hints::record_hints(|oracle| {
            parse_g1_with_subgroup_check_with_oracle(&off_subgroup, Some(oracle))
        });
        assert!(!first.is_empty() && !second.is_empty());
        assert_eq!(tape, [first, second].concat());
    }
}
