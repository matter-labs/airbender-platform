use super::mpnat::{DoubleWord, MPNat, Word, BASE, WORD_BITS};
use super::vec_in;
use alloc::vec::Vec;
use core::alloc::Allocator;

// Computes the "Montgomery Product" of two numbers.
// See Coarsely Integrated Operand Scanning (CIOS) Method in
// https://www.microsoft.com/en-us/research/wp-content/uploads/1996/01/j37acmon.pdf
// In short, computes `xy (r^-1) mod n`, where `r = 2^(8*4*s)` and `s` is the number of
// digits needs to represent `n`. `n_prime` has the property that `r(r^(-1)) - nn' = 1`.
// Note: This algorithm only works if `xy < rn` (generally we will either have both `x < n`, `y < n`
// or we will have `x < r`, `y < n`).
pub fn monpro<A: Allocator + Clone>(
    x: &MPNat<A>,
    y: &MPNat<A>,
    n: &MPNat<A>,
    n_prime: Word,
    out: &mut [Word],
) {
    debug_assert!(
        n.is_odd(),
        "Montgomery multiplication only makes sense with odd modulus"
    );
    debug_assert!(
        out.len() >= n.digits.len() + 2,
        "Output needs 2 extra words over the size needed to represent n"
    );
    let s = out.len() - 2;
    // Using a range loop as opposed to `out.iter_mut().enumerate().take(s)`
    // does make a meaningful performance difference in this case.
    #[allow(clippy::needless_range_loop)]
    for i in 0..s {
        let mut c = 0;
        for j in 0..s {
            let (prod, carry) = shifted_carrying_mul(
                out[j],
                x.digits.get(j).copied().unwrap_or(0),
                y.digits.get(i).copied().unwrap_or(0),
                c,
            );
            out[j] = prod;
            c = carry;
        }
        let (sum, carry) = carrying_add(out[s], c, false);
        out[s] = sum;
        out[s + 1] = carry as Word;
        let m = out[0].wrapping_mul(n_prime);
        let (_, carry) = shifted_carrying_mul(out[0], m, n.digits.first().copied().unwrap_or(0), 0);
        c = carry;
        for j in 1..s {
            let (prod, carry) =
                shifted_carrying_mul(out[j], m, n.digits.get(j).copied().unwrap_or(0), c);
            out[j - 1] = prod;
            c = carry;
        }
        let (sum, carry) = carrying_add(out[s], c, false);
        out[s - 1] = sum;
        out[s] = out[s + 1] + (carry as Word); // overflow impossible at this stage
    }
    // Result is only in the first s + 1 words of the output.
    out[s + 1] = 0;

    // Check if we need to do the final subtraction
    for i in (0..=s).rev() {
        match out[i].cmp(n.digits.get(i).unwrap_or(&0)) {
            core::cmp::Ordering::Less => return, // No subtraction needed
            core::cmp::Ordering::Greater => break,
            core::cmp::Ordering::Equal => (),
        }
    }

    let mut b = false;
    for (i, out_digit) in out.iter_mut().enumerate().take(s) {
        let (diff, borrow) = borrowing_sub(*out_digit, n.digits.get(i).copied().unwrap_or(0), b);
        *out_digit = diff;
        b = borrow;
    }
    let (diff, borrow) = borrowing_sub(out[s], 0, b);
    out[s] = diff;

    debug_assert!(!borrow, "No borrow needed since out < n");
}

// Equivalent to `monpro(x, x, n, n_prime, out)`, but more efficient.
pub fn monsq<A: Allocator + Clone>(x: &MPNat<A>, n: &MPNat<A>, n_prime: Word, out: &mut [Word]) {
    debug_assert!(
        n.is_odd(),
        "Montgomery multiplication only makes sense with odd modulus"
    );
    debug_assert!(
        x.digits.len() <= n.digits.len(),
        "x cannot be larger than n"
    );
    debug_assert!(
        out.len() > 2 * n.digits.len(),
        "Output needs double the digits to hold the value of x^2 plus an extra word"
    );
    let s = n.digits.len();

    big_sq(x, out);
    for i in 0..s {
        let mut c: Word = 0;
        let m = out[i].wrapping_mul(n_prime);
        for j in 0..s {
            let (prod, carry) =
                shifted_carrying_mul(out[i + j], m, n.digits.get(j).copied().unwrap_or(0), c);
            out[i + j] = prod;
            c = carry;
        }
        let mut j = i + s;
        while c > 0 {
            let (sum, carry) = carrying_add(out[j], c, false);
            out[j] = sum;
            c = carry as Word;
            j += 1;
        }
    }
    // Only keep the last `s + 1` digits in `out`.
    for i in 0..(s + 1) {
        out[i] = out[i + s];
    }
    out[(s + 1)..].fill(0);

    // Check if we need to do the final subtraction
    for i in (0..=s).rev() {
        match out[i].cmp(n.digits.get(i).unwrap_or(&0)) {
            core::cmp::Ordering::Less => return,
            core::cmp::Ordering::Greater => break,
            core::cmp::Ordering::Equal => (),
        }
    }

    let mut b = false;
    for (i, out_digit) in out.iter_mut().enumerate().take(s) {
        let (diff, borrow) = borrowing_sub(*out_digit, n.digits.get(i).copied().unwrap_or(0), b);
        *out_digit = diff;
        b = borrow;
    }
    let (diff, borrow) = borrowing_sub(out[s], 0, b);
    out[s] = diff;

    debug_assert!(!borrow, "No borrow needed since out < n");
}

