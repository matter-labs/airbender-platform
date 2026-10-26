// Ported from zksync-os 185a0568:basic_system/src/system_functions/modexp/mod.rs
// Ported from zksync-os 185a0568:zk_ee/src/common_traits/mod.rs
//! Arbitrary-length modular exponentiation, the crypto half of the zksync-os ModExp precompile
//! (`0x05`): the wrapper keeps the EVM half (the parsing of the header, the EIP-7823 length cap,
//! gas and the native cost, and the cycle markers).
//!
//! [`modexp`] takes an optional oracle (see [`crate::hints`]). With one, it runs the advice path of
//! zksync-os `185a0568` ([`advice::modexp`]), which asks the same hints in the same order and checks
//! them (a wrong hint panics); with none, its path without advice ([`software::modexp`]). This
//! replaces zksync-os's choice by target and `USE_ADVICE`: the caller chooses. The results are the
//! same once [`write_modexp_output`] pads or truncates them to the length of the modulus.

pub mod advice;
pub mod software;
pub(crate) mod u256;

#[cfg(all(test, target_pointer_width = "64"))]
mod test_vectors;

use crate::hints::{read_querier_word, HintError, HintOracle, QuerierMemory};
use alloc::vec::Vec;
use core::alloc::Allocator;

/// `base^exp mod modulus` for big-endian operands, as the precompile computes it before
/// [`write_modexp_output`]: the advice path with an oracle, the software path without one.
pub fn modexp<O: HintOracle, A: Allocator + Clone>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
    allocator: A,
    oracle: Option<&mut O>,
) -> Vec<u8, A> {
    match oracle {
        Some(oracle) => advice::modexp(base, exp, modulus, oracle, allocator),
        None => software::modexp(base, exp, modulus, allocator),
    }
}

/// Fallible `Extend` (zksync-os `zk_ee::common_traits::TryExtend`): the destination of
/// [`write_modexp_output`].
pub trait TryExtend<T> {
    type Error;

    fn try_extend<I>(&mut self, iter: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = T>;
}

impl<A: Allocator, T> TryExtend<T> for Vec<T, A> {
    type Error = ();

    fn try_extend<I>(&mut self, iter: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = T>,
    {
        self.extend(iter);
        Ok(())
    }
}

/// Writes the output of the precompile to `dst`: the result of [`modexp`] (of any of its paths)
/// as `mod_len` big-endian bytes, truncated to its low bytes or left-padded with zeroes.
pub fn write_modexp_output<D: ?Sized + TryExtend<u8>, A: Allocator>(
    output: Vec<u8, A>,
    mod_len: usize,
    dst: &mut D,
) -> Result<(), D::Error> {
    if output.len() >= mod_len {
        // truncate
        dst.try_extend(output[(output.len() - mod_len)..].iter().copied())
    } else {
        dst.try_extend(core::iter::repeat_n(0, mod_len - output.len()).chain(output))
    }
}

/// The bytes of `bytes` from its first non-zero one: the significant bytes of a big-endian integer
/// (zksync-os's cost model of ModExp counts the significant digits with it too)
pub fn strip_leading_zeroes(bytes: &[u8]) -> &[u8] {
    let first_nonzero = bytes
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(bytes.len());
    &bytes[first_nonzero..]
}

/// Parameters for modular exponentiation oracle query
/// Used to request division advice for big integer operations during modexp. The querier sends the
/// address of a `ModExpAdviceParamsGeneric<usize>`, and the oracle answers with the lengths of the
/// quotient and of the remainder in `u32` words, then their words (little-endian).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ModExpAdviceParamsGeneric<W> {
    pub op: W,          // Operation type (0 = division)
    pub a_ptr: W,       // Pointer to dividend
    pub a_len: W,       // Length of dividend in 256-bit digits
    pub b_ptr: W,       // Pointer to divisor
    pub b_len: W,       // Length of divisor in 256-bit digits
    pub modulus_ptr: W, // Pointer to modulus
    pub modulus_len: W, // Length of modulus in 256-bit digits
}

/// Used for proving (RISC-V 32-bit)
pub type ModExpAdviceParams = ModExpAdviceParamsGeneric<u32>;

/// Used for native execution (64-bit)
pub type ModExpAdviceParams64 = ModExpAdviceParamsGeneric<u64>;

/// The request of a modexp advice query, as the oracle reads it through the input word: the address
/// of a `ModExpAdviceParamsGeneric<usize>` of the querier, whose fields are querier words.
pub fn read_modexp_advice_params<M: QuerierMemory + ?Sized>(
    memory: &M,
    address: usize,
) -> Result<ModExpAdviceParamsGeneric<usize>, HintError> {
    let word_size = memory.word_size();
    let field = |index: usize| {
        let field_address = index
            .checked_mul(word_size)
            .and_then(|offset| address.checked_add(offset))
            .ok_or(HintError("querier address overflows"))?;
        read_querier_word(memory, field_address)
    };
    Ok(ModExpAdviceParamsGeneric {
        op: field(0)?,
        a_ptr: field(1)?,
        a_len: field(2)?,
        b_ptr: field(3)?,
        b_len: field(4)?,
        modulus_ptr: field(5)?,
        modulus_len: field(6)?,
    })
}

/// `hex::decode` and `hex::encode` for the ported tests (there is no `hex` crate here)
#[cfg(test)]
pub(crate) mod hex {
    use std::string::String;
    use std::vec::Vec;

