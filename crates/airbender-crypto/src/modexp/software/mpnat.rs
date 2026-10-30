use super::arith::{
    big_wrapping_mul, big_wrapping_pow, borrowing_sub, carrying_add, compute_r_mod_n, in_place_add,
    in_place_mul_sub, in_place_shl, in_place_shr, join_as_double, mod_inv, monpro, monsq,
};

extern crate alloc;
use super::vec_in;
use alloc::vec::Vec;
use core::alloc::Allocator;

#[cfg(target_pointer_width = "32")]
pub type Word = u32;
#[cfg(target_pointer_width = "32")]
pub type DoubleWord = u64;

#[cfg(target_pointer_width = "64")]
pub type Word = u64;
#[cfg(target_pointer_width = "64")]
pub type DoubleWord = u128;

pub const WORD_BYTES: usize = core::mem::size_of::<Word>();
pub const WORD_BITS: usize = Word::BITS as usize;
pub const BASE: DoubleWord = (Word::MAX as DoubleWord) + 1;

/// Multi-precision natural number, represented in base `Word::MAX + 1 = 2^WORD_BITS`.
/// The digits are stored in little-endian order, i.e. digits[0] is the least
/// significant digit.
#[derive(Debug)]
pub struct MPNat<A: Allocator + Clone> {
    pub digits: Vec<Word, A>,
}

impl<A: Allocator + Clone> MPNat<A> {
    fn strip_leading_zeroes(a: &[u8]) -> (&[u8], bool) {
        let len = a.len();
        let end = a.iter().position(|&x| x != 0).unwrap_or(len);

        if end == len {
            (&[], true)
        } else {
            (&a[end..], false)
        }
    }

    // Koç's algorithm for inversion mod 2^k
    // https://eprint.iacr.org/2017/411.pdf
    fn koc_2017_inverse(aa: &Self, k: usize, allocator: A) -> Self {
        debug_assert!(aa.is_odd());

        let length = k / WORD_BITS;
        let mut b = MPNat {
            digits: vec_in!(allocator.clone(); 0; length + 1),
        };
        b.digits[0] = 1;

        let mut a = MPNat {
            digits: aa.digits.clone(),
        };
        // `resize` without `grow`
        if a.digits.capacity() < length + 1 {
            let mut new_digits = Vec::with_capacity_in(length + 1, a.digits.allocator().clone());
            new_digits.resize(length + 1, 0);
            new_digits[..a.digits.len()].copy_from_slice(a.digits.as_slice());
            a.digits = new_digits;
        } else {
            a.digits.resize(length + 1, 0);
        }

        let mut neg: bool = false;

        let mut res = MPNat {
            digits: vec_in!(allocator.clone(); 0; length + 1),
        };

        let (mut wordpos, mut bitpos) = (0, 0);

        for _ in 0..k {
            let x = b.digits[0] & 1;
            if x != 0 {
                if !neg {
                    // b = a - b
                    let mut tmp = MPNat {
                        digits: a.digits.clone(),
                    };
                    in_place_mul_sub(&mut tmp.digits, &b.digits, 1);
                    b = tmp;
                    neg = true;
                } else {
                    // b = b - a
                    in_place_add(&mut b.digits, &a.digits);
                }
            }

            in_place_shr(&mut b.digits, 1);

            res.digits[wordpos] |= x << bitpos;

            bitpos += 1;
            if bitpos == WORD_BITS {
                bitpos = 0;
                wordpos += 1;
            }
        }

        res
    }

    pub fn from_big_endian(bytes: &[u8], allocator: A) -> Self {
        if bytes.is_empty() {
            return Self {
                digits: vec_in!(allocator.clone(); 0),
            };
        }
        // Remainder on division by WORD_BYTES
        let r = bytes.len() & (WORD_BYTES - 1);
        let n_digits = if r == 0 {
            bytes.len() / WORD_BYTES
        } else {
            // Need an extra digit for the remainder
            (bytes.len() / WORD_BYTES) + 1
        };
        let mut digits = vec_in!(allocator.clone(); 0; n_digits);
        // buffer to hold Word-sized slices of the input bytes
        let mut buf = [0u8; WORD_BYTES];
        let mut i = n_digits - 1;
        if r != 0 {
            buf[(WORD_BYTES - r)..].copy_from_slice(&bytes[0..r]);
            digits[i] = Word::from_be_bytes(buf);
            if i == 0 {
                // Special case where there is just one digit
                return Self { digits };
            }
            i -= 1;
        }
        let mut j = r;
        loop {
            let next_j = j + WORD_BYTES;
            buf.copy_from_slice(&bytes[j..next_j]);
            digits[i] = Word::from_be_bytes(buf);
            if i == 0 {
                break;
            } else {
                i -= 1;
                j = next_j;
            }
        }
        // throw away leading zeros
        while digits.len() > 1 && digits[digits.len() - 1] == 0 {
            digits.pop();
        }
        Self { digits }
    }