// Given x odd, computes `x^(-1) mod 2^32`.
// See `MODULAR-INVERSE` in https://link.springer.com/content/pdf/10.1007/3-540-46877-3_21.pdf
pub fn mod_inv(x: Word) -> Word {
    debug_assert_eq!(x & 1, 1, "Algorithm only valid for odd n");

    let mut y = 1;
    for i in 2..WORD_BITS {
        let mask = (1 << i) - 1;
        let xy = x.wrapping_mul(y) & mask;
        let q = 1 << (i - 1);
        if xy >= q {
            y += q;
        }
    }
    let xy = x.wrapping_mul(y);
    let q = 1 << (WORD_BITS - 1);
    if xy >= q {
        y += q;
    }
    y
}

/// Computes R mod n, where R = 2^(WORD_BITS*k) and k = n.digits.len()
/// Note that if R = qn + r, q must be smaller than 2^WORD_BITS since `2^(WORD_BITS) * n > R`
/// (adding a whole additional word to n is too much).
/// Uses the two most significant digits of n to approximate the quotient,
/// then computes the difference to get the remainder. It is possible that this
/// quotient is too big by 1; we can catch that case by looking for overflow
/// in the subtraction.
pub fn compute_r_mod_n<A: Allocator + Clone>(n: &MPNat<A>, out: &mut [Word]) {
    let k = n.digits.len();

    if k == 1 {
        let r = BASE;
        let result = r % (n.digits[0] as DoubleWord);
        out[0] = result as Word;
        return;
    }

    debug_assert!(n.is_odd(), "This algorithm only works for odd numbers");
    debug_assert!(
        out.len() >= k,
        "Output must be able to hold numbers of the same size as n"
    );

    let approx_n = join_as_double(n.digits[k - 1], n.digits[k - 2]);
    let approx_q = DoubleWord::MAX / approx_n;
    debug_assert!(
        approx_q <= (Word::MAX as DoubleWord),
        "quotient must fit in a single digit"
    );
    let mut approx_q = approx_q as Word;

    loop {
        let mut c = 0;
        let mut b = false;
        for (n_digit, out_digit) in n.digits.iter().zip(out.iter_mut()) {
            let (prod, carry) = carrying_mul(approx_q, *n_digit, c);
            c = carry;
            let (diff, borrow) = borrowing_sub(0, prod, b);
            b = borrow;
            *out_digit = diff;
        }
        let (diff, borrow) = borrowing_sub(1, c, b);
        if borrow {
            // approx_q was too large so `R - approx_q*n` overflowed.
            // try again with approx_q -= 1
            approx_q -= 1;
        } else {
            debug_assert_eq!(
                diff, 0,
                "R - qn must be smaller than n, hence fit in k digits"
            );
            break;
        }
    }
}

/// Computes `base ^ exp`, ignoring overflow.
pub fn big_wrapping_pow<A: Allocator + Clone>(
    base: &MPNat<A>,
    exp: &[u8],
    scratch_space: &mut [Word],
    allocator: A,
) -> MPNat<A> {
    // Compute result via the "binary method", see Knuth The Art of Computer Programming
    let mut result = MPNat {
        digits: vec_in!(allocator.clone(); 0; scratch_space.len()),
    };
    result.digits[0] = 1;
    for &b in exp {
        let mut mask: u8 = 1 << 7;
        while mask > 0 {
            big_wrapping_mul(&result, &result, scratch_space);
            result.digits.copy_from_slice(scratch_space);
            scratch_space.fill(0); // zero-out the scratch space
            if b & mask != 0 {
                big_wrapping_mul(&result, base, scratch_space);
                result.digits.copy_from_slice(scratch_space);
                scratch_space.fill(0); // zero-out the scratch space
            }
            mask >>= 1;
        }
    }
    result
}

