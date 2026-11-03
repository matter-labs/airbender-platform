//! Variable-time field inversion: the safegcd of the secp256k1 field (`modinv64`) for prime fields
//! with 4-limb moduli (up to 256 bits), the arkworks inversion otherwise

use crate::secp256k1::field::mod_inv64::{ModInfo, Signed62};
use ark_ff::{BigInteger, Field, PrimeField};

const M62: u64 = u64::MAX >> 2;

fn to_signed62(w: [u64; 4]) -> Signed62 {
    Signed62([
        (w[0] & M62) as i64,
        ((w[0] >> 62 | w[1] << 2) & M62) as i64,
        ((w[1] >> 60 | w[2] << 4) & M62) as i64,
        ((w[2] >> 58 | w[3] << 6) & M62) as i64,
        (w[3] >> 56) as i64,
    ])
}

fn from_signed62(s: Signed62) -> [u64; 4] {
    let [a0, a1, a2, a3, a4] = s.0.map(|limb| limb as u64);
    [
        a0 | a1 << 62,
        a1 >> 2 | a2 << 60,
        a2 >> 4 | a3 << 58,
        a3 >> 6 | a4 << 56,
    ]
}

fn words<B: BigInteger>(b: &B) -> [u64; 4] {
    b.as_ref().try_into().expect("a 4-limb representation")
}

/// `1 / m mod 2^62` of an odd `m`
fn inv_mod_2_62(m: u64) -> u64 {
    let mut x = m;
    for _ in 0..5 {
        x = x.wrapping_mul(2u64.wrapping_sub(m.wrapping_mul(x)));
    }
    x & M62
}

/// The inverse of `value` modulo the odd `modulus`, little-endian words with `value < modulus`;
/// zero for zero
pub(crate) fn inverse_words(value: [u64; 4], modulus: [u64; 4]) -> [u64; 4] {
    let mod_info = ModInfo::new(to_signed62(modulus).0, inv_mod_2_62(modulus[0]));
    from_signed62(to_signed62(value).modinv64(&mod_info))
}

fn safegcd_inverse<F: PrimeField>(a: &F) -> Option<F> {
    if a.is_zero() {
        return None;
    }
    let inverse = inverse_words(words(&a.into_bigint()), words(&F::MODULUS));
    let mut b = F::BigInt::default();
    b.as_mut().copy_from_slice(&inverse);
    F::from_bigint(b)
}

/// The inverse of `a`, `None` for zero
pub fn inverse<F: Field>(a: &F) -> Option<F> {
    if F::extension_degree() != 1 || F::BasePrimeField::MODULUS.as_ref().len() != 4 {
        return a.inverse();
    }
    let element = a
        .to_base_prime_field_elements()
        .next()
        .expect("a prime field element");
    safegcd_inverse(&element).map(F::from_base_prime_field)
}

#[cfg(test)]
mod tests {
    use super::inverse;
    use ark_ff::Field;

    fn check<F: Field>() {
        let c = F::from(0x9e37_79b9_7f4a_7c15u64);
        let mut x = F::from(12345u64);
        for special in [F::one(), -F::one(), F::from(2u64), -F::from(2u64)] {
            assert_eq!(inverse(&special), special.inverse());
        }
        for _ in 0..10_000 {
            x = x * c + F::from(7u64);
            assert_eq!(inverse(&x), x.inverse());
        }
        assert_eq!(inverse(&F::zero()), None);
    }

    #[test]
    fn matches_arkworks() {
        check::<ark_bn254::Fq>();
        check::<ark_bn254::Fr>();
        check::<ark_bls12_381::Fr>();
        check::<ark_bls12_381::Fq>();
        check::<ark_bn254::Fq2>();
    }
}