    pub fn is_power_of_two(&self) -> bool {
        // A multi-precision number is a power of 2 iff exactly one digit
        // is a power of 2 and all others are zero.
        let mut found_power_of_two = false;
        for &d in self.digits.iter() {
            let is_p2 = d.is_power_of_two();
            if (!is_p2 && d != 0) || (is_p2 && found_power_of_two) {
                return false;
            } else if is_p2 {
                found_power_of_two = true;
            }
        }
        found_power_of_two
    }

    pub fn is_odd(&self) -> bool {
        // A binary number is odd iff its lowest order bit is set.
        self.digits[0] & 1 == 1
    }

    /// Computes `self ^ exp mod modulus`. `exp` must be given as big-endian bytes.
    pub fn modpow(&mut self, exp: &[u8], modulus: &Self, allocator: A) -> Self {
        // exp must be stripped because it is iterated over in
        // big_wrapping_pow and modpow_montgomery, and a large
        // zero-padded exp leads to performance issues.
        let (exp, exp_is_zero) = Self::strip_leading_zeroes(exp);

        // base^0 is always 1, regardless of base.
        // Hence, the result is 0 for (base^0) % 1, and 1
        // for every modulus larger than 1.
        //
        // The case of modulus being 0 should have already been
        // handled in modexp().
        debug_assert!(!(modulus.digits.len() == 1 && modulus.digits[0] == 0));
        if exp_is_zero {
            if modulus.digits.len() == 1 && modulus.digits[0] == 1 {
                return Self {
                    digits: vec_in!(allocator.clone(); 0),
                };
            } else {
                return Self {
                    digits: vec_in!(allocator.clone(); 1),
                };
            }
        }

        // 0^exp mod m = 0 for any exp > 0 (exp > 0 guaranteed above)
        if self.digits.iter().all(|&d| d == 0) {
            return Self {
                digits: vec_in!(allocator.clone(); 0),
            };
        }

        // 1^exp mod m = 1 for any exp when m > 1 (when m == 1 the result is 0)
        if self.digits.len() == 1
            && self.digits[0] == 1
            && !(modulus.digits.len() == 1 && modulus.digits[0] == 1)
        {
            return Self {
                digits: vec_in!(allocator.clone(); 1),
            };
        }

        if exp.len() <= core::mem::size_of::<usize>() {
            let exp_as_number = {
                let mut tmp: usize = 0;
                for d in exp {
                    tmp *= 256;
                    tmp += (*d) as usize;
                }
                tmp
            };

            if let Some(max_output_digits) = self.digits.len().checked_mul(exp_as_number) {
                if modulus.digits.len() > max_output_digits {
                    // Special case: modulus is larger than `base ^ exp`, so division is not relevant
                    let mut scratch_space = vec_in!(allocator.clone(); 0; max_output_digits);
                    return big_wrapping_pow(self, exp, &mut scratch_space, allocator.clone());
                }
            }
        }

        if modulus.is_power_of_two() {
            return self.modpow_with_power_of_two(exp, modulus, allocator.clone());
        } else if modulus.is_odd() {
            return self.modpow_montgomery(exp, modulus, allocator);
        }

        // If the modulus is not a power of two and not an odd number then
        // it is a product of some power of two with an odd number. In this
        // case we will use the Chinese remainder theorem to get the result.
        // See http://www.people.vcu.edu/~jwang3/CMSC691/j34monex.pdf

        let trailing_zeros = modulus.digits.iter().take_while(|x| x == &&0).count();
        let additional_zero_bits = modulus.digits[trailing_zeros].trailing_zeros() as usize;
        let power_of_two = {
            let mut tmp = MPNat {
                digits: vec_in!(allocator.clone(); 0; trailing_zeros + 1),
            };
            tmp.digits[trailing_zeros] = 1 << additional_zero_bits;
            tmp
        };
        let power_of_two_mask = *power_of_two.digits.last().unwrap() - 1;
        let odd = {
            let num_digits = modulus.digits.len() - trailing_zeros;
            let mut tmp = MPNat {
                digits: vec_in!(allocator.clone(); 0; num_digits),
            };
            if additional_zero_bits > 0 {
                tmp.digits[0] = modulus.digits[trailing_zeros] >> additional_zero_bits;
                for i in 1..num_digits {
                    let d = modulus.digits[trailing_zeros + i];
                    tmp.digits[i - 1] +=
                        (d & power_of_two_mask) << (WORD_BITS - additional_zero_bits);
                    tmp.digits[i] = d >> additional_zero_bits;
                }
            } else {
                tmp.digits
                    .copy_from_slice(&modulus.digits[trailing_zeros..]);
            }
            while tmp.digits.last() == Some(&0) {
                tmp.digits.pop();
            }
            tmp
        };
        debug_assert!(power_of_two.is_power_of_two(), "Factored out power of two");
        debug_assert!(
            odd.is_odd(),
            "Remaining number is odd after factoring out powers of two"
        );
        debug_assert!(
            {
                let mut tmp = vec_in!(allocator.clone(); 0; modulus.digits.len());
                big_wrapping_mul(&power_of_two, &odd, &mut tmp);
                tmp == modulus.digits
            },
            "modulus is factored"
        );

        let mut base_copy = MPNat {
            digits: self.digits.clone(),
        };
        let x1 = base_copy.modpow_montgomery(exp, &odd, allocator.clone());
        let x2 = self.modpow_with_power_of_two(exp, &power_of_two, allocator.clone());

        let odd_inv = Self::koc_2017_inverse(
            &odd,
            trailing_zeros * WORD_BITS + additional_zero_bits,
            allocator.clone(),
        );

        let s = power_of_two.digits.len();
        let mut scratch = vec_in!(allocator.clone(); 0; s);
        let diff = {
            scratch.fill(0);
            let mut b = false;
            for (i, scratch_digit) in scratch.iter_mut().enumerate().take(s) {
                let (diff, borrow) = borrowing_sub(
                    x2.digits.get(i).copied().unwrap_or(0),
                    x1.digits.get(i).copied().unwrap_or(0),
                    b,
                );
                *scratch_digit = diff;
                b = borrow;
            }
            MPNat { digits: scratch }
        };
        let y = {
            let mut out = vec_in!(allocator.clone(); 0; s);
            big_wrapping_mul(&diff, &odd_inv, &mut out);
            *out.last_mut().unwrap() &= power_of_two_mask;
            MPNat { digits: out }
        };

        // Re-use allocation for efficiency
        let mut digits = diff.digits;
        let s = modulus.digits.len();
        digits.fill(0);
        // `resize` without `grow`
        if digits.capacity() < s {
            let mut new_digits = Vec::with_capacity_in(s, digits.allocator().clone());
            new_digits.resize(s, 0);
            new_digits[..digits.len()].copy_from_slice(digits.as_slice());
            digits = new_digits;
        } else {
            digits.resize(s, 0);
        }
        big_wrapping_mul(&odd, &y, &mut digits);
        let mut c = false;
        for (i, out_digit) in digits.iter_mut().enumerate() {
            let (sum, carry) = carrying_add(x1.digits.get(i).copied().unwrap_or(0), *out_digit, c);
            c = carry;
            *out_digit = sum;
        }
        MPNat { digits }
    }