/// Computes `(x * y) mod 2^(WORD_BITS*out.len())`.
pub fn big_wrapping_mul<A: Allocator + Clone>(x: &MPNat<A>, y: &MPNat<A>, out: &mut [Word]) {
    let s = out.len();
    for i in 0..s {
        let mut c: Word = 0;
        for j in 0..(s - i) {
            let (prod, carry) = shifted_carrying_mul(
                out[i + j],
                x.digits.get(j).copied().unwrap_or(0),
                y.digits.get(i).copied().unwrap_or(0),
                c,
            );
            c = carry;
            out[i + j] = prod;
        }
    }
}

/// Computes `x^2`, storing the result in `out`.
pub fn big_sq<A: Allocator + Clone>(x: &MPNat<A>, out: &mut [Word]) {
    debug_assert!(
        out.len() > 2 * x.digits.len(),
        "Output needs double the digits to hold the value of x^2"
    );
    let s = x.digits.len();
    for i in 0..s {
        let (product, carry) = shifted_carrying_mul(out[i + i], x.digits[i], x.digits[i], 0);
        out[i + i] = product;
        let mut c = carry as DoubleWord;
        for j in (i + 1)..s {
            let mut new_c: DoubleWord = 0;
            let res = (x.digits[i] as DoubleWord) * (x.digits[j] as DoubleWord);
            let (res, overflow) = res.overflowing_add(res);
            if overflow {
                new_c += BASE;
            }
            let (res, overflow) = (out[i + j] as DoubleWord).overflowing_add(res);
            if overflow {
                new_c += BASE;
            }
            let (res, overflow) = res.overflowing_add(c);
            if overflow {
                new_c += BASE;
            }
            out[i + j] = res as Word;
            c = new_c + ((res >> WORD_BITS) as DoubleWord);
        }
        let (sum, carry) = carrying_add(out[i + s], c as Word, false);
        out[i + s] = sum;
        out[i + s + 1] = ((c >> WORD_BITS) as Word) + (carry as Word);
    }
}

// Performs `a <<= shift`, returning the overflow
pub fn in_place_shl(a: &mut [Word], shift: u32) -> Word {
    let mut c: Word = 0;
    let carry_shift = (WORD_BITS as u32) - shift;
    for a_digit in a.iter_mut() {
        let carry = a_digit.overflowing_shr(carry_shift).0;
        *a_digit = a_digit.overflowing_shl(shift).0 | c;
        c = carry;
    }
    c
}

// Performs `a >>= shift`, returning the overflow
pub fn in_place_shr(a: &mut [Word], shift: u32) -> Word {
    let mut b: Word = 0;
    let borrow_shift = (WORD_BITS as u32) - shift;
    for a_digit in a.iter_mut().rev() {
        let borrow = a_digit.overflowing_shl(borrow_shift).0;
        *a_digit = a_digit.overflowing_shr(shift).0 | b;
        b = borrow;
    }
    b
}

// Performs a += b, returning if there was overflow
pub fn in_place_add(a: &mut [Word], b: &[Word]) -> bool {
    debug_assert!(a.len() == b.len());

    let mut c = false;
    for (a_digit, b_digit) in a.iter_mut().zip(b) {
        let (sum, carry) = carrying_add(*a_digit, *b_digit, c);
        *a_digit = sum;
        c = carry;
    }

    c
}

// Performs `a -= xy`, returning the "borrow".
pub fn in_place_mul_sub(a: &mut [Word], x: &[Word], y: Word) -> Word {
    debug_assert!(a.len() == x.len());

    // a -= x*0 leaves an unchanged, so return early
    if y == 0 {
        return 0;
    }

    // carry is between -big_digit::MAX and 0, so to avoid overflow we store
    // offset_carry = carry + big_digit::MAX
    let mut offset_carry = Word::MAX;

    for (a_digit, x_digit) in a.iter_mut().zip(x) {
        // We want to calculate sum = x - y * c + carry.
        // sum >= -(big_digit::MAX * big_digit::MAX) - big_digit::MAX
        // sum <= big_digit::MAX
        // Offsetting sum by (big_digit::MAX << big_digit::BITS) puts it in DoubleBigDigit range.
        let offset_sum = join_as_double(Word::MAX, *a_digit) - Word::MAX as DoubleWord
            + offset_carry as DoubleWord
            - ((*x_digit as DoubleWord) * (y as DoubleWord));

        let new_offset_carry = (offset_sum >> WORD_BITS) as Word;
        let new_x = offset_sum as Word;
        offset_carry = new_offset_carry;
        *a_digit = new_x;
    }

    // Return the borrow.
    Word::MAX - offset_carry
}

