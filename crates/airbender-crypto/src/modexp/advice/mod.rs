//! ModExp with hints: a modulus of up to 32 bytes on the single-digit path (`single_digit`), a
//! longer one on the multi-digit path (`bigint`), both walking the exponent as `exponent` does.
//!
//! One algorithm for every target: what is asked, what is answered and what is checked do not
//! depend on the arithmetic under it, which is selected by `cfg(bigint_ops)` alone: the 256-bit
//! values of the single-digit path in `modexp::u256`, the digits of the multi-digit path in
//! `arith`. With the delegation both are sequences of delegation calls; without it (RV64, and
//! hosts built without `bigint_ops`) both are carry chains on `u64` limbs. The queries, and so
//! the hint tape, are the same on either.

use bigint::ModexpAdvisor;

mod arith;
mod bigint;
mod exponent;
mod single_digit;

use crate::hints::wide_div;

use self::bigint::BigintRepr;
use crate::modexp::{checked_operand, ModExpOutput};
use airbender_oracle::MemoryOracle;

/// `base^exp mod modulus` for big-endian operands, with hints from `oracle`. Every hint is checked
/// and a wrong one panics. A zero result (also for a modulus of 0 or 1) has no bytes;
/// [`write_modexp_output`](super::write_modexp_output) pads it. The base and the modulus must
/// have at most [`MAX_OPERAND_BYTES`](super::MAX_OPERAND_BYTES) significant bytes.
pub fn modexp<O: MemoryOracle>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
    oracle: &mut O,
) -> ModExpOutput {
    let mut advisor = self::bigint::OracleAdvisor { inner: oracle };

    modexp_inner(base, exp, modulus, &mut advisor)
}

/// [`modexp`] with hints computed natively instead of by an oracle, for fuzzing.
#[cfg(any(test, feature = "testing"))]
pub fn delegated_modexp_with_naive_advisor(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
) -> ModExpOutput {
    let mut advisor = bigint::naive_advisor::NaiveAdvisor;
    modexp_inner(base, exp, modulus, &mut advisor)
}

fn modexp_inner(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
    advisor: &mut impl ModexpAdvisor,
) -> ModExpOutput {
    let modulus_digits = checked_operand(modulus);
    let base = checked_operand(base);
    if modulus_digits.is_empty() {
        return ModExpOutput::new();
    }
    if modulus_digits.len() <= 32 {
        // one digit: the U256 path
        let mut padded = [0u8; 32];
        padded[32 - modulus_digits.len()..].copy_from_slice(modulus_digits);
        let m = crate::modexp::u256::U256::from_be_bytes(&padded);
        if m.is_one() {
            // x^e mod 1 = 0
            return ModExpOutput::new();
        }
        return single_digit::modexp(base, exp, &m, advisor);
    }

    let m = BigintRepr::from_big_endian(modulus_digits);
    debug_assert!(m.digits > 1);
    let x = BigintRepr::from_big_endian(base);
    let x = x.modpow(exp, m, advisor);
    x.to_big_endian()
}

#[cfg(test)]
mod test {
    use super::arith::{DIGIT_BYTES, DIGIT_LIMBS, MAX_DIGITS};
    use super::bigint::naive_advisor::NaiveAdvisor;
    use super::*;
    use crate::modexp::u256::U256;
    use num_bigint::BigUint;
    use num_traits::Zero;
    use std::vec;

    fn modexp_with_advisor(
        base: &[u8],
        exp: &[u8],
        modulus: &[u8],
        advisor: &mut impl ModexpAdvisor,
    ) -> ModExpOutput {
        super::modexp_inner(base, exp, modulus, advisor)
    }

