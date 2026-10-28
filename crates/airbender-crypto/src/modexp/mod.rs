//! Arbitrary-length modular exponentiation for the ModExp precompile (`0x05`).
//!
//! [`modexp`] runs the hinted path ([`advice::modexp`]) when given an oracle, and the software
//! path ([`software::modexp`]) otherwise. Both agree after [`write_modexp_output`].

pub mod advice;
pub mod software;
pub(crate) mod u256;

#[cfg(all(test, target_pointer_width = "64"))]
mod test_vectors;

use crate::hints::{read_querier_word, HintError, HintOracle, QuerierMemory};
use alloc::vec::Vec;
use core::alloc::Allocator;

/// `base^exp mod modulus` for big-endian operands: the advice path with an oracle, the software
/// path without one.
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

/// Fallible `Extend`: the destination of [`write_modexp_output`].
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

/// Writes a [`modexp`] result to `dst` as `mod_len` big-endian bytes, truncated to its low bytes
/// or left-padded with zeroes.
pub fn write_modexp_output<D: ?Sized + TryExtend<u8>, A: Allocator>(
    output: Vec<u8, A>,
    mod_len: usize,
    dst: &mut D,
) -> Result<(), D::Error> {
    if output.len() >= mod_len {
        dst.try_extend(output[(output.len() - mod_len)..].iter().copied())
    } else {
        dst.try_extend(core::iter::repeat_n(0, mod_len - output.len()).chain(output))
    }
}

/// The significant bytes of a big-endian integer.
pub fn strip_leading_zeroes(bytes: &[u8]) -> &[u8] {
    let first_nonzero = bytes
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(bytes.len());
    &bytes[first_nonzero..]
}

/// Request of the modexp advice query: the querier sends its address. The oracle answers with the
/// quotient and remainder lengths in `u32` words, then their words, little-endian.
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

/// The request as laid out on RV32.
pub type ModExpAdviceParams = ModExpAdviceParamsGeneric<u32>;

/// The request as laid out on 64-bit hosts.
pub type ModExpAdviceParams64 = ModExpAdviceParamsGeneric<u64>;

/// Reads the request at `address` in querier memory, one querier word per field.
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