    // Computes `self ^ exp mod modulus` using Montgomery multiplication.
    // See https://www.microsoft.com/en-us/research/wp-content/uploads/1996/01/j37acmon.pdf
    fn modpow_montgomery(&mut self, exp: &[u8], modulus: &Self, allocator: A) -> Self {
        // The montgomery method only works with odd modulus.
        debug_assert!(modulus.is_odd());

        // n_prime satisfies `r * (r^(-1)) - modulus * n' = 1`, where
        // `r = 2^(WORD_BITS*modulus.digits.len())`.
        let n_prime = Word::MAX - mod_inv(modulus.digits[0]) + 1;
        let s = modulus.digits.len();

        let mut x_bar = MPNat {
            digits: vec_in!(allocator.clone(); 0; s),
        };
        // Initialize result as `r mod modulus` (Montgomery form of 1)
        compute_r_mod_n(modulus, &mut x_bar.digits);

        // Reduce base mod modulus
        self.sub_to_same_size(modulus);

        // Need to compute a_bar = base * r mod modulus;
        // First directly multiply base * r to get a 2s-digit number,
        // then reduce mod modulus.
        let a_bar = {
            let mut tmp = MPNat {
                digits: vec_in!(allocator.clone(); 0; 2 * s),
            };
            big_wrapping_mul(self, &x_bar, &mut tmp.digits);
            tmp.sub_to_same_size(modulus);
            tmp
        };

        // scratch space for monpro algorithm
        let mut scratch = vec_in!(allocator.clone(); 0; 2 * s + 1);
        let monpro_len = s + 2;

        // Use binary method for computing exp, but with monpro as the multiplication
        for &b in exp {
            let mut mask: u8 = 1 << 7;
            while mask > 0 {
                monsq(&x_bar, modulus, n_prime, &mut scratch);
                x_bar.digits.copy_from_slice(&scratch[0..s]);
                scratch.fill(0);
                if b & mask != 0 {
                    monpro(
                        &x_bar,
                        &a_bar,
                        modulus,
                        n_prime,
                        &mut scratch[0..monpro_len],
                    );
                    x_bar.digits.copy_from_slice(&scratch[0..s]);
                    scratch.fill(0);
                }
                mask >>= 1;
            }
        }

        // Convert out of Montgomery form by computing monpro with 1
        let one = {
            // We'll reuse the memory space from a_bar for efficiency.
            let mut digits = a_bar.digits;
            digits.fill(0);
            digits[0] = 1;
            MPNat { digits }
        };
        monpro(&x_bar, &one, modulus, n_prime, &mut scratch[0..monpro_len]);
        scratch.resize(s, 0);
        MPNat { digits: scratch }
    }