    pub(crate) fn decode(digits: &str) -> Result<Vec<u8>, &'static str> {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err("not a hex digit"),
        };
        let digits = digits.as_bytes();
        if !digits.len().is_multiple_of(2) {
            return Err("odd number of hex digits");
        }
        digits
            .chunks(2)
            .map(|pair| Ok((digit(pair[0])? << 4) | digit(pair[1])?))
            .collect()
    }

    pub(crate) fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|byte| std::format!("{byte:02x}"))
            .collect()
    }
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::test_vectors::MODEXP_VECTORS;
    use super::*;
    use crate::bigint_delegation::delegation::DELEGATION_CALLS;
    use crate::hints::test_support::TapeOracle;
    use crate::hints::{
        record_hints, NoHints, RecordingHintOracle, MODEXP_ADVICE_QUERY_ID,
        U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
    };
    use num_bigint::BigUint;
    use num_traits::Zero;
    use std::alloc::Global;
    use std::vec;

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex::decode("00aBcD"), Ok(vec![0x00, 0xab, 0xcd]));
        assert_eq!(hex::encode([0x00, 0xab, 0xcd]), "00abcd");
        assert!(hex::decode("abc").is_err());
        assert!(hex::decode("0g").is_err());
    }

    /// The operands of a precompile input, as the wrapper of zksync-os reads them
    /// (`modexp/mod.rs:282-411`): three 32-byte big-endian lengths, then the base, the exponent
    /// and the modulus, all zero-padded where the input is short. Returns them and the length of
    /// the modulus.
    fn parse_input(input: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, usize) {
        let mut header = [0u8; 96];
        for (dst, src) in header.iter_mut().zip(input) {
            *dst = *src;
        }
        let len = |index: usize| {
            let bytes: [u8; 32] = header[32 * index..32 * (index + 1)].try_into().unwrap();
            usize::try_from(ruint::aliases::U256::from_be_bytes(bytes)).unwrap()
        };
        let (base_len, exp_len, mod_len) = (len(0), len(1), len(2));
        let mut rest = input.get(96..).unwrap_or_default();
        let mut read_padded = |len: usize| {
            let (bytes, tail) = rest.split_at(len.min(rest.len()));
            rest = tail;
            let mut operand = bytes.to_vec();
            operand.resize(len, 0);
            operand
        };
        let base = read_padded(base_len);
        let exp = read_padded(exp_len);
        let modulus = read_padded(mod_len);
        (base, exp, modulus, mod_len)
    }

    /// The output of the precompile for the result `output` of `modexp`
    fn padded(output: Vec<u8, Global>, mod_len: usize) -> Vec<u8> {
        let mut dst = Vec::new();
        write_modexp_output(output, mod_len, &mut dst).unwrap();
        dst
    }

    /// The vector named `name`, parsed
    fn vector(name: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>, usize, Vec<u8>) {
        let (_, input, expected) = MODEXP_VECTORS
            .iter()
            .find(|(vector, _, _)| *vector == name)
            .unwrap();
        let (base, exp, modulus, mod_len) = parse_input(&hex::decode(input).unwrap());
        (base, exp, modulus, mod_len, hex::decode(expected).unwrap())
    }

    /// FNV-1a of the tapes of the vectors, each word little-endian and each tape after its
    /// length in words. The integration test `tests/modexp_native.rs`, where the single-digit path
    /// runs on the naive `U256` (the one of native builds), checks that its tapes give the same.
    const TAPES_DIGEST: u64 = 0x3b18_d4f1_fabf_8ba6;

    fn digest_tape(digest: &mut u64, tape: &[u32]) {
        for word in core::iter::once(tape.len() as u32).chain(tape.iter().copied()) {
            for byte in word.to_le_bytes() {
                *digest ^= u64::from(byte);
                *digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }

    /// The 62 ModExp vectors of zksync-os's precompile tests, through the software path, through
    /// the advice path with the recording oracle, and through the advice path again replaying
    /// the recorded tape as the guest reads it (the whole tape and no more), and through the
    /// dispatch with and without an oracle
    #[test]
    fn vectors_both_paths() {
        const NEXT_ANSWER: u32 = 0x5eed_5eed;
        assert_eq!(MODEXP_VECTORS.len(), 62);
        let mut hinted = 0;
        let mut digest = 0xcbf2_9ce4_8422_2325;
        for (name, input, expected) in MODEXP_VECTORS {
            let (base, exp, modulus, mod_len) = parse_input(&hex::decode(input).unwrap());
            let expected = hex::decode(expected).unwrap();

            let software = software::modexp(&base, &exp, &modulus, Global);
            assert_eq!(
                padded(software.clone(), mod_len),
                expected,
                "{name}: software"
            );

            let (advice, tape) =
                record_hints(|oracle| advice::modexp(&base, &exp, &modulus, oracle, Global));
            assert_eq!(padded(advice.clone(), mod_len), expected, "{name}: advice");
            if !tape.is_empty() {
                hinted += 1;
            }
            digest_tape(&mut digest, &tape);
            let mut guest = TapeOracle::new(tape.iter().copied().chain([NEXT_ANSWER]));
            let replayed = advice::modexp(&base, &exp, &modulus, &mut guest, Global);
            assert_eq!(replayed, advice, "{name}: replay");
            assert_eq!(
                guest.words,
                [NEXT_ANSWER],
                "{name}: the replay takes the tape"
            );

            let without_oracle = modexp(&base, &exp, &modulus, Global, None::<&mut NoHints>);
            assert_eq!(
                without_oracle, software,
                "{name}: dispatch without an oracle"
            );
            let (with_oracle, dispatch_tape) =
                record_hints(|oracle| modexp(&base, &exp, &modulus, Global, Some(oracle)));
            assert_eq!(with_oracle, advice, "{name}: dispatch with an oracle");
            assert_eq!(dispatch_tape, tape, "{name}: dispatch tape");
        }
        // all but the two with a modulus of 0, and those with a base of 0, a base of 1, and a zero
        // exponent with a base of fewer digits than the modulus
        assert_eq!(hinted, 57, "{hinted} vectors took hints");
        assert_eq!(digest, TAPES_DIGEST, "{digest:#018x}");
    }

    #[test]
    fn output_is_padded_or_truncated_to_the_modulus_length() {
        let output = |bytes: &[u8]| {
            let mut vec = Vec::new_in(Global);
            vec.extend_from_slice(bytes);
            vec
        };
        assert_eq!(padded(output(&[1, 2]), 4), [0, 0, 1, 2]);
        assert_eq!(padded(output(&[1, 2]), 2), [1, 2]);
        assert_eq!(padded(output(&[1, 2, 3]), 2), [2, 3]);
        assert_eq!(padded(output(&[]), 3), [0, 0, 0]);
        assert_eq!(padded(output(&[0]), 0), [0u8; 0]);

        /// A destination that fails
        struct Full;

        impl TryExtend<u8> for Full {
            type Error = &'static str;

            fn try_extend<I: IntoIterator<Item = u8>>(
                &mut self,
                _iter: I,
            ) -> Result<(), Self::Error> {
                Err("full")
            }
        }

        assert_eq!(write_modexp_output(output(&[1]), 1, &mut Full), Err("full"));
        assert_eq!(write_modexp_output(output(&[1]), 3, &mut Full), Err("full"));
    }

    /// An oracle that counts the queries it forwards, by query ID
    struct CountingOracle<O> {
        inner: O,
        modexp_queries: usize,
        wide_quotient_queries: usize,
    }

    impl<O: HintOracle> HintOracle for CountingOracle<O> {
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            match query_id {
                MODEXP_ADVICE_QUERY_ID => self.modexp_queries += 1,
                U256_WIDE_DIV_REM_ADVICE_QUERY_ID => self.wide_quotient_queries += 1,
                _ => panic!("unexpected query {query_id:#x}"),
            }
            // SAFETY: guaranteed by the caller
            unsafe { self.inner.send_query(query_id, input_word) }
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.inner.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            // SAFETY: guaranteed by the caller
            unsafe { self.inner.write_words(dst, num_words) }
        }

        fn finish_query(&mut self) -> Result<(), HintError> {
            self.inner.finish_query()
        }
    }

    /// A call of the advice path with honest answers: its bigint delegations (on the host
    /// emulation of the delegation, whose calls `DELEGATION_CALLS` counts on this thread), and its
    /// ModExp and wide quotient queries
    fn measure(base: &[u8], exp: &[u8], modulus: &[u8]) -> (u64, usize, usize) {
        let mut oracle = CountingOracle {
            inner: RecordingHintOracle::new(),
            modexp_queries: 0,
            wide_quotient_queries: 0,
        };
        DELEGATION_CALLS.with(|calls| calls.set(0));
        let output = advice::modexp(base, exp, modulus, &mut oracle, Global);
        let calls = DELEGATION_CALLS.with(|calls| calls.get());
        drop(output);
        (calls, oracle.modexp_queries, oracle.wide_quotient_queries)
    }

    /// The delegations that the `debug_assert!`s of the single-digit path add to a call with
    /// `products` modular multiplications and squarings in a debug build: `!m.is_zero() &&
    /// !m.is_one()` at the start (2 comparisons), and `x < m && y < m` in each product (2 scratch
    /// copies, comparisons and subtractions). The RV32 counts of zksync-os are of a release build.
    fn single_digit_debug_delegations(products: usize) -> u64 {
        if cfg!(debug_assertions) {
            2 + 6 * products as u64
        } else {
            0
        }
    }

    /// The wide quotients the single-digit path asks to reduce `base` (`single_digit::reduce_base`):
    /// one per 32-byte digit from the top, except while the accumulator is zero and the digit is
    /// below the modulus
    fn base_reduction_queries(base: &[u8], modulus: &[u8]) -> usize {
        let m = BigUint::from_bytes_be(modulus);
        let (head, digits) = strip_leading_zeroes(base).as_rchunks::<32>();
        let head = (!head.is_empty()).then_some(head);
        let mut acc = BigUint::zero();
        let mut queries = 0;
        for digit in head
            .into_iter()
            .chain(digits.iter().map(|digit| &digit[..]))
        {
            let lo = BigUint::from_bytes_be(digit);
            if acc.is_zero() && lo < m {
                acc = lo;
                continue;
            }
            queries += 1;
            acc = ((acc << 256u32) + lo) % &m;
        }
        queries
    }

    /// The xorshift of zksync-os's `bench_modexp_shapes` (`tests/instances/precompiles/src/lib.rs`)
    fn pseudo_random(seed: u64, len: usize) -> Vec<u8> {
        let mut x = seed | 1;
        (0..len)
            .map(|_| {
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 56) as u8
            })
            .collect()
    }

    /// The bigint delegations of a ModExp call against the golden counts that zksync-os
    /// `185a0568` gives on RV32 (the transpiler's delegation counters inside the `modexp` cycle
    /// markers), on these inputs: a modulus of 1, 2,
    /// 3, 4 and 8 digits (`pseudo_random(1000 + digits)` with the top bit and bit 0 set), a base
    /// below it (`pseudo_random(2000 + digits)` with the top bit cleared), and exponents that
    /// differ only in the exponent walk. The differences of inputs that differ only in the
    /// exponent cancel everything else (parsing, base reduction, output, the precompile wrapper):
    /// they are the per-operation costs, constant for the single-digit path (a multiplication 12,
    /// a squaring 13, the zero check of the accumulator 1) and dependent on the operands for the
    /// multi-digit one. The per-call counts are compared too: the wrapper of zksync-os makes no
    /// bigint delegation.
    #[test]
    fn delegation_counts_match_zksync_os() {
        const DIGITS: [usize; 5] = [1, 2, 3, 4, 8];
        // e2, e3, e4, e5, e7, w01_00, w10_00, w01_01
        const PER_CALL: [[u64; 8]; 5] = [
            [22, 34, 36, 48, 60, 1039, 1092, 1051],
            [77, 140, 136, 199, 263, 4736, 4974, 4799],
            [145, 278, 267, 399, 528, 9614, 10096, 9746],
            [234, 461, 439, 666, 892, 16294, 17110, 16523],
            [788, 1631, 1527, 2368, 3205, 58880, 61802, 59722],
        ];
        // e3 - e2, e5 - e4, e7 - e5, w01_01 - w01_00 (a multiplication each), e4 - e2 (a squaring
        // and a zero check), w10_00 - w01_00 (four squarings and a zero check)
        const DIFFERENCES: [[u64; 6]; 5] = [
            [12, 12, 12, 12, 14, 53],
            [63, 63, 64, 63, 59, 238],
            [133, 132, 129, 132, 122, 482],
            [227, 227, 226, 229, 205, 816],
            [843, 841, 837, 842, 739, 2922],
        ];
        let mut windowed_lo = vec![0u8; 9];
        windowed_lo[0] = 0x01;
        let mut windowed_hi = vec![0u8; 9];
        windowed_hi[0] = 0x10;
        let mut windowed_lo_mul = windowed_lo.clone();
        windowed_lo_mul[8] = 0x01;
        // the exponents, and the products (multiplications and squarings) of their walk: a
        // squaring per bit after the leading one and a multiplication per set bit; for the
        // windowed walk, the 14 products of the table, then 4 squarings per nibble after the
        // leading non-zero one and a multiplication per non-zero nibble
        let exps: [(Vec<u8>, usize); 8] = [
            (vec![2], 1),
            (vec![3], 2),
            (vec![4], 2),
            (vec![5], 3),
            (vec![7], 4),
            (windowed_lo, 14 + 64),
            (windowed_hi, 14 + 68),
            (windowed_lo_mul, 14 + 64 + 1),
        ];
        for (row, digits) in DIGITS.into_iter().enumerate() {
            let len = 32 * digits;
            let mut modulus = pseudo_random(1000 + digits as u64, len);
            modulus[0] |= 0x80;
            modulus[len - 1] |= 1;
            let mut base = pseudo_random(2000 + digits as u64, len);
            base[0] &= 0x7f;
            let counts = exps.each_ref().map(|(exp, products)| {
                let (calls, modexp_queries, wide_quotient_queries) = measure(&base, exp, &modulus);
                if digits == 1 {
                    // the base is below the modulus: a wide quotient per product
                    assert_eq!(modexp_queries, 0);
                    assert_eq!(wide_quotient_queries, *products);
                    calls - single_digit_debug_delegations(*products)
                } else {
                    assert_eq!(wide_quotient_queries, 0);
                    calls
                }
            });
            let [e2, e3, e4, e5, e7, w01_00, w10_00, w01_01] = counts;
            assert_eq!(
                [
                    e3 - e2,
                    e5 - e4,
                    e7 - e5,
                    w01_01 - w01_00,
                    e4 - e2,
                    w10_00 - w01_00
                ],
                DIFFERENCES[row],
                "differences, {digits} digits"
            );
            assert_eq!(counts, PER_CALL[row], "per call, {digits} digits");
        }
        // the single-digit constants: S + Z = 14 and 4 S + Z = 53
        let [mul, square_and_zero_check, four_squares_and_zero_check] =
            [DIFFERENCES[0][0], DIFFERENCES[0][4], DIFFERENCES[0][5]];
        let square = (four_squares_and_zero_check - square_and_zero_check) / 3;
        assert_eq!((mul, square, square_and_zero_check - square), (12, 13, 1));
    }

    /// The per-call bigint delegations of the `MODEXP_SINGLE_DIGIT_TESTS` vectors against zksync-os
    /// `185a0568` on RV32
    #[test]
    fn delegation_counts_of_the_single_digit_vectors_match_zksync_os() {
        const PER_CALL: [(&str, u64); 11] = [
            ("modexp_single_digit_bn254_fr_inverse", 4238),
            ("modexp_single_digit_max_quotient", 4286),
            ("modexp_single_digit_pow2_modulus_2p255", 1296),
            (
                "modexp_single_digit_prime_2p256m189_inverse_of_minus1",
                4286,
            ),
            ("modexp_single_digit_two_limb_modulus", 4416),
            ("modexp_single_digit_binary_5bit", 76),
            ("modexp_single_digit_binary_63bit", 1620),
            ("modexp_single_digit_windowed_64bit", 998),
            ("modexp_single_digit_oversized_base_100B", 295),
            ("modexp_single_digit_zero_absorption_windowed", 256),
            ("modexp_single_digit_zero_absorption_binary", 34),
        ];
        for (name, golden) in PER_CALL {
            let (base, exp, modulus, _, _) = vector(name);
            let (calls, modexp_queries, wide_quotient_queries) = measure(&base, &exp, &modulus);
            assert_eq!(modexp_queries, 0, "{name}");
            let products = wide_quotient_queries - base_reduction_queries(&base, &modulus);
            assert_eq!(
                calls - single_digit_debug_delegations(products),
                golden,
                "{name}"
            );
        }
    }

    /// The per-call bigint delegations of zksync-os's `bench_modexp_shapes` against zksync-os
    /// `185a0568` on RV32: multi-digit moduli, with bases of
    /// the same length (so an initial reduction)
    #[test]
    fn delegation_counts_of_the_benchmark_shapes_match_zksync_os() {
        let exp_256 = pseudo_random(42, 32);
        let exp_64 = pseudo_random(43, 8);
        let shapes: [(&str, usize, &[u8], usize, u64); 4] = [
            ("modexp_2_digits", 64, &exp_256, 64, 19480),
            ("modexp_rsa2048_verify", 256, &[1, 0, 1], 256, 12617),
            ("modexp_8_digits", 256, &exp_256, 256, 246048),
            ("modexp_32_digits", 1024, &exp_64, 1024, 1038319),
        ];
        for (name, base_len, exp, mod_len, golden) in shapes {
            let base = pseudo_random(name.len() as u64 * 7919, base_len);
            let mut modulus = pseudo_random(name.len() as u64 * 104729, mod_len);
            modulus[0] |= 0x80;
            modulus[mod_len - 1] |= 1;
            let (calls, _, wide_quotient_queries) = measure(&base, exp, &modulus);
            assert_eq!(wide_quotient_queries, 0, "{name}");
            assert_eq!(calls, golden, "{name}");
        }
    }

    /// Without an oracle the software path runs and asks nothing; with one, the advice path, which
    /// asks for hints
    #[test]
    fn the_path_is_chosen_by_the_presence_of_an_oracle() {
        for (base, exp, modulus) in [
            (&[3u8][..], &[0x11u8; 9][..], &[0xf1u8; 32][..]),
            (&[3u8][..], &[0x11u8; 9][..], &[0xf1u8; 33][..]),
        ] {
            let software = software::modexp(base, exp, modulus, Global);
            assert_eq!(
                modexp(base, exp, modulus, Global, None::<&mut NoHints>),
                software
            );
            let mut oracle = CountingOracle {
                inner: RecordingHintOracle::new(),
                modexp_queries: 0,
                wide_quotient_queries: 0,
            };
            let advice = modexp(base, exp, modulus, Global, Some(&mut oracle));
            assert!(oracle.modexp_queries + oracle.wide_quotient_queries > 0);
            assert_eq!(
                BigUint::from_bytes_be(&advice),
                BigUint::from_bytes_be(&software)
            );
        }
    }
}
