//! ModExp without `cfg(test)`, where the single-digit path runs on the naive `U256`: the vectors
//! through both paths, with tapes that must match the unit tests' (delegated `U256`) and replay.
#![cfg(all(feature = "alloc", target_pointer_width = "64"))]
#![cfg_attr(
    all(feature = "alloc", target_pointer_width = "64"),
    feature(allocator_api)
)]

#[path = "../src/modexp/test_vectors.rs"]
mod test_vectors;

mod modexp {
    use super::test_vectors::MODEXP_VECTORS;
    use airbender_crypto::hints::{record_hints, HintError, MemoryOracle, NoHints};
    use airbender_crypto::modexp::{advice, modexp, software, write_modexp_output};
    use std::alloc::Global;
    use std::collections::VecDeque;

    /// `modexp::tests::TAPES_DIGEST`
    const TAPES_DIGEST: u64 = 0x3b18_d4f1_fabf_8ba6;

    /// As `modexp::tests::digest_tape`
    fn digest_tape(digest: &mut u64, tape: &[u32]) {
        for word in core::iter::once(tape.len() as u32).chain(tape.iter().copied()) {
            for byte in word.to_le_bytes() {
                *digest ^= u64::from(byte);
                *digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }

    /// The guest's oracle: every answer word is the next word of the tape
    struct Tape(VecDeque<u32>);

    impl MemoryOracle for Tape {
        type Error = HintError;

        unsafe fn send_query(
            &mut self,
            _query_id: u32,
            _input_word: usize,
        ) -> Result<(), HintError> {
            Ok(())
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.0.pop_front().ok_or(HintError("the tape is exhausted"))
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            for i in 0..num_words {
                let word = self.read_word()?;
                // SAFETY: `i < num_words`, guaranteed by the caller
                unsafe { dst.add(i).write(word) };
            }
            Ok(())
        }
    }

    fn decode(digits: &str) -> Vec<u8> {
        (0..digits.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&digits[i..i + 2], 16).unwrap())
            .collect()
    }

    /// As `modexp::tests::parse_input`
    fn parse_input(input: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, usize) {
        let mut header = [0u8; 96];
        for (dst, src) in header.iter_mut().zip(input) {
            *dst = *src;
        }
        let len = |index: usize| {
            let bytes = &header[32 * index..32 * (index + 1)];
            assert!(bytes[..24].iter().all(|byte| *byte == 0));
            u64::from_be_bytes(bytes[24..].try_into().unwrap()) as usize
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

    #[test]
    fn vectors_on_the_naive_digits() {
        let mut digest = 0xcbf2_9ce4_8422_2325;
        for (name, input, expected) in MODEXP_VECTORS {
            let (base, exp, modulus, mod_len) = parse_input(&decode(input));
            let expected = decode(expected);

            let software = modexp(&base, &exp, &modulus, Global, None::<&mut NoHints>);
            assert_eq!(software, software::modexp(&base, &exp, &modulus, Global));
            assert_eq!(padded(software, mod_len), expected, "{name}: software");

            let (advice, tape) =
                record_hints(|oracle| modexp(&base, &exp, &modulus, Global, Some(oracle)));
            assert_eq!(padded(advice.clone(), mod_len), expected, "{name}: advice");
            digest_tape(&mut digest, &tape);

            let mut guest = Tape(tape.into());
            let replayed = advice::modexp(&base, &exp, &modulus, &mut guest, Global);
            assert_eq!(replayed, advice, "{name}: replay");
            assert!(guest.0.is_empty(), "{name}: the replay takes the tape");
        }
        assert_eq!(digest, TAPES_DIGEST, "{digest:#018x}");
    }
}
