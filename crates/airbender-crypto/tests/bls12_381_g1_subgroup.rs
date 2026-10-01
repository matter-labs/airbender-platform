//! The BLS12-381 G1 subgroup check (`is_in_correct_subgroup_assuming_on_curve`) through the
//! public API in the forward field representation (arkworks, default features), against its
//! previous implementation (kept here, on the public API: the same code path), upstream arkworks
//! and the ground truth `[r]P == O`; and its callers: `Valid::check`, `deserialize_compressed`
//! and `deserialize_uncompressed` (validating), `g1_from_compressed_with_sqrt`, the EIP-2537
//! parser with the subgroup check as zksync-os runs it. The delegated representation, and the
//! intermediate multiples: the unit tests at the bottom of `src/bls12_381/curves/g1.rs`.
// the forward representation only
#![cfg(not(any(
    feature = "proving",
    all(target_arch = "riscv32", feature = "bigint_ops")
)))]

use airbender_crypto::ark_ec::bls12::Bls12Config;
use airbender_crypto::ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use airbender_crypto::ark_ff::{AdditiveGroup, Field, PrimeField, UniformRand, Zero};
use airbender_crypto::ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Valid};
use airbender_crypto::bls12_381::curves::Config;
use airbender_crypto::bls12_381::eip2537::parse_g1_bytes;
use airbender_crypto::bls12_381::g1::endomorphism;
use airbender_crypto::bls12_381::{g1_from_compressed_with_sqrt, Fq, G1Affine};
use ark_std::rand::{rngs::StdRng, Rng, SeedableRng};
use ark_std::test_rng;
use core::ops::Neg;
use num_bigint::BigUint;
use proptest::prelude::{any, ProptestConfig};
use proptest::{prop_assert_eq, proptest};

type RefFq = ark_bls12_381::Fq;
type RefFr = ark_bls12_381::Fr;
type RefAffine = ark_bls12_381::G1Affine;
type RefProjective = ark_bls12_381::G1Projective;

/// The primes of `m = (X + 1) / 3`: the cofactor is `h = 3 m²`, and the points of order dividing
/// it form `Z/m × Z/3m` (there are no points of order `ℓ²`)
const PRIMES_OF_M: [u64; 4] = [11, 10177, 859267, 52437899];

/// The body of `is_in_correct_subgroup_assuming_on_curve` before the double-and-adds (GLV
/// multiplications, an inversion, the early-out), on the public API
fn subgroup_check_before(p: &G1Affine) -> bool {
    // Algorithm from Section 6 of https://eprint.iacr.org/2021/1130.
    //
    // Check that endomorphism_p(P) == -[X^2]P

    // An early-out optimization described in Section 6.
    // If uP == P but P != point of infinity, then the point is not in the right
    // subgroup.
    let x_times_p = p.mul_bigint(Config::X);
    if x_times_p.eq(p) && !p.infinity {
        return false;
    }

    let minus_x_squared_times_p = x_times_p.mul_bigint(Config::X).neg();
    let endomorphism_p = endomorphism(p);
    minus_x_squared_times_p.eq(&endomorphism_p)
}

fn x_abs() -> BigUint {
    BigUint::from(Config::X[0])
}

fn r() -> BigUint {
    RefFr::MODULUS.into()
}

/// The cofactor `h = (X + 1)² / 3`: `#E(Fq) = h r`
fn h() -> BigUint {
    (x_abs() + 1u32).pow(2) / 3u32
}

/// `[k]P` by a plain double-and-add on arkworks' projective group law, which handles every case:
/// exact for any point of the curve (`k` is not reduced modulo `r`, no GLV)
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

