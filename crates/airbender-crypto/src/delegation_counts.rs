//! Delegation-call counts of the field, tower and curve operations, on the host emulation of
//! the bigint delegation: the cost model of the proving target is the number of these calls.
//! Run with `cargo test -p airbender-crypto --lib delegation_counts -- --nocapture`.

use crate::bigint_delegation::delegation::DELEGATION_CALLS;
use crate::extension_tower::tests::{rand_fp, rand_fp12, rand_fp2, rand_fp6};
use crate::extension_tower::*;
use ark_ec::pairing::Pairing;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::AdditiveGroup;
use ark_ff::{
    CyclotomicMultSubgroup, Field, Fp12, Fp12Config, Fp2, Fp2Config, Fp6, Fp6Config, PrimeField,
    UniformRand,
};
use ark_serialize::Valid;
use ark_std::test_rng;

use crate::bigint_delegation::delegation::DELEGATION_CALLS_BY_OP;

fn count(f: impl FnOnce()) -> u64 {
    DELEGATION_CALLS.with(|c| c.set(0));
    DELEGATION_CALLS_BY_OP.with(|c| *c.borrow_mut() = [0; 8]);
    f();
    DELEGATION_CALLS.with(|c| c.get())
}

/// The last count split by operation: add, sub, sub-negate, mul-low, mul-high, eq, carry?, memcpy
fn by_op() -> String {
    let ops = [
        "add", "sub", "subneg", "mullo", "mulhi", "eq", "?", "memcpy",
    ];
    DELEGATION_CALLS_BY_OP.with(|c| {
        c.borrow()
            .iter()
            .zip(ops)
            .filter(|(n, _)| **n != 0)
            .map(|(n, name)| std::format!("{name} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    })
}

struct Rows(Vec<String>);

impl Rows {
    fn row(&mut self, name: &str, f: impl FnOnce()) {
        let n = count(f);
        self.0
            .push(std::format!("{name:<46} {n:>8}   [{}]", by_op()));
    }
    fn print(&self) {
        for l in &self.0 {
            println!("{l}");
        }
    }
}

fn tower_rows<P: Fp12Config>(rows: &mut Rows)
where
    P::Fp6Config: MulByXi,
    <P::Fp6Config as Fp6Config>::Fp2Config: NonresidueMinusOne,
{
    type F2<P> = Fp2<<<P as Fp12Config>::Fp6Config as Fp6Config>::Fp2Config>;
    type F6<P> = Fp6<<P as Fp12Config>::Fp6Config>;
    type F<P> = <<<P as Fp12Config>::Fp6Config as Fp6Config>::Fp2Config as Fp2Config>::Fp;
    let mut rng = test_rng();
    let (a, b): (F<P>, F<P>) = (rand_fp(&mut rng), rand_fp(&mut rng));
    let (a2, b2): (F2<P>, F2<P>) = (rand_fp2(&mut rng), rand_fp2(&mut rng));
    let (a6, b6): (F6<P>, F6<P>) = (rand_fp6(&mut rng), rand_fp6(&mut rng));
    let (a12, b12): (Fp12<P>, Fp12<P>) = (rand_fp12(&mut rng), rand_fp12(&mut rng));
    rows.row("Fq mul", || {
        let mut x = a;
        x *= &b;
    });
    rows.row("Fq square", || {
        let mut x = a;
        x.square_in_place();
    });
    rows.row("Fq add", || {
        let mut x = a;
        x += &b;
    });
    rows.row("Fq sub", || {
        let mut x = a;
        x -= &b;
    });
    rows.row("Fq double", || {
        let mut x = a;
        x.double_in_place();
    });
    rows.row("Fq neg", || {
        let mut x = a;
        x.neg_in_place();
    });
    rows.row("Fq inverse", || {
        let _ = a.inverse();
    });
    rows.row("Fq2 mul (in place)", || {
        let mut x = a2;
        fp2_mul_assign(&mut x, &b2);
    });
    rows.row("Fq2 mul arkworks", || {
        let mut x = a2;
        x *= &b2;
    });
    rows.row("Fq2 square (in place)", || {
        let mut x = a2;
        fp2_square_in_place(&mut x);
    });
    rows.row("Fq2 square arkworks", || {
        let mut x = a2;
        x.square_in_place();
    });
    rows.row("Fq2 mul by xi", || {
        let mut x = a2;
        fp2_mul_by_xi::<P::Fp6Config>(&mut x);
    });
    rows.row("Fq6 mul (in place)", || {
        let mut x = a6;
        fp6_mul_assign(&mut x, &b6);
    });
    rows.row("Fq6 mul arkworks", || {
        let mut x = a6;
        x *= &b6;
    });
    rows.row("Fq6 mul_by_01 (in place)", || {
        let mut x = a6;
        fp6_mul_by_01(&mut x, &b6.c0, &b6.c1);
    });
    rows.row("Fq12 mul (in place)", || {
        let mut x = a12;
        fp12_mul_assign(&mut x, &b12);
    });
    rows.row("Fq12 mul arkworks", || {
        let mut x = a12;
        x *= &b12;
    });
    rows.row("Fq12 square (in place)", || {
        let mut x = a12;
        fp12_square_in_place(&mut x);
    });
    rows.row("Fq12 square arkworks", || {
        let mut x = a12;
        x.square_in_place();
    });
    rows.row("Fq12 mul_by_034 (in place)", || {
        let mut x = a12;
        fp12_mul_by_034(&mut x, &b2, &a2, &b6.c2);
    });
    rows.row("Fq12 mul_by_034 arkworks", || {
        let mut x = a12;
        x.mul_by_034(&b2, &a2, &b6.c2);
    });
    rows.row("Fq12 mul_by_014 (in place)", || {
        let mut x = a12;
        fp12_mul_by_014(&mut x, &b2, &a2, &b6.c2);
    });
    rows.row("Fq12 mul_by_014 arkworks", || {
        let mut x = a12;
        x.mul_by_014(&b2, &a2, &b6.c2);
    });
    rows.row("Fq12 cyclotomic square (in place)", || {
        let mut x = a12;
        fp12_cyclotomic_square_in_place(&mut x);
    });
    rows.row("Fq12 cyclotomic square arkworks", || {
        let mut x = a12;
        x.cyclotomic_square_in_place();
    });
    rows.row("Fq12 inverse arkworks", || {
        let _ = a12.inverse();
    });
}

fn curve_rows<E: Pairing>(rows: &mut Rows) {
    let mut rng = test_rng();
    // random subgroup points as multiples of the generators (`UniformRand` on the points
    // samples the base field by rejection, which does not terminate for bls12-381)
    let scalar = |rng: &mut _| E::ScalarField::rand(rng).into_bigint();
    let p = E::G1Affine::generator()
        .mul_bigint(scalar(&mut rng))
        .into_affine();
    let p2 = E::G1Affine::generator()
        .mul_bigint(scalar(&mut rng))
        .into_affine();
    let q = E::G2Affine::generator()
        .mul_bigint(scalar(&mut rng))
        .into_affine();
    let s = E::ScalarField::rand(&mut rng);
    rows.row("G2 prepare", || {
        let _: E::G2Prepared = q.into();
    });
    rows.row("miller loop (1 pair)", || {
        let _ = E::multi_miller_loop([p], [q]);
    });
    let m = E::multi_miller_loop([p], [q]);
    rows.row("final exponentiation", || {
        let _ = E::final_exponentiation(m);
    });
    rows.row("pairing (1 pair) total", || {
        let _ = E::pairing(p, q);
    });
    rows.row("pairing (2 pairs) total", || {
        let _ = E::multi_pairing([p, p2], [q, q]);
    });
    rows.row("G1 scalar mul", || {
        let _ = p.mul_bigint(s.into_bigint());
    });
    rows.row("G1 add + into_affine", || {
        let _ = (p + p2).into_affine();
    });
    rows.row("G1 validation (on curve + subgroup)", || {
        let _ = p.check();
    });
    rows.row("G2 validation (on curve + subgroup)", || {
        let _ = q.check();
    });
}

#[test]
fn bn254() {
    use crate::bn254::curves::{Bn254, G2Affine};
    let mut rows = Rows(Vec::new());
    tower_rows::<crate::bn254::fields::Fq12Config>(&mut rows);
    curve_rows::<Bn254>(&mut rows);
    let q = G2Affine::generator()
        .mul_bigint(crate::bn254::Fr::rand(&mut test_rng()).into_bigint())
        .into_affine();
    rows.row("G2 subgroup check reference (6x^2 P)", || {
        let _ = crate::bn254::curves::g2::is_in_subgroup_reference(&q);
    });
    println!("=== bn254");
    rows.print();
}

#[test]
fn bls12_381() {
    use crate::bls12_381::curves::Bls12_381;
    let mut rows = Rows(Vec::new());
    tower_rows::<crate::bls12_381::fields::Fq12Config>(&mut rows);
    curve_rows::<Bls12_381>(&mut rows);
    println!("=== bls12-381");
    rows.print();
}
