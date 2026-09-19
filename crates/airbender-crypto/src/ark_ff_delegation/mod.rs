#[macro_use]
pub mod biginteger;
mod const_helpers;
pub(crate) mod fp;

pub(crate) use biginteger::BigIntMacro;
pub use biginteger::{BigInt, BigInteger};
pub use fp::{Fp, Fp256, Fp512, MontBackend, MontConfig, MontFp};

/// `a^e` with 4-bit fixed windows: one squaring per bit and one multiplication per non-zero
/// window (plus the 15 of the table) instead of one per set bit.
pub fn pow_window4<F: ark_ff::Field>(a: &F, e: &[u64]) -> F {
    let mut table = [F::ONE; 16];
    for i in 1..16 {
        table[i] = table[i - 1];
        table[i] *= a;
    }
    let mut res = F::ONE;
    let mut started = false;
    for limb in e.iter().rev() {
        for window in (0..16).rev() {
            let digit = ((limb >> (window * 4)) & 0xf) as usize;
            if started {
                for _ in 0..4 {
                    res.square_in_place();
                }
            }
            if digit != 0 {
                if started {
                    res *= &table[digit];
                } else {
                    res = table[digit];
                    started = true;
                }
            }
        }
    }
    res
}

#[cfg(test)]
mod pow_tests {
    #[test]
    fn windowed_pow_matches_square_and_multiply() {
        use ark_ff::{Field, UniformRand};
        use ark_std::test_rng;
        let mut rng = test_rng();
        for _ in 0..50 {
            let a = ark_bn254::Fq::rand(&mut rng);
            let e = [
                u64::rand(&mut rng),
                u64::rand(&mut rng),
                u64::rand(&mut rng),
                u64::rand(&mut rng),
            ];
            assert_eq!(super::pow_window4(&a, &e), a.pow(e));
            assert_eq!(super::pow_window4(&a, &[0, 0, 0, 0]), ark_bn254::Fq::ONE);
            assert_eq!(super::pow_window4(&a, &[1, 0, 0, 0]), a);
        }
    }
}