    fn modpow_with_power_of_two(&mut self, exp: &[u8], modulus: &Self, allocator: A) -> Self {
        debug_assert!(modulus.is_power_of_two());
        // We know `modulus` is a power of 2. So reducing is as easy as bit shifting.
        // We also know the modulus is non-zero because 0 is not a power of 2.

        // First reduce self to be the same size as the modulus
        self.force_same_size(modulus);
        // The modulus is a power of 2 but that power may not be a multiple of a whole word.
        // We can clear out any higher order bits to fix this.
        let modulus_mask = *modulus.digits.last().unwrap() - 1;
        *self.digits.last_mut().unwrap() &= modulus_mask;

        // We know that `totient(2^k) = 2^(k-1)`, therefore by Euler's theorem
        // we can also reduce the exponent mod `2^(k-1)`. Effectively this means
        // throwing away bytes to make `exp` shorter. Note: Euler's theorem only applies
        // if the base and modulus are coprime (which in this case means the base is odd).
        let exp = if self.is_odd() && (exp.len() > WORD_BYTES * modulus.digits.len()) {
            &exp[(exp.len() - WORD_BYTES * modulus.digits.len())..]
        } else {
            exp
        };

        let mut scratch_space = vec_in!(allocator.clone(); 0; modulus.digits.len());
        let mut result = big_wrapping_pow(self, exp, &mut scratch_space, allocator.clone());

        // The modulus is a power of 2 but that power may not be a multiple of a whole word.
        // We can clear out any higher order bits to fix this.
        *result.digits.last_mut().unwrap() &= modulus_mask;

        result
    }

    /// Makes `self` have the same number of digits as `other` by
    /// pushing 0s or dropping higher order digits as needed.
    /// This is equivalent to reducing `self` modulo `2^(WORD_BITS*k)` where
    /// `k` is the number of digits in `other`.
    fn force_same_size(&mut self, other: &Self) {
        // `resize` without `grow`
        if self.digits.capacity() < other.digits.len() {
            let mut new_digits =
                Vec::with_capacity_in(other.digits.len(), self.digits.allocator().clone());
            new_digits.resize(other.digits.len(), 0);
            new_digits[..self.digits.len()].copy_from_slice(self.digits.as_slice());
            self.digits = new_digits;
        } else {
            self.digits.resize(other.digits.len(), 0);
        }

        // This is here to just drive the point home about what the
        // invariant is after calling this function.
        debug_assert_eq!(self.digits.len(), other.digits.len());
    }