    fn modexp_honest(base: &[u8], exp: &[u8], modulus: &[u8]) -> ModExpOutput {
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

    /// The exact output bytes: none for zero, otherwise whole digits: 32 bytes for a modulus of
    /// up to 32 bytes, the digits of the arithmetic for a longer one
    #[test]
    fn output_bytes() {
        let check = |base: &[u8], exp: &[u8], modulus: &[u8], output: &[u8]| {
            assert_eq!(modexp_honest(base, exp, modulus), output, "{modulus:02x?}");
        };
        let mut one = vec![0u8; 32];
        one[31] = 1;
        check(&[0], &[1], &[2], &[]);
        check(&[0], &[0], &[2], &one);
        check(&[0], &[0], &[0xff; 64], &one[32 - DIGIT_BYTES..]);
        // the zero top digits of a result are dropped, not its zero top bytes
        let mut small = vec![0u8; DIGIT_BYTES];
        small[DIGIT_BYTES - 1] = 9;
        check(&[3], &[2], &[0xff; 64], &small);
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

    /// Bits of a digit of the arithmetic
    const DIGIT_BITS: usize = 8 * DIGIT_BYTES;

    /// The digits of a forged quotient or remainder as written: 64-bit limbs, low first, in
    /// whole digits of the arithmetic
    type Digits = std::vec::Vec<u64>;

    /// The digits of `x`, none for zero
    fn digits_of(x: &BigUint) -> Digits {
        let mut limbs: Digits = x.iter_u64_digits().collect();
        limbs.resize(limbs.len().next_multiple_of(DIGIT_LIMBS), 0);
        limbs
    }

    fn digit_count(digits: &Digits) -> usize {
        digits.len() / DIGIT_LIMBS
    }

    /// `digits` as `count` digits: cut, or extended by digits of limbs `limb`
    fn with_digits(mut digits: Digits, count: usize, limb: u64) -> Digits {
        digits.resize(count * DIGIT_LIMBS, limb);
        digits
    }

    /// `count` digits of all ones
    fn ones(count: usize) -> Digits {
        vec![u64::MAX; count * DIGIT_LIMBS]
    }

    /// Writes `digits` as they are (zero top digits too), as many as the buffer holds
    fn write_digits(digits: &[u64], dst: &mut BigintRepr) {
        let capacity = dst.clear_as_capacity_mut();
        let count = (digits.len() / DIGIT_LIMBS).min(capacity.len());
        let limbs = capacity.as_mut_ptr().cast::<u64>();
        for (i, limb) in digits[..count * DIGIT_LIMBS].iter().enumerate() {
            // SAFETY: a limb of the first `count` slots, each `DIGIT_LIMBS` aligned limbs
            unsafe { limbs.add(i).write(*limb) };
        }
        // SAFETY: the first `count` slots were written, within the capacity
        unsafe { dst.set_num_digits(count) };
    }

    /// Reduction advice written as given, one answer per query
    struct MaliciousReductionAdvisor {
        /// (quotient, remainder)
        responses: std::vec::Vec<(Digits, Digits)>,
        next_response: usize,
    }

    impl ModexpAdvisor for MaliciousReductionAdvisor {
        fn get_reduction_op_advice(
            &mut self,
            _a: &BigintRepr,
            _m: &BigintRepr,
            quotient_dst: &mut BigintRepr,
            remainder_dst: &mut BigintRepr,
        ) {
            let (quotient, remainder) = &self.responses[self.next_response];
            self.next_response += 1;
            write_digits(quotient, quotient_dst);
            write_digits(remainder, remainder_dst);
        }

        fn wide_quotient(&mut self, _lo: &U256, _hi: &U256, _modulus: &U256) -> (U256, U256) {
            unreachable!("the multi-digit path asks for no wide quotient")
        }
    }

    /// `m = 2^512 - 1`, `x = 2^256 + 2`, so `x^2 = m + r` with `r = 4 * 2^256 + 5`. With `D` the
    /// base of the digits, the forged answer for the squaring is `q = 1 + D`, `r + D`:
    /// `q * m + r` is then `x^2 + D * 2^512`, which equals `x^2` modulo `D * 2^512`, the digits
    /// of the product, and must be rejected because it does not fit them.
    #[test]
    #[should_panic(expected = "modexp advice: q * m + r does not fit the digits of the product")]
    fn test_modexp_delegation_malicious_advisor_truncates_reconstruction_regression() {
        let mut base = vec![0u8; 33];
        base[0] = 0x01;
        base[32] = 0x02;
        let modulus = [0xff; 64];

        // 4 * 2^256 + 5, in whole digits
        let mut expected = [0u8; 64];
        expected[31] = 0x04;
        expected[63] = 0x05;
        let significant = (33usize).next_multiple_of(DIGIT_BYTES);
        assert_eq!(
            modexp_honest(&base, &[2], &modulus),
            expected[64 - significant..]
        );

        let one = BigUint::from(1u32);
        let digit_base = &one << DIGIT_BITS;
        let x = BigUint::from_bytes_be(&base);
        let r = (BigUint::from(4u32) << 256) + 5u32;
        let mut advisor = MaliciousReductionAdvisor {
            responses: vec![
                // the initial reduction, honest: x = 0 * m + x
                (vec![], digits_of(&x)),
                (
                    digits_of(&(&one + &digit_base)),
                    digits_of(&(r + digit_base)),
                ),
            ],
            next_response: 0,
        };
        modexp_with_advisor(&base, &[2], &modulus, &mut advisor);
    }

    /// What a forgery answers for `a / m`, given the true quotient and remainder
    type Forgery = fn(&BigUint, &BigUint, &BigUint, &BigUint) -> (Digits, Digits);

    /// Answers reduction advice honestly, except call `forged_call` (every call for `None`),
    /// which gets what `forge` makes of the honest answer
    struct ForgingReductionAdvisor {
        calls: usize,
        forged_call: Option<usize>,
        forge: Forgery,
    }

    impl ModexpAdvisor for ForgingReductionAdvisor {
        fn get_reduction_op_advice(
            &mut self,
            a: &BigintRepr,
            m: &BigintRepr,
            quotient_dst: &mut BigintRepr,
            remainder_dst: &mut BigintRepr,
        ) {
            use super::bigint::naive_advisor::to_biguint;
            let (a, m) = (to_biguint(a), to_biguint(m));
            let (q, r) = (&a / &m, &a % &m);
            let call = self.calls;
            self.calls += 1;
            let (quotient, remainder) = if self.forged_call.is_none_or(|forged| forged == call) {
                (self.forge)(&q, &r, &a, &m)
            } else {
                (digits_of(&q), digits_of(&r))
            };
            write_digits(&quotient, quotient_dst);
            write_digits(&remainder, remainder_dst);
        }

        fn wide_quotient(&mut self, _lo: &U256, _hi: &U256, _modulus: &U256) -> (U256, U256) {
            unreachable!("the multi-digit path asks for no wide quotient")
        }
    }

    /// Answers that differ from the honest one in content, in length or only in their spelling
    /// (zero top digits, a remainder that is not reduced). The last ones are right values, and
    /// must not fail anywhere but in a check either.
    const FORGERIES: [Forgery; 20] = [
        // the honest answer
        |q, r, _, _| (digits_of(q), digits_of(r)),
        // zero top digits: on the remainder (a zero remainder gets digits), up to the modulus's
        // length, and on the quotient
        |q, r, _, _| {
            let remainder = digits_of(r);
            let count = digit_count(&remainder) + 1;
            (digits_of(q), with_digits(remainder, count, 0))
        },
        |q, r, _, m| {
            let count = digit_count(&digits_of(m));
            (digits_of(q), with_digits(digits_of(r), count, 0))
        },
        |q, r, _, _| {
            let quotient = digits_of(q);
            let count = digit_count(&quotient) + 1;
            (with_digits(quotient, count, 0), digits_of(r))
        },
        // a remainder that is congruent but not reduced
        |q, r, _, m| {
            if q.is_zero() {
                (digits_of(q), digits_of(&(r + m)))
            } else {
                (digits_of(&(q - 1u32)), digits_of(&(r + m)))
            }
        },
        |q, r, _, m| {
            if *q < BigUint::from(2u32) {
                (digits_of(q), digits_of(r))
            } else {
                (digits_of(&(q - 2u32)), digits_of(&(r + m + m)))
            }
        },
        // wrong values of the right lengths
        |q, r, _, _| (digits_of(&(q + 1u32)), digits_of(r)),
        |q, r, _, _| (digits_of(q), digits_of(&(r + 1u32))),
        |q, r, _, m| (digits_of(&(q + 1u32)), digits_of(&(r + m))),
        |_, _, a, _| (Digits::new(), digits_of(a)),
        |_, _, a, _| (digits_of(a), Digits::new()),
        |_, _, _, _| (Digits::new(), Digits::new()),
        |q, _, _, m| {
            (
                ones(digit_count(&digits_of(q))),
                ones(digit_count(&digits_of(m))),
            )
        },
        // a quotient off by one, down and up, and a remainder equal to the modulus
        |q, r, _, _| {
            if q.is_zero() {
                (digits_of(&(q + 2u32)), digits_of(r))
            } else {
                (digits_of(&(q - 1u32)), digits_of(r))
            }
        },
        |q, _, _, m| (digits_of(q), digits_of(m)),
        // a wrong top limb alone, where the honest one is zero or not
        |q, r, _, m| {
            let mut remainder = with_digits(digits_of(r), digit_count(&digits_of(m)), 0);
            *remainder.last_mut().unwrap() ^= 1 << 63;
            (digits_of(q), remainder)
        },
        // `q * m + r` that is the product only modulo a power of the digit base
        |q, r, a, m| {
            let (a_digits, m_digits) = (digit_count(&digits_of(a)), digit_count(&digits_of(m)));
            let shift = DIGIT_BITS * (a_digits - m_digits.min(a_digits));
            (
                digits_of(&(q + (BigUint::from(1u32) << shift))),
                digits_of(r),
            )
        },
        // too long: by a digit each, and whole buffers
        |q, r, _, m| {
            let count = digit_count(&digits_of(m)) + 1;
            (digits_of(q), with_digits(digits_of(r), count, 1))
        },
        |q, r, a, _| {
            let count = digit_count(&digits_of(a)) + 2;
            (with_digits(digits_of(q), count, 1), digits_of(r))
        },
        |_, _, _, _| (ones(2 * MAX_DIGITS + 8), ones(2 * MAX_DIGITS + 8)),
    ];

    /// The output of a ModExp with `advisor`, or the message of its panic
    fn outcome(
        base: &[u8],
        exp: &[u8],
        modulus: &[u8],
        advisor: &mut impl ModexpAdvisor,
    ) -> Result<std::vec::Vec<u8>, std::string::String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            modexp_with_advisor(base, exp, modulus, advisor).to_vec()
        }))
        .map_err(|payload| match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(payload) => *payload
                .downcast::<std::string::String>()
                .expect("a panic message"),
        })
    }

    fn reference(base: &[u8], exp: &[u8], modulus: &[u8]) -> BigUint {
        BigUint::from_bytes_be(base).modpow(
            &BigUint::from_bytes_be(exp),
            &BigUint::from_bytes_be(modulus),
        )
    }

    /// `2^bits` in big-endian bytes
    fn power_of_two(bits: usize) -> std::vec::Vec<u8> {
        let mut bytes = vec![0u8; bits / 8 + 1];
        bytes[0] = 1 << (bits % 8);
        bytes
    }

    /// Operands for the forgeries: both walks, a composite modulus whose residues become zero,
    /// and the longest operands EIP-7823 allows
    fn forgery_cases() -> std::vec::Vec<(std::vec::Vec<u8>, std::vec::Vec<u8>, std::vec::Vec<u8>)> {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let mut odd = rng.bytes(70);
        odd[69] |= 1;
        let mut just_above = power_of_two(4096);
        just_above[512] = 1;
        vec![
            // the binary and the windowed walk, with an initial reduction
            (rng.bytes(100), vec![0x1d], odd.clone()),
            (rng.bytes(40), rng.bytes(9), odd),
            // 2^512 divides the powers of 2^300 from the square on: zero remainders
            (power_of_two(300), vec![0x0b], power_of_two(512)),
            // a base equal to the modulus: the initial reduction gives zero
            (vec![0xff; 64], vec![5], vec![0xff; 64]),
            // 513-byte operands: products of 34 digits
            (power_of_two(4096), vec![2], just_above),
            // 1024-byte operands: products that fill the buffers
            (vec![0xff; 1024], vec![3], {
                let mut modulus = vec![0xff; 1024];
                modulus[1023] = 0xfd;
                modulus
            }),
        ]
    }

    /// Whatever an advisor answers, ModExp fails only in a check of the advice, and when it
    /// returns its output is the right one, which the output buffer holds.
    #[test]
    fn forged_reduction_advice_fails_only_in_its_checks() {
        let (mut accepted, mut rejected) = (0, 0);
        for (base, exp, modulus) in forgery_cases() {
            let expected = reference(&base, &exp, &modulus);
            let mut honest = ForgingReductionAdvisor {
                calls: 0,
                forged_call: Some(usize::MAX),
                forge: FORGERIES[0],
            };
            let output = outcome(&base, &exp, &modulus, &mut honest).expect("honest advice");
            assert_eq!(BigUint::from_bytes_be(&output), expected);
            let calls = honest.calls;
            assert!(calls > 0, "the case asks for advice");

            let mut forged_calls = vec![None, Some(0), Some(1), Some(calls / 2), Some(calls - 1)];
            forged_calls.dedup();
            for forged_call in forged_calls {
                if forged_call.is_some_and(|call| call >= calls) {
                    continue;
                }
                for (index, forge) in FORGERIES.iter().enumerate() {
                    let mut advisor = ForgingReductionAdvisor {
                        calls: 0,
                        forged_call,
                        forge: *forge,
                    };
                    match outcome(&base, &exp, &modulus, &mut advisor) {
                        Ok(output) => {
                            assert!(output.len() <= crate::modexp::MAX_OPERAND_BYTES);
                            assert_eq!(
                                BigUint::from_bytes_be(&output),
                                expected,
                                "forgery {index} at {forged_call:?} is accepted with a wrong result"
                            );
                            accepted += 1;
                        }
                        Err(message) => {
                            assert!(
                                message.starts_with("modexp advice: "),
                                "forgery {index} at {forged_call:?} fails outside the checks: {message}"
                            );
                            assert_ne!(index, 0, "honest advice is rejected: {message}");
                            rejected += 1;
                        }
                    }
                }
            }
        }
        // both happen: the spellings of right answers are accepted, the wrong answers rejected
        assert!(
            accepted > 50 && rejected > 50,
            "{accepted} accepted, {rejected} rejected"
        );
    }

    /// A zero remainder spelled with digits is a right answer: the next product is zero, and is
    /// not a reason to fail (regression: it reached an internal assertion).
    #[test]
    fn a_zero_remainder_with_digits_is_accepted() {
        let (base, exp, modulus) = (power_of_two(300), vec![0x0b], power_of_two(512));
        for forge in [FORGERIES[1], FORGERIES[2]] {
            let mut advisor = ForgingReductionAdvisor {
                calls: 0,
                forged_call: None,
                forge,
            };
            let output = outcome(&base, &exp, &modulus, &mut advisor).expect("a right answer");
            assert!(output.is_empty(), "2^3300 mod 2^512 = 0");
        }
    }

    /// A remainder that is congruent but not below the modulus is caught by the last check when
    /// it is the result
    #[test]
    fn an_unreduced_result_is_rejected() {
        // x^1 for a base of the modulus's length: the initial reduction is the result
        let (base, modulus) = (vec![0x80; 64], vec![0x7f; 64]);
        let mut advisor = ForgingReductionAdvisor {
            calls: 0,
            forged_call: Some(0),
            forge: FORGERIES[4],
        };
        assert_eq!(
            outcome(&base, &[1], &modulus, &mut advisor),
            Err("modexp advice: the result is not below the modulus".to_owned())
        );
    }

    /// The honest advisor serves operands up to the bound, whose products are double width
    #[test]
    fn naive_advisor_matches_reference_at_the_bound() {
        for (base, exp, modulus) in forgery_cases() {
            assert_matches_reference(&base, &exp, &modulus);
        }
        let mut rng = Rng(0x1234_5678_9abc_def1);
        for len in [513, 800, 1024] {
            let mut modulus = rng.bytes(len);
            modulus[0] |= 0x80;
            assert_matches_reference(&rng.bytes(len), &rng.bytes(2), &modulus);
        }
    }

    #[test]
    fn naive_advisor_reduces_a_513_byte_square() {
        use super::delegated_modexp_with_naive_advisor;

        let mut base = vec![0u8; 513];
        base[0] = 1; // 2^4096

        let mut modulus = base.clone();
        modulus[512] = 1; // 2^4096 + 1

        let result = delegated_modexp_with_naive_advisor(&base, &[2], &modulus);
    }
}