/// Hex helpers for tests (there is no `hex` dependency).
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
    use crate::hints::{record_hints, NoHints, RecordingHintOracle};
    use std::alloc::Global;
    use std::vec;

    /// Splits a precompile input (three 32-byte lengths, then the operands, zero-padded if short)
    /// into base, exponent, modulus and the modulus length.
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

    fn padded(output: Vec<u8, Global>, mod_len: usize) -> Vec<u8> {
        let mut dst = Vec::new();
        write_modexp_output(output, mod_len, &mut dst).unwrap();
        dst
    }

    /// FNV-1a of all vector tapes (each as its length, then its words, little-endian).
    /// `tests/modexp_native.rs` checks that the naive `U256` records the same tapes.
    const TAPES_DIGEST: u64 = 0x3b18_d4f1_fabf_8ba6;

    fn digest_tape(digest: &mut u64, tape: &[u32]) {
        for word in core::iter::once(tape.len() as u32).chain(tape.iter().copied()) {
            for byte in word.to_le_bytes() {
                *digest ^= u64::from(byte);
                *digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }

    /// Each vector through the software path, the advice path with recorded hints, a replay of
    /// the tape (which must consume all of it), and the dispatch with and without an oracle.
    #[test]
    fn vectors_both_paths() {
        const NEXT_ANSWER: u32 = 0x5eed_5eed;
        let mut hinted = 0;
        let mut digest = 0xcbf2_9ce4_8422_2325;
        for (name, input, expected) in MODEXP_VECTORS {
            let (base, exp, modulus, mod_len) = parse_input(&hex::decode(input).unwrap());
            let expected = hex::decode(expected).unwrap();

            let software = software::modexp(&base, &exp, &modulus, Global);
            assert_eq!(padded(software.clone(), mod_len), expected, "{name}");

            let (advice, tape) =
                record_hints(|oracle| advice::modexp(&base, &exp, &modulus, oracle, Global));
            assert_eq!(padded(advice.clone(), mod_len), expected, "{name}: advice");
            hinted += usize::from(!tape.is_empty());
            digest_tape(&mut digest, &tape);
            let mut guest = TapeOracle::new(tape.iter().copied().chain([NEXT_ANSWER]));
            let replayed = advice::modexp(&base, &exp, &modulus, &mut guest, Global);
            assert_eq!(replayed, advice, "{name}: replay");
            assert_eq!(guest.words, [NEXT_ANSWER], "{name}: replay leaves words");

            let without_oracle = modexp(&base, &exp, &modulus, Global, None::<&mut NoHints>);
            assert_eq!(without_oracle, software, "{name}: dispatch without oracle");
            let (with_oracle, dispatch_tape) =
                record_hints(|oracle| modexp(&base, &exp, &modulus, Global, Some(oracle)));
            assert_eq!(with_oracle, advice, "{name}: dispatch with oracle");
            assert_eq!(dispatch_tape, tape, "{name}: dispatch tape");
        }
        // the others have modulus 0, base 0 or 1, or exponent 0 with a base below the modulus
        assert_eq!(hinted, 57);
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

    /// Bigint delegations of one advice-path call with honest hints.
    fn delegations(base: &[u8], exp: &[u8], modulus: &[u8]) -> u64 {
        let mut oracle = RecordingHintOracle::new();
        DELEGATION_CALLS.with(|calls| calls.set(0));
        let output = advice::modexp(base, exp, modulus, &mut oracle, Global);
        let calls = DELEGATION_CALLS.with(|calls| calls.get());
        drop(output);
        calls
    }

    /// Delegations the single-digit path's `debug_assert!`s add (2, then 6 per product); the
    /// golden counts are of release builds.
    fn single_digit_debug_delegations(products: usize) -> u64 {
        if cfg!(debug_assertions) {
            2 + 6 * products as u64
        } else {
            0
        }
    }

    /// The xorshift of zksync-os's `bench_modexp_shapes`
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

    /// Bigint delegations per call against zksync-os's RV32 counts, for moduli of 1 to 8 digits
    /// and exponents on both walks. These keep the guest's cost equal to zksync-os's.
    #[test]
    fn delegation_counts_match_zksync_os() {
        const DIGITS: [usize; 5] = [1, 2, 3, 4, 8];
        const PER_CALL: [[u64; 8]; 5] = [
            [22, 34, 36, 48, 60, 1039, 1092, 1051],
            [77, 140, 136, 199, 263, 4736, 4974, 4799],
            [145, 278, 267, 399, 528, 9614, 10096, 9746],
            [234, 461, 439, 666, 892, 16294, 17110, 16523],
            [788, 1631, 1527, 2368, 3205, 58880, 61802, 59722],
        ];
        let windowed = |top: u8, bottom: u8| {
            let mut exp = vec![0u8; 9];
            (exp[0], exp[8]) = (top, bottom);
            exp
        };
        // each exponent and its number of modular products: binary walk below 64 bits, else 14
        // table products and 4 squarings per nibble
        let exps: [(Vec<u8>, usize); 8] = [
            (vec![2], 1),
            (vec![3], 2),
            (vec![4], 2),
            (vec![5], 3),
            (vec![7], 4),
            (windowed(0x01, 0), 14 + 64),
            (windowed(0x10, 0), 14 + 68),
            (windowed(0x01, 1), 14 + 64 + 1),
        ];
        for (digits, golden) in DIGITS.into_iter().zip(PER_CALL) {
            let len = 32 * digits;
            let mut modulus = pseudo_random(1000 + digits as u64, len);
            modulus[0] |= 0x80;
            modulus[len - 1] |= 1;
            let mut base = pseudo_random(2000 + digits as u64, len);
            base[0] &= 0x7f;
            let counts = exps.each_ref().map(|(exp, products)| {
                let calls = delegations(&base, exp, &modulus);
                if digits == 1 {
                    calls - single_digit_debug_delegations(*products)
                } else {
                    calls
                }
            });
            assert_eq!(counts, golden, "{digits} digits");
        }
    }

    /// As above, for zksync-os's `bench_modexp_shapes`: bases as long as the modulus (so an
    /// initial reduction), up to 32 digits.
    #[test]
    fn delegation_counts_of_the_benchmark_shapes_match_zksync_os() {
        let exp_256 = pseudo_random(42, 32);
        let exp_64 = pseudo_random(43, 8);
        let shapes: [(&str, &[u8], usize, u64); 4] = [
            ("modexp_2_digits", &exp_256, 64, 19480),
            ("modexp_rsa2048_verify", &[1, 0, 1], 256, 12617),
            ("modexp_8_digits", &exp_256, 256, 246048),
            ("modexp_32_digits", &exp_64, 1024, 1038319),
        ];
        for (name, exp, len, golden) in shapes {
            let base = pseudo_random(name.len() as u64 * 7919, len);
            let mut modulus = pseudo_random(name.len() as u64 * 104729, len);
            modulus[0] |= 0x80;
            modulus[len - 1] |= 1;
            assert_eq!(delegations(&base, exp, &modulus), golden, "{name}");
        }
    }
}
