use alloc::vec::Vec;
use bigint::ModexpAdvisor;
use core::alloc::Allocator;

mod bigint;
mod digit;
mod exponent;
mod single_digit;

use self::bigint::BigintRepr;
use crate::modexp::strip_leading_zeroes;
use airbender_oracle::MemoryOracle;

/// `base^exp mod modulus` for big-endian operands, with hints from `oracle`. Every hint is checked
/// and a wrong one panics. A zero result (also for a modulus of 0 or 1) has no bytes;
/// [`write_modexp_output`](super::write_modexp_output) pads it.
pub fn modexp<O: MemoryOracle, A: Allocator + Clone>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
    oracle: &mut O,
    allocator: A,
) -> Vec<u8, A> {
    let mut advisor = self::bigint::OracleAdvisor { inner: oracle };

    modexp_inner::<A>(base, exp, modulus, &mut advisor, allocator)
}

/// [`modexp`] with hints computed natively instead of by an oracle, for fuzzing.
#[cfg(feature = "testing")]
pub fn delegated_modexp_with_naive_advisor(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
    use alloc::alloc::Global;
    let mut advisor = bigint::naive_advisor::NaiveAdvisor;
    modexp_inner::<Global>(base, exp, modulus, &mut advisor, Global)
}

#[allow(clippy::needless_borrow)]
fn modexp_inner<A: Allocator + Clone>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
    advisor: &mut impl ModexpAdvisor,
    allocator: A,
) -> Vec<u8, A> {
    let modulus_digits = strip_leading_zeroes(modulus);
    if modulus_digits.is_empty() {
        return Vec::new_in(allocator);
    }
    if modulus_digits.len() <= 32 {
        // one digit: the U256 path
        let mut padded = [0u8; 32];
        padded[32 - modulus_digits.len()..].copy_from_slice(modulus_digits);
        let m = crate::modexp::u256::U256::from_be_bytes(&padded);
        if m.is_one() {
            // x^e mod 1 = 0
            return Vec::new_in(allocator);
        }
        return single_digit::modexp(base, exp, &m, advisor, allocator);
    }

    let m = BigintRepr::from_big_endian_with_double_capacity(&modulus, allocator.clone());
    debug_assert!(m.digits > 1);
    {
        let min_capacity = m.capacity();
        let x = BigintRepr::from_big_endian_with_double_capacity_or_min_capacity(
            &base,
            min_capacity,
            allocator.clone(),
        );
        let x = x.modpow(&exp, m, advisor, allocator.clone());
        x.to_big_endian(allocator)
    }
}

#[cfg(test)]
mod test {
    use std::alloc::Global;

    use super::bigint::naive_advisor::NaiveAdvisor;
    use super::*;
    use crate::modexp::u256::U256;
    use num_bigint::BigUint;
    use num_traits::Zero;

    fn modexp_with_advisor(
        base: &[u8],
        exp: &[u8],
        modulus: &[u8],
        advisor: &mut impl ModexpAdvisor,
    ) -> Vec<u8> {
        super::modexp_inner(base, exp, modulus, advisor, Global)
    }

    fn modexp_honest(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
        modexp_with_advisor(base, exp, modulus, &mut NaiveAdvisor)
    }

    /// A deterministic xorshift64* generator
    struct Rng(u64);

    impl Rng {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn bytes(&mut self, len: usize) -> std::vec::Vec<u8> {
            (0..len).map(|_| self.next_u64() as u8).collect()
        }
    }

    fn assert_matches_reference(base: &[u8], exp: &[u8], modulus: &[u8]) {
        let output = modexp_honest(base, exp, modulus);
        let modulus_big = BigUint::from_bytes_be(modulus);
        let expected = if modulus_big.is_zero() {
            BigUint::zero()
        } else {
            BigUint::from_bytes_be(base).modpow(&BigUint::from_bytes_be(exp), &modulus_big)
        };
        assert_eq!(
            BigUint::from_bytes_be(&output),
            expected,
            "base {base:02x?} exp {exp:02x?} modulus {modulus:02x?}"
        );
    }