/// Computes `a + xy + c` where any overflow is captured as the "carry",
/// the second part of the output. The arithmetic in this function is
/// guaranteed to never overflow because even when all 4 variables are
/// equal to `Word::MAX` the output is smaller than `DoubleWord::MAX`.
pub fn shifted_carrying_mul(a: Word, x: Word, y: Word, c: Word) -> (Word, Word) {
    let wide = { (a as DoubleWord) + ((x as DoubleWord) * (y as DoubleWord)) + (c as DoubleWord) };
    (wide as Word, (wide >> WORD_BITS) as Word)
}

/// Computes `xy + c` where any overflow is captured as the "carry",
/// the second part of the output. The arithmetic in this function is
/// guaranteed to never overflow because even when all 3 variables are
/// equal to `Word::MAX` the output is smaller than `DoubleWord::MAX`.
pub fn carrying_mul(x: Word, y: Word, c: Word) -> (Word, Word) {
    let wide = { ((x as DoubleWord) * (y as DoubleWord)) + (c as DoubleWord) };
    (wide as Word, (wide >> WORD_BITS) as Word)
}

// Computes `x + y` with "carry the 1" semantics
pub fn carrying_add(x: Word, y: Word, carry: bool) -> (Word, bool) {
    let (a, b) = x.overflowing_add(y);
    let (c, d) = a.overflowing_add(carry as Word);
    (c, b | d)
}

// Computes `x - y` with "borrow from your neighbour" semantics
pub fn borrowing_sub(x: Word, y: Word, borrow: bool) -> (Word, bool) {
    let (a, b) = x.overflowing_sub(y);
    let (c, d) = a.overflowing_sub(borrow as Word);
    (c, b | d)
}