// field elements through their integers: independent of the representation
fn fq(x: RefFq) -> Fq {
    Fq::from(BigUint::from(x))
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

/// Points of order dividing the cofactor, none O: two `[r]R` (order dividing `X + 1`), both
/// points of order 3 (`(0, ±2)`) and, for every prime `ℓ | m`, three of order `ℓ` (generically
/// independent) and the sum of the first two (unless O)
fn cofactor_points(rng: &mut impl Rng) -> Vec<RefAffine> {
    let (r, h) = (r(), h());
    let two = RefFq::from(2u32);
    let mut points = vec![
        RefAffine::new_unchecked(RefFq::ZERO, two),
        RefAffine::new_unchecked(RefFq::ZERO, -two),
    ];
    for _ in 0..2 {
        let t = mul_exact(&random_point(rng), &r).into_affine();
        assert!(!t.is_zero() && mul_exact(&t, &(x_abs() + 1u32)).is_zero());
        points.push(t);
    }
    let t = torsion_point(rng, &(&r * &h / 3u32), 3);
    assert!(points[..2].contains(&t));
    for l in PRIMES_OF_M {
        let multiple = &r * &h / BigUint::from(l * l);
        let (a, b, c) = (
            torsion_point(rng, &multiple, l),
            torsion_point(rng, &multiple, l),
            torsion_point(rng, &multiple, l),
        );
        points.extend([a, b, c]);
        let sum = (a + b).into_affine();
        if !sum.is_zero() {
            points.push(sum);
        }
    }
    points
}

/// The new check against the old one and, for a point of the curve, against upstream arkworks
/// and the ground truth; the result
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
    new
}

/// The callers of the check on a point of the curve, against arkworks: `Valid::check`,
/// the validating deserializations (compressed, uncompressed), the decompression with a square
/// root supplied from outside (zksync-os's oracle path), the EIP-2537 parser followed by the
/// check as in zksync-os's `parse_g1_with_subgroup_check`; all accept exactly the points of G1
fn check_callers(p: &RefAffine) {
    let expected = in_g1(p);
    let ours = from_ref(p);
    assert_eq!(ours.check().is_ok(), expected, "Valid::check at {p}");
    assert_eq!(p.check().is_ok(), expected, "arkworks Valid::check at {p}");
    for compressed in [true, false] {
        let (mut bytes, mut reference_bytes) = (vec![], vec![]);
        match compressed {
            true => {
                ours.serialize_compressed(&mut bytes).unwrap();
                p.serialize_compressed(&mut reference_bytes).unwrap();
            }
            false => {
                ours.serialize_uncompressed(&mut bytes).unwrap();
                p.serialize_uncompressed(&mut reference_bytes).unwrap();
            }
        }
        assert_eq!(bytes, reference_bytes);
        let (decoded, reference) = match compressed {
            true => (
                G1Affine::deserialize_compressed(&bytes[..]),
                RefAffine::deserialize_compressed(&bytes[..]),
            ),
            false => (
                G1Affine::deserialize_uncompressed(&bytes[..]),
                RefAffine::deserialize_uncompressed(&bytes[..]),
            ),
        };
        assert_eq!(decoded.is_ok(), expected, "deserialization at {p}");
        assert_eq!(
            reference.is_ok(),
            expected,
            "arkworks deserialization at {p}"
        );
        if let (Ok(decoded), Ok(reference)) = (decoded, reference) {
            assert_eq!(to_ref(&decoded), reference);
        }
        if compressed {
            let decoded = g1_from_compressed_with_sqrt(&bytes, |y_squared| y_squared.sqrt());
            assert_eq!(
                decoded.is_ok(),
                expected,
                "decompression with a root at {p}"
            );
            if let Ok(decoded) = decoded {
                assert_eq!(to_ref(&decoded), *p);
            }
        }
    }
    let mut eip2537 = [0u8; 128];
    if !p.is_zero() {
        for (at, c) in [(16, p.x), (80, p.y)] {
            let integer = BigUint::from(c).to_bytes_be();
            eip2537[at + 48 - integer.len()..at + 48].copy_from_slice(&integer);
        }
    }
    let (point, _) = parse_g1_bytes(&eip2537).expect("a point of the curve");
    assert_eq!(to_ref(&point), *p);
    assert_eq!(
        point.is_zero() || point.is_in_correct_subgroup_assuming_on_curve(),
        expected,
        "EIP-2537 at {p}"
    );
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
    let mut references = vec![RefAffine::identity(), g, -g];
    for k in [BigUint::from(1u32), BigUint::from(2u32), r() - 1u32] {
        references.push(mul_exact(&g, &k).into_affine());
    }
    for _ in 0..32 {
        references.push(random_g1(&mut rng));
    }
    // cleared cofactors: [h]R and [k h]R
    for _ in 0..8 {
        let p = random_point(&mut rng);
        let k: BigUint = RefFr::rand(&mut rng).into();
        for multiple in [h(), k * h()] {
            references.push(mul_exact(&p, &multiple).into_affine());
        }
    }
    points.extend(references.iter().map(from_ref));
    for p in &points {
        assert!(check(p), "{p:?}");
    }
    for p in &references {
        check_callers(p);
    }
}