    /// The exact output bytes: none for zero, whole 32-byte digits otherwise
    #[test]
    fn output_bytes() {
        let check = |base: &[u8], exp: &[u8], modulus: &[u8], output: &[u8]| {
            assert_eq!(modexp_honest(base, exp, modulus), output, "{modulus:02x?}");
        };
        let mut one = vec![0u8; 32];
        one[31] = 1;
        check(&[0], &[1], &[2], &[]);
        check(&[0], &[0], &[2], &one);
        check(&[0], &[0], &[0xff; 64], &one);
        check(&[3], &[2], &[1], &[]);
        check(&[0], &[0], &[0, 0, 1], &[]);
        check(&[3], &[5], &[], &[]);
        check(&[3], &[5], &[0; 100], &[]);
    }

    /// Moduli of 1 to 32 bytes (small ones give two-digit quotients), bases of up to 4 digits,
    /// exponents on both walks, some sparse
    #[test]
    fn single_digit_modulus_matches_reference() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for round in 0..400 {
            let modulus_len = 1 + (rng.next_u64() as usize) % 32;
            let mut modulus = rng.bytes(modulus_len);
            if round % 7 == 0 {
                modulus = vec![1 + (rng.next_u64() as u8) % 16];
            }
            let base_len = (rng.next_u64() as usize) % 130;
            let base = rng.bytes(base_len);
            let exp_len = match round % 4 {
                0 => (rng.next_u64() as usize) % 8,
                1 => 8,
                2 => 9 + (rng.next_u64() as usize) % 24,
                _ => 32 + (rng.next_u64() as usize) % 40,
            };
            let mut exp = rng.bytes(exp_len);
            if round % 11 == 0 {
                for byte in exp.iter_mut() {
                    *byte &= 0x11;
                }
            }
            assert_matches_reference(&base, &exp, &modulus);
        }
    }

    #[test]
    fn single_digit_modulus_special_cases() {
        let m = vec![0xffu8; 32];
        // 0^0, 0^e, 1^e, base = m, base = m + 1, exp = 0 with a long base
        assert_matches_reference(&[], &[], &m);
        assert_matches_reference(&[0, 0], &[5], &m);
        assert_matches_reference(&[1], &[0xff; 40], &m);
        assert_matches_reference(&m, &[3], &m);
        let mut m_plus_one = vec![1u8];
        m_plus_one.extend(std::iter::repeat_n(0u8, 32));
        assert_matches_reference(&m_plus_one, &[3], &m);
        assert_matches_reference(&[0xab; 100], &[], &m);
        // even moduli, one with a zero-absorbing base
        assert_matches_reference(&[7], &[0xff; 9], &[2]);
        assert_matches_reference(&[2], &[0xff; 9], &[0, 0, 0x40]);
        // leading zero bytes do not change the path
        let mut padded = vec![0u8; 40];
        padded.extend_from_slice(&m);
        assert_matches_reference(&[0xab; 33], &[0xcd; 9], &padded);
        assert_matches_reference(&[0, 0, 0xab, 0xcd], &[0, 0, 0, 0x81], &[0, 0, 0xf1]);
    }

    /// Moduli of 2 to 5 digits, bases of up to 8 digits, exponents on both walks
    #[test]
    fn multi_digit_modulus_matches_reference() {
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        for round in 0..60 {
            let modulus_len = 33 + (rng.next_u64() as usize) % 128;
            let modulus = rng.bytes(modulus_len);
            let base_len = (rng.next_u64() as usize) % 260;
            let base = rng.bytes(base_len);
            let exp_len = match round % 3 {
                0 => (rng.next_u64() as usize) % 8,
                1 => 8 + (rng.next_u64() as usize) % 4,
                _ => 12 + (rng.next_u64() as usize) % 10,
            };
            let exp = rng.bytes(exp_len);
            assert_matches_reference(&base, &exp, &modulus);
        }
        let m = vec![0xffu8; 64];
        assert_matches_reference(&[], &[], &m);
        assert_matches_reference(&[0], &[5], &m);
        assert_matches_reference(&[1], &[0xff; 40], &m);
        assert_matches_reference(&m, &[3], &m);
        assert_matches_reference(&[0xab; 100], &[], &m);
        // 2^0xffff mod 2^508 = 0: the accumulator becomes zero
        let mut even = vec![0u8; 63];
        even.insert(0, 0x10);
        assert_matches_reference(&[2], &[0xff, 0xff], &even);
        // a carry in the delegated addition (regression)
        let mut modulus = vec![
            214, 2, 245, 148, 60, 16, 255, 255, 255, 255, 255, 255, 255, 12, 0, 0, 0, 216, 112,
            144, 135, 112, 173, 239, 243, 255, 194, 78, 78, 1, 46, 10, 211, 128, 5,
        ];
        modulus.resize(64, 0);
        assert_matches_reference(&[255], &[48, 255, 128, 209], &modulus);
    }

    /// Reduction advice written as given, one answer per query
    struct MaliciousReductionAdvisor {
        /// (quotient, remainder) as 64-bit limbs, one per digit
        responses: std::vec::Vec<(std::vec::Vec<u64>, std::vec::Vec<u64>)>,
        next_response: usize,
    }

    fn write_bigint_digits<A: core::alloc::Allocator + Clone>(
        digits: &[u64],
        dst: &mut BigintRepr<A>,
    ) {
        unsafe {
            let dst_capacity = dst.clear_as_capacity_mut();
            for (dst_digit, src_digit) in dst_capacity.iter_mut().zip(digits.iter()) {
                let dst_ptr: *mut u64 = dst_digit.as_mut_ptr().cast::<[u64; 4]>().cast();
                dst_ptr.write(*src_digit);
                dst_ptr.add(1).write(0);
                dst_ptr.add(2).write(0);
                dst_ptr.add(3).write(0);
            }
            dst.set_num_digits(digits.len());
        }
    }

    impl ModexpAdvisor for MaliciousReductionAdvisor {
        fn get_reduction_op_advice<A: core::alloc::Allocator + Clone>(
            &mut self,
            _a: &BigintRepr<A>,
            _m: &BigintRepr<A>,
            quotient_dst: &mut BigintRepr<A>,
            remainder_dst: &mut BigintRepr<A>,
        ) {
            let (quotient, remainder) = &self.responses[self.next_response];
            self.next_response += 1;
            write_bigint_digits(quotient, quotient_dst);
            write_bigint_digits(remainder, remainder_dst);
        }

        fn wide_quotient(&mut self, _lo: &U256, _hi: &U256, _modulus: &U256) -> (U256, U256) {
            unreachable!("the multi-digit path asks for no wide quotient")
        }
    }

    /// `m = 2^512 - 1`, `x = 2^256 + 2`, so `x^2 mod m = 4 * 2^256 + 5`. The forged answer for the
    /// squaring is `q = 2^256 + 1`, `r = 5 * 2^256 + 5`: `q * m + r = 2^768 + x^2`, which equals
    /// `x^2` modulo 2^768 and must be rejected because it does not fit.
    #[test]
    #[should_panic(expected = "the product does not fit")]
    fn test_modexp_delegation_malicious_advisor_truncates_reconstruction_regression() {
        let mut base = vec![0u8; 33];
        base[0] = 0x01;
        base[32] = 0x02;
        let modulus = vec![0xff; 64];

        let mut expected = vec![0u8; 64];
        expected[31] = 0x04;
        expected[63] = 0x05;
        assert_eq!(modexp_honest(&base, &[2], &modulus), expected);

        let mut advisor = MaliciousReductionAdvisor {
            responses: vec![
                // the initial reduction, honest: x = 0 * m + x
                (vec![], vec![2, 1]),
                (vec![1, 1], vec![5, 5]),
            ],
            next_response: 0,
        };
        modexp_with_advisor(&base, &[2], &modulus, &mut advisor);
    }
}