pub fn join_as_double(hi: Word, lo: Word) -> DoubleWord {
    DoubleWord::from(lo) | (DoubleWord::from(hi) << WORD_BITS)
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
pub fn mp_nat_to_u128(x: &MPNat<std::alloc::Global>) -> u128 {
    use std::alloc::Global;

    let mut buf = [0u8; 16];
    let result = x.to_big_endian(Global);
    let k = result.len();
    buf[(16 - k)..].copy_from_slice(&result);
    u128::from_be_bytes(buf)
}

#[cfg(test)]
mod test {
    use super::*;
    use num_bigint::BigUint;
    use std::alloc::Global;
    extern crate std;
    use std::vec;

    fn mp(n: u128) -> MPNat<Global> {
        MPNat::from_big_endian(&n.to_be_bytes(), Global)
    }

    fn big(x: &MPNat<Global>) -> BigUint {
        BigUint::from_bytes_be(&x.to_big_endian(Global))
    }

    fn n_prime(m: &MPNat<Global>) -> Word {
        Word::MAX - mod_inv(m.digits[0]) + 1
    }

    #[test]
    fn test_monpro_and_monsq() {
        use num_bigint::BigInt;
        use num_integer::Integer;

        // 5 * 7 = 0 mod 35 and 5 * 5 = 0 mod 25 need the final subtraction
        for (x, y, n) in [
            (6, 7, 31),
            (5, 7, 35),
            (5, 5, 25),
            (
                0x16FF_221F_CB7D,
                0x0C75_8535_434F,
                0x011E_842B_6BAA_5017_EBF2_8293,
            ),
            (
                0xa6b0ce71a380dea7c83435bc,
                0xa6b0ce71a380dea7c83435bc,
                0xc4550871a1cfc67af3e77eceb2ecfce5,
            ),
        ] {
            let m = mp(n);
            // x * y * r^-1 mod n
            let r = BigInt::from(2).pow((WORD_BITS * m.digits.len()) as u32);
            let n_big = BigInt::from(n);
            let r_inv = r.extended_gcd(&n_big).x;
            let expected =
                |x: u128, y: u128| (BigInt::from(x) * BigInt::from(y) * &r_inv).mod_floor(&n_big);

            let mut out = vec![0; m.digits.len() + 2];
            monpro(&mp(x), &mp(y), &m, n_prime(&m), &mut out);
            let result = BigInt::from(big(&MPNat { digits: out }));
            assert_eq!(result, expected(x, y), "{x}*{y}");

            let mut out = vec![0; 2 * m.digits.len() + 1];
            monsq(&mp(x), &m, n_prime(&m), &mut out);
            let result = BigInt::from(big(&MPNat { digits: out }));
            assert_eq!(result, expected(x, x), "{x}^2");
        }
    }

    #[test]
    fn test_r_mod_n() {
        for n in [
            0x80_00_00_00_01,
            0x8000_0000_0000_0000_0001,
            0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF,
        ] {
            let x = mp(n);
            let mut out = vec![0; x.digits.len()];
            compute_r_mod_n(&x, &mut out);
            let expected =
                BigUint::from(2_u32).pow((WORD_BITS * x.digits.len()) as u32) % BigUint::from(n);
            assert_eq!(
                BigUint::from(mp_nat_to_u128(&MPNat { digits: out })),
                expected
            );
        }
    }

    #[test]
    fn test_in_place_shifts() {
        for (n, shift) in [(1, 10), (Word::MAX as u128, 5), (u128::MAX, 16)] {
            let mut x = mp(n);
            in_place_shl(&mut x.digits, shift);
            let mask = BASE
                .overflowing_pow(x.digits.len() as u32)
                .0
                .wrapping_sub(1);
            assert_eq!(mp_nat_to_u128(&x), n.overflowing_shl(shift).0 & mask);

            let mut x = mp(n);
            in_place_shr(&mut x.digits, shift);
            assert_eq!(mp_nat_to_u128(&x), n.overflowing_shr(shift).0);
        }
    }

    #[test]
    fn test_mod_inv() {
        for n in [1, 3, 0x1234_5679, Word::MAX - 2, Word::MAX] {
            assert_eq!(n.wrapping_mul(mod_inv(n)), 1, "{n}");
        }
    }

    #[test]
    fn test_big_wrapping_pow() {
        for (a, b) in [(1u128, 1u32), (2, 64), (2766, 844)] {
            let expected = BigUint::from(a).pow(b);
            let mut scratch = vec![
                0;
                1 + (expected.to_bytes_be().len()
                    / crate::modexp::software::mpnat::WORD_BYTES)
            ];
            let result = big_wrapping_pow(&mp(a), &b.to_be_bytes(), &mut scratch, Global);
            assert_eq!(big(&result), expected, "{a}^{b}");
        }
    }

    #[test]
    fn test_big_wrapping_mul() {
        for (a, b, output_digits) in [
            (0, 0, 1),
            (Word::MAX.into(), Word::MAX.into(), 1),
            (Word::MAX.into(), Word::MAX.into(), 2),
            (DoubleWord::MAX - 5, DoubleWord::MAX - 6, 2),
            (
                0x8ae1_5515_fc92_b1c0_b473_8ce8_6bbf_7218,
                0x43e9_8b77_1f7c_aa93_6c4c_85e9_7fd0_504f,
                3,
            ),
        ] {
            let expected = (BigUint::from(a) * BigUint::from(b))
                % BigUint::from(2_u32).pow((output_digits * WORD_BITS) as u32);
            let mut out = vec![0; output_digits];
            big_wrapping_mul(&mp(a), &mp(b), &mut out);
            assert_eq!(big(&MPNat { digits: out }), expected, "{a}*{b}");
        }
    }

    #[test]
    fn test_big_sq() {
        // the last one overflows in the inner loop
        for a in [
            "00",
            "ffffffffffffffff",
            "8e67904953db9a2bf6da64bf8bda866d",
            "ffffffffffffffffffffffffffffffff",
            "ffffffff800000008000000040000000ffffffff80000000",
        ] {
            let x = MPNat::from_big_endian(&crate::modexp::hex::decode(a).unwrap(), Global);
            let mut out = vec![0; 2 * x.digits.len() + 1];
            big_sq(&x, &mut out);
            assert_eq!(big(&MPNat { digits: out }), big(&x).pow(2_u32), "{a}^2");
        }
    }

    // These examples are correctly stated
    #[allow(clippy::mistyped_literal_suffixes)]
    #[test]
    fn test_word_arithmetic() {
        assert_eq!(borrowing_sub(47, 5, false), (42, false));
        assert_eq!(borrowing_sub(101, 7, true), (93, false));
        assert_eq!(
            borrowing_sub(0x00_00_01_00, 0x00_00_10_00, true),
            (Word::MAX - 0x0F_00, true)
        );
        assert_eq!(shifted_carrying_mul(5, 6, 7, 8), (55, 0));
        assert_eq!(
            shifted_carrying_mul(
                Word::MAX - 0x11,
                Word::MAX - 0x1234,
                Word::MAX - 0xABCD,
                Word::MAX - 0xFF
            ),
            (0x0C_38_0C_94, Word::MAX - 0xBE00)
        );
        assert_eq!(
            shifted_carrying_mul(Word::MAX, Word::MAX, Word::MAX, Word::MAX),
            (Word::MAX, Word::MAX)
        );
    }
}