    /// Assumes `self` has more digits than `other`.
    /// Makes `self` have the same number of digits as `other` by subtracting off multiples
    /// of `other`. This is a partial reduction of `self` modulo `other`, but rather
    /// than doing the full division, the goal is simply to make the two numbers have the
    /// same number of digits.
    fn sub_to_same_size(&mut self, other: &Self) {
        // Remove leading zeros before starting
        while self.digits.len() > 1 && self.digits.last() == Some(&0) {
            self.digits.pop();
        }

        let n = other.digits.len();
        let m = self.digits.len().saturating_sub(n);
        if m == 0 {
            return;
        }

        let other_most_sig = *other.digits.last().unwrap() as DoubleWord;

        if self.digits.len() == 2 {
            // This is the smallest case since `n >= 1` and `m > 0`
            // implies that `self.digits.len() >= 2`.
            // In this case we can use DoubleWord-sized arithmetic
            // to get the answer directly.
            let self_most_sig = self.digits.pop().unwrap();
            let a = join_as_double(self_most_sig, self.digits[0]);
            let b = other_most_sig;
            self.digits[0] = (a % b) as Word;
            return;
        }

        if n == 1 {
            // The divisor is only 1 digit, so the long-division
            // algorithm is easy.
            let k = self.digits.len() - 1;
            for j in (0..k).rev() {
                let self_most_sig = self.digits.pop().unwrap();
                let self_second_sig = self.digits[j];
                let r = join_as_double(self_most_sig, self_second_sig) % other_most_sig;
                self.digits[j] = r as Word;
            }
            return;
        }

        // At this stage we know that `n >= 2` and `self.digits.len() >= 3`.
        // The smaller cases are covered in the if-statements above.

        // The algorithm below only works well when the divisor's
        // most significant digit is at least `BASE / 2`.
        // If it is too small then we "normalize" by multiplying
        // both numerator and denominator by a common factor
        // and run the algorithm on those numbers.
        // See Knuth The Art of Computer Programming vol. 2 section 4.3 for details.
        let shift = (other_most_sig as Word).leading_zeros();
        if shift > 0 {
            // Normalize self
            let overflow = in_place_shl(&mut self.digits, shift);
            // `push` without `grow`
            if self.digits.capacity() == self.digits.len() {
                let mut new_digits =
                    Vec::with_capacity_in(self.digits.len() + 1, self.digits.allocator().clone());
                new_digits.append(&mut self.digits);
                self.digits = new_digits;
            }
            self.digits.push(overflow);

            // Normalize other
            let mut normalized = other.digits.clone();
            let overflow = in_place_shl(&mut normalized, shift);
            debug_assert_eq!(overflow, 0, "Normalizing modulus cannot overflow");
            debug_assert_eq!(
                normalized[n - 1].leading_zeros(),
                0,
                "Most significant bit is set"
            );

            // Run algorithm on normalized values
            self.sub_to_same_size(&MPNat { digits: normalized });

            // need to de-normalize to get the correct result
            in_place_shr(&mut self.digits, shift);

            return;
        }

        let other_second_sig = other.digits[n - 2] as DoubleWord;
        let mut self_most_sig: Word = 0;
        for j in (0..=m).rev() {
            let self_second_sig = *self.digits.last().unwrap();
            let self_third_sig = self.digits[self.digits.len() - 2];

            let a = join_as_double(self_most_sig, self_second_sig);
            let mut q_hat = a / other_most_sig;
            let mut r_hat = a % other_most_sig;

            loop {
                let a = q_hat * other_second_sig;
                let b = join_as_double(r_hat as Word, self_third_sig);
                if q_hat >= BASE || a > b {
                    q_hat -= 1;
                    r_hat += other_most_sig;
                    if BASE <= r_hat {
                        break;
                    }
                } else {
                    break;
                }
            }

            let mut borrow = in_place_mul_sub(&mut self.digits[j..], &other.digits, q_hat as Word);
            if borrow > self_most_sig {
                // q_hat was too large, add back one multiple of the modulus
                let carry = in_place_add(&mut self.digits[j..], &other.digits);
                debug_assert!(
                    carry,
                    "Adding back should cause overflow to cancel the borrow"
                );
                borrow -= 1;
            }
            // Most significant digit of self has been cancelled out
            debug_assert_eq!(borrow, self_most_sig);
            self_most_sig = self.digits.pop().unwrap();
        }

        // `push` without `grow`
        if self.digits.capacity() == self.digits.len() {
            let mut new_digits =
                Vec::with_capacity_in(self.digits.len() + 1, self.digits.allocator().clone());
            new_digits.append(&mut self.digits);
            self.digits = new_digits;
        }
        self.digits.push(self_most_sig);
        debug_assert!(self.digits.len() <= n);
    }