#[test]
fn points_outside_the_subgroup() {
    let mut rng = test_rng();
    let cofactor_points = cofactor_points(&mut rng);
    // not in G1 by construction
    let mut points = cofactor_points.clone();
    for t in &cofactor_points {
        let g = random_g1(&mut rng);
        points.extend([(g + t).into_affine(), (g - t).into_affine()]);
    }
    for _ in 0..64 {
        let p = random_point(&mut rng);
        points.extend([p, -p]);
    }
    for p in &points {
        assert!(!p.is_zero() && p.is_on_curve());
        assert!(!check(&from_ref(p)), "{p:?}");
        check_callers(p);
    }
    // random multiples of points of the cofactor part and of random points, and sums of points
    // of the cofactor part: labelled by the ground truth only
    for i in 0..32 {
        let k: BigUint = RefFr::rand(&mut rng).into();
        let t = cofactor_points[i % cofactor_points.len()];
        let u = cofactor_points[(7 * i + 3) % cofactor_points.len()];
        let p = random_point(&mut rng);
        for q in [
            mul_exact(&t, &k),
            mul_exact(&p, &k),
            mul_exact(&p, &(&k * r())),
            t + u,
            random_g1(&mut rng) + t + u,
        ] {
            let q = q.into_affine();
            check(&from_ref(&q));
            check_callers(&q);
        }
    }
}

/// Outside the contract of the check (`deserialize_with_mode` without compression calls it on
/// points off the curve): only the old and the new check are compared
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
    for _ in 0..64 {
        points.push((RefFq::rand(&mut rng), RefFq::rand(&mut rng)));
    }
    for _ in 0..16 {
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

#[test]
fn random_sweep() {
    let mut rng = StdRng::seed_from_u64(39);
    let cofactor_points = cofactor_points(&mut rng);
    let mut in_subgroup = 0;
    for i in 0..4096 {
        let p = random_point(&mut rng);
        in_subgroup += check(&from_ref(&p)) as usize;
        let g = random_g1(&mut rng);
        assert!(check(&from_ref(&g)));
        let t = cofactor_points[i % cofactor_points.len()];
        assert!(!check(&from_ref(&(g + t).into_affine())));
        assert!(!check(&from_ref(&(p + t).into_affine())) || in_g1(&p));
    }
    assert_eq!(in_subgroup, 0);
}

#[test]
fn random_points_proptest() {
    let cofactor_points = cofactor_points(&mut test_rng());
    let n = cofactor_points.len();
    let config = ProptestConfig::with_cases(256);
    proptest!(config, |(
        x_bytes in any::<[u8; 48]>(),
        greatest in any::<bool>(),
        k_bytes in any::<[u8; 32]>(),
        t in 0..n,
    )| {
        // the next point of the curve from x
        let mut x = RefFq::from_be_bytes_mod_order(&x_bytes);
        let p = (0..256)
            .find_map(|_| {
                let p = RefAffine::get_point_from_x_unchecked(x, greatest);
                x += RefFq::ONE;
                p
            })
            .expect("a point of the curve");
        let k = RefFr::from_be_bytes_mod_order(&k_bytes);
        let g = (RefProjective::generator() * k).into_affine();
        let t = cofactor_points[t];
        for q in [p, -p, g, (g + t).into_affine(), (p + t).into_affine()] {
            let ours = from_ref(&q);
            let new = ours.is_in_correct_subgroup_assuming_on_curve();
            prop_assert_eq!(new, subgroup_check_before(&ours));
            prop_assert_eq!(new, q.is_in_correct_subgroup_assuming_on_curve());
            prop_assert_eq!(new, in_g1(&q));
        }
    });
}