    pub fn to_big_endian(&self, allocator: A) -> Vec<u8, A> {
        if self.digits.iter().all(|x| x == &0) {
            return vec_in!(allocator.clone(); 0);
        }

        // Safety: unwrap is safe since `self.digits` is always non-empty.
        let most_sig_bytes: [u8; WORD_BYTES] = self.digits.last().unwrap().to_be_bytes();
        // The most significant digit may not need 4 bytes.
        // Only include as many bytes as needed in the output.
        let be_initial_bytes = {
            let mut tmp: &[u8] = &most_sig_bytes;
            while !tmp.is_empty() && tmp[0] == 0 {
                tmp = &tmp[1..];
            }
            tmp
        };

        let mut result = vec_in!(allocator.clone(); 0u8; (self.digits.len() - 1) * WORD_BYTES + be_initial_bytes.len());
        result[0..be_initial_bytes.len()].copy_from_slice(be_initial_bytes);
        for (i, d) in self.digits.iter().take(self.digits.len() - 1).enumerate() {
            let bytes = d.to_be_bytes();
            let j = result.len() - WORD_BYTES * i;
            result[(j - WORD_BYTES)..j].copy_from_slice(&bytes);
        }
        result
    }
}

#[cfg(test)]
mod test {
    use crate::modexp::hex;
    use crate::modexp::software::arith::mp_nat_to_u128;
    use crate::modexp::software::mpnat::MPNat;
    extern crate std;
    use num_bigint::BigUint;
    use std::alloc::Global;

    fn mp(n: u128) -> MPNat<Global> {
        MPNat::from_big_endian(&n.to_be_bytes(), Global)
    }

    #[test]
    fn test_modpow_even() {
        // (base, exp, modulus, expected): moduli with a sub-word power of two, with a zero low
        // word, and with both
        for (base, exp, modulus, expected) in [
            (3, 5, 20, 3),
            (
                0x2ff4f4df4c518867207c84b57a77aa50,
                0xca83c2925d17c577c9a03598b6f360,
                0xf863d4f17a5405d84814f54c92f803c8,
                0x8d216c9a1fb275ed18eb340ed43cacc0,
            ),
            (
                0x9c12fe4a1a97d17c1e4573247a43b0e5,
                0x466f3e0a2e8846b8c48ecbf612b96412,
                0x710d7b9d5718acff0000000000000000,
                0x569bf65929e71cd10a553a8623bdfc99,
            ),
            (
                0x6d018fdeaa408222cb10ff2c36124dcf,
                0x8e35fc05d490bb138f73c2bc284a67a7,
                0x6c237160750d78400000000000000000,
                0x3fe14e11392c6c6be8efe956c965d5af,
            ),
        ] {
            let result = mp(base).modpow(&u128::to_be_bytes(exp), &mp(modulus), Global);
            assert_eq!(mp_nat_to_u128(&result), expected);
        }

        let modexp = |base: &str, exp: &str, modulus: &str| {
            let [base, exp, modulus] =
                [base, exp, modulus].map(|digits| hex::decode(digits).unwrap());
            hex::encode(crate::modexp::software::modexp(
                &base, &exp, &modulus, Global,
            ))
        };
        assert_eq!(
            modexp(
                "36abd4524e89a34c89c420942547e12c7be1",
                "01000000000517ea78",
                "02f0758c6a04200955b649c35722b800000000"
            ),
            "023f4f762936eb0973d46b6eadb59d68d06101"
        );
        assert_eq!(modexp("00", "", "02"), "01");
    }

    #[test]
    fn test_modpow_montgomery() {
        for (base, exp, modulus, expected) in [
            (3, 5, 19, 15),
            (3, 5, 0x9346_9d50_1f74_d1c1, 243),
            (
                0x5c4b74ec760dfb021499f5c5e3c69222,
                0x62b2a34b21cf4cc036e880b3fb59fe09,
                0x7b799c4502cd69bde8bb12601ce3ff15,
                0x10c9d9071d0b86d6a59264d2f461200,
            ),
        ] {
            let result = mp(base).modpow_montgomery(&u128::to_be_bytes(exp), &mp(modulus), Global);
            assert_eq!(
                mp_nat_to_u128(&result),
                expected,
                "{base}^{exp} % {modulus}"
            );
        }
    }

    #[test]
    fn test_modpow_with_power_of_two() {
        // an exponent reduced for an odd base, a base longer than the modulus, two words
        for (base, exp, modulus, expected) in [
            (3, 1_000_000, 1 << 30, 641836289),
            (
                0xabcd_ef01_2345_6789_1111,
                0x1234_5678_90ab_cdef,
                1 << 5,
                17,
            ),
            (
                0x3f47_9dc0_d5b9_6003,
                0xa180_e045_e314_8581,
                1 << 118,
                0x0028_3d19_e6cc_b8a0_e050_6abb_b9b1_1a03,
            ),
        ] {
            let result =
                mp(base).modpow_with_power_of_two(&u128::to_be_bytes(exp), &mp(modulus), Global);
            assert_eq!(mp_nat_to_u128(&result), expected);
        }
    }

    #[test]
    fn test_sub_to_same_size() {
        fn check(a: &[u8], n: &[u8]) {
            let mut x = MPNat::from_big_endian(a, Global);
            let y = MPNat::from_big_endian(n, Global);
            x.sub_to_same_size(&y);
            assert!(x.digits.len() <= y.digits.len());
            let x = BigUint::from_bytes_be(&x.to_big_endian(Global));
            let (a, n) = (BigUint::from_bytes_be(a), BigUint::from_bytes_be(n));
            assert_eq!(x % &n, &a % &n, "{a} % {n}");
        }

        check(
            &0x10_00_00_00_00u128.to_be_bytes(),
            &0xFF_00_00_00u128.to_be_bytes(),
        );
        let n = 10u128;
        let a = 57 + 2 * n + 0x1234_0000_0000 * n + 0x000b_0000_0000_0000_0000 * n;
        check(&a.to_be_bytes(), &n.to_be_bytes());
        // the final borrow equals the top digit; the q_hat/r_hat adjustment
        for (a, n) in [
            (
                "0000000000000000ae5ff08bfc0271a4fee04902c9d912618ef5022ca0000000",
                "ae5ff08bfc0271a4fee0490f70000000",
            ),
            (
                "0000000000000000000000ffffffff0000000001000000000000000000000000",
                "ffffffff00000000ffffffffff000000",
            ),
        ] {
            check(&hex::decode(a).unwrap(), &hex::decode(n).unwrap());
        }
    }

    #[test]
    fn test_mp_nat_is_odd_and_is_power_of_two() {
        for n in [0, 1, 2, 1025, 0xFF_FF_FF_FF_00_00_04_00, (1 << 64) + 1] {
            assert_eq!(mp(n).is_odd(), n % 2 == 1, "{n}");
        }
        for (n, expected) in [
            (0, false),
            (1, true),
            (1327, false),
            ((1 << 1) + (1 << 35), false),
            ((1 << 1) + (1 << 65), false),
            (1 << 5, true),
            (1 << 64, true),
            (1 << 127, true),
        ] {
            assert_eq!(mp(n).is_power_of_two(), expected, "{n}");
        }
    }

    #[test]
    fn test_mp_nat_be() {
        for hex_input in [
            "",
            "00",
            "77",
            "00000000abcd",
            "abcdef0011223344",
            "abcdef001122334455",
            "abcdef01234567891011121314151617181920",
        ] {
            let mp = MPNat::from_big_endian(&hex::decode(hex_input).unwrap(), Global);
            let trimmed = match hex_input.trim_start_matches('0') {
                "" => "00",
                x => x,
            };
            assert_eq!(hex::encode(mp.to_big_endian(Global)), trimmed);
        }
    }
}
