//! ModExp without `cfg(test)`, on the arithmetic of the build (plain `u64` limbs, or the
//! emulated bigint delegation with `bigint_ops`): the vectors through both paths, with tapes
//! that must match the unit tests' and replay; operands of every size class against the path
//! without hints and `num-bigint`, with the hints of the real oracle side; and tapes with wrong
//! answers, which must be rejected.
#![cfg(all(feature = "oracle_implementations", target_pointer_width = "64"))]

#[path = "../src/modexp/test_vectors.rs"]
mod test_vectors;

mod modexp {
    use super::test_vectors::MODEXP_VECTORS;
    use airbender_crypto::hints::{
        record_hints, HintError, HintTarget, MemoryOracle, NoHints, RecordingHintOracle,
    };
    use airbender_crypto::modexp::{advice, modexp, software, write_modexp_output};
    use num_bigint::BigUint;
    use std::collections::VecDeque;
    use std::sync::{Mutex, MutexGuard};

    /// The emulated delegation of a build with `bigint_ops` keeps one scratch space for the
    /// process: the tests take turns
    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

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

    fn padded(output: &[u8], mod_len: usize) -> Vec<u8> {
        let mut dst = Vec::new();
        write_modexp_output(output, mod_len, &mut dst).unwrap();
        dst
    }

    #[test]
    fn vectors_on_the_arithmetic_of_the_build() {
        let _serial = serial();
        let mut digest = 0xcbf2_9ce4_8422_2325;
        for (name, input, expected) in MODEXP_VECTORS {
            let (base, exp, modulus, mod_len) = parse_input(&decode(input));
            let expected = decode(expected);

            let software = modexp(&base, &exp, &modulus, None::<&mut NoHints>);
            assert_eq!(software, software::modexp(&base, &exp, &modulus));
            assert_eq!(padded(&software, mod_len), expected, "{name}: software");

            let (advice, tape) = record_hints(|oracle| modexp(&base, &exp, &modulus, Some(oracle)));
            assert_eq!(padded(&advice, mod_len), expected, "{name}: advice");
            digest_tape(&mut digest, &tape);

            let mut guest = Tape(tape.into());
            let replayed = advice::modexp(&base, &exp, &modulus, &mut guest);
            assert_eq!(replayed, advice, "{name}: replay");
            assert!(guest.0.is_empty(), "{name}: the replay takes the tape");
        }
        assert_eq!(digest, TAPES_DIGEST, "{digest:#018x}");
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

        fn below(&mut self, bound: usize) -> usize {
            (self.next_u64() % bound as u64) as usize
        }

        fn bytes(&mut self, len: usize) -> Vec<u8> {
            (0..len).map(|_| self.next_u64() as u8).collect()
        }
    }

    /// `base^exp mod modulus` by `num-bigint`, as the `modulus.len()` bytes of the precompile
    fn reference(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
        let m = BigUint::from_bytes_be(modulus);
        let result = if m == BigUint::from(0u32) {
            Vec::new()
        } else {
            BigUint::from_bytes_be(base)
                .modpow(&BigUint::from_bytes_be(exp), &m)
                .to_bytes_be()
        };
        padded(&result, modulus.len())
    }

    /// The hinted path with the answers of the oracle side, recorded as the guest without the
    /// delegation reads them: its output and the tape
    fn record_native(base: &[u8], exp: &[u8], modulus: &[u8]) -> (Vec<u8>, Vec<u32>) {
        let mut recorder = RecordingHintOracle::recording_for(HintTarget::Native);
        let output = advice::modexp(base, exp, modulus, &mut recorder);
        (output.to_vec(), recorder.into_tape())
    }

    /// The hinted path, the path without hints and `num-bigint` agree; the tape of the native
    /// run is the one recorded for the guest with the delegation, and replays to the same output,
    /// consumed exactly. Returns the tape.
    fn check(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u32> {
        let context = || format!("base {base:02x?} exp {exp:02x?} modulus {modulus:02x?}");
        let expected = reference(base, exp, modulus);
        let software = software::modexp(base, exp, modulus);
        assert_eq!(padded(&software, modulus.len()), expected, "{}", context());

        let (hinted, tape) = record_native(base, exp, modulus);
        assert_eq!(padded(&hinted, modulus.len()), expected, "{}", context());
        // the output has no more bytes than the modulus has significant ones, in whole digits
        assert!(
            hinted.len() <= modulus.len().next_multiple_of(32),
            "{}",
            context()
        );

        let (dispatched, guest_tape) =
            record_hints(|oracle| modexp(base, exp, modulus, Some(oracle)));
        assert_eq!(dispatched.to_vec(), hinted, "{}", context());
        assert_eq!(guest_tape, tape, "{}", context());

        let mut guest = Tape(tape.iter().copied().collect());
        let replayed = advice::modexp(base, exp, modulus, &mut guest);
        assert_eq!(replayed.to_vec(), hinted, "{}: replay", context());
        assert!(
            guest.0.is_empty(),
            "{}: the replay takes the tape",
            context()
        );
        tape
    }

    /// Exponents of every kind: none, zero, small, on both walks, sparse, with leading zeroes
    fn exponents(rng: &mut Rng, long: usize) -> Vec<Vec<u8>> {
        let mut sparse = rng.bytes(12);
        for byte in sparse.iter_mut() {
            *byte &= 0x10;
        }
        sparse[0] |= 1;
        let mut with_zeroes = vec![0u8; 5];
        with_zeroes.extend(rng.bytes(2));
        vec![
            vec![],
            vec![0],
            vec![0, 0, 0],
            vec![1],
            vec![2],
            vec![3],
            vec![1, 0, 1],
            rng.bytes(7),
            rng.bytes(8),
            rng.bytes(9),
            sparse,
            with_zeroes,
            rng.bytes(long),
        ]
    }

    /// FNV-1a of the tapes of `size_classes_match_the_references`, as `digest_tape` folds them:
    /// the same on either arithmetic, which ask the same queries whatever their digit is
    const SIZE_CLASS_TAPES_DIGEST: u64 = 0x7d47_49ee_c8ed_67ca;

    /// Moduli of 1 byte to the bound (1 limb, 4 limbs, 5 to 128 limbs, and lengths that are not
    /// whole limbs or digits), odd and even, with bases shorter, as long and longer, and every
    /// kind of exponent; operands with leading zero bytes, limbs and digits
    #[test]
    fn size_classes_match_the_references() {
        let _serial = serial();
        let mut rng = Rng(0x5DEE_CE66_D1CE_4E5B);
        let mut hinted = 0;
        let mut digest = 0xcbf2_9ce4_8422_2325;
        let lengths = [
            1, 2, 7, 8, 9, 16, 24, 31, 32, 33, 39, 40, 41, 48, 63, 64, 65, 72, 96, 100, 128, 129,
            192, 255, 256, 257, 384, 511, 512, 513, 777, 1016, 1023, 1024,
        ];
        for (round, len) in lengths.into_iter().enumerate() {
            // long exponents only where a product is cheap
            let long = if len <= 64 { 40 } else { 10 };
            for (index, exp) in exponents(&mut rng, long).into_iter().enumerate() {
                let mut modulus = rng.bytes(len);
                modulus[0] |= 1 << rng.below(8);
                match (round + index) % 4 {
                    0 => modulus[len - 1] |= 1,
                    1 => modulus[len - 1] &= 0xfe,
                    // divisible by a power of two of whole limbs
                    2 if len > 16 => modulus[len - 16..].fill(0),
                    _ => {}
                }
                let base_len = match index % 6 {
                    0 => 0,
                    1 => 1 + rng.below(len),
                    2 | 3 => len,
                    4 => (len + 1 + rng.below(40)).min(1024),
                    _ => 1024,
                };
                let mut base = rng.bytes(base_len);
                if index % 5 == 3 {
                    // a base above the modulus, of its length
                    base[0] = 0xff;
                }
                let tape = check(&base, &exp, &modulus);
                hinted += usize::from(!tape.is_empty());
                digest_tape(&mut digest, &tape);

                // leading zeroes: bytes, whole limbs and whole digits, on each operand
                if index % 4 == 1 {
                    for zeroes in [1, 8, 32, 45] {
                        let lead = |operand: &[u8]| [&vec![0u8; zeroes][..], operand].concat();
                        let tape = check(&lead(&base), &lead(&exp), &lead(&modulus));
                        assert_eq!(tape, check(&base, &exp, &modulus));
                    }
                }
            }
        }
        assert!(hinted > 300, "{hinted} calls took hints");
        assert_eq!(digest, SIZE_CLASS_TAPES_DIGEST, "{digest:#018x}");
    }

    /// Moduli 0 and 1, bases 0, 1, the modulus and around it, powers of two, and all ones
    #[test]
    fn special_operands_match_the_references() {
        let _serial = serial();
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for len in [1, 8, 32, 33, 40, 64, 100, 256, 1024] {
            let mut modulus = rng.bytes(len);
            modulus[0] |= 0x80;
            let m = BigUint::from_bytes_be(&modulus);
            let one = BigUint::from(1u32);
            let mut power = vec![0u8; len];
            power[0] = 0x80;
            let mut even_power = vec![0u8; len];
            even_power[len - 1 - (len - 1) / 2] = 1;
            let bases = [
                vec![],
                vec![0; len],
                vec![1],
                vec![2],
                modulus.clone(),
                (&m - &one).to_bytes_be(),
                (&m + &one).to_bytes_be(),
                (&m * &m).to_bytes_be()[..len.min(1024)].to_vec(),
                vec![0xff; len],
                vec![0xff; 1024],
                power.clone(),
            ];
            let moduli = [
                modulus.clone(),
                vec![0xff; len],
                power,
                even_power,
                // 0 and 1, in any number of bytes
                vec![0; len],
                [vec![0; len - 1], vec![1]].concat(),
                vec![],
            ];
            let exps = [vec![], vec![1], vec![2], vec![0x0b], rng.bytes(9)];
            for modulus in &moduli {
                for base in &bases {
                    for exp in &exps {
                        check(base, exp, modulus);
                    }
                }
            }
        }
        // a huge exponent, on a modulus of each path
        for len in [20, 32, 48] {
            let mut modulus = rng.bytes(len);
            modulus[len - 1] |= 1;
            check(&rng.bytes(len), &rng.bytes(1024), &modulus);
        }
    }

    /// The outcome of the hinted path on a tape: its padded output if it returns, having read
    /// the whole tape or not, and none if it panics
    fn replay(base: &[u8], exp: &[u8], modulus: &[u8], tape: &[u32]) -> Option<(Vec<u8>, bool)> {
        let mut guest = Tape(tape.iter().copied().collect());
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            advice::modexp(base, exp, modulus, &mut guest).to_vec()
        }))
        .ok()
        .map(|output| (padded(&output, modulus.len()), guest.0.is_empty()))
    }

    /// Operands for the wrong tapes: a modulus of one 256-bit value on both walks, moduli of 5,
    /// 8 and 32 limbs, a base longer than the modulus, and the longest operands
    fn tamper_cases(rng: &mut Rng) -> Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let mut odd = |len: usize| {
            let mut modulus = rng.bytes(len);
            modulus[0] |= 0x80;
            modulus[len - 1] |= 1;
            modulus
        };
        let moduli = [
            odd(32),
            odd(32),
            odd(40),
            odd(64),
            odd(256),
            odd(48),
            odd(1024),
        ];
        let [m32, m32_other, m40, m64, m256, m48, m1024] = moduli;
        vec![
            (rng.bytes(32), vec![0x1d], m32),
            (rng.bytes(70), rng.bytes(9), m32_other),
            (rng.bytes(40), vec![0x1d], m40),
            (rng.bytes(64), rng.bytes(9), m64),
            (rng.bytes(256), vec![1, 0, 1], m256),
            (rng.bytes(200), vec![5], m48),
            (rng.bytes(1024), vec![3], m1024),
        ]
    }

    /// Any one word of a tape changed: the call panics. It never returns another result.
    #[test]
    fn tapes_with_a_wrong_word_are_rejected() {
        let _serial = serial();
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        let (mut rejected, mut accepted) = (0, 0);
        for (base, exp, modulus) in tamper_cases(&mut rng) {
            let expected = reference(&base, &exp, &modulus);
            let tape = check(&base, &exp, &modulus);
            assert!(tape.len() >= 16);
            // every word of a short tape, and a spread of the words of a long one
            let step = tape.len().div_ceil(160);
            for index in (0..tape.len()).step_by(step).chain([tape.len() - 1]) {
                let word = tape[index];
                for wrong in [word ^ 1, word ^ (1 << 31), word.wrapping_add(2), !word] {
                    let mut tampered = tape.clone();
                    tampered[index] = wrong;
                    match replay(&base, &exp, &modulus, &tampered) {
                        Some((output, _)) => {
                            assert_eq!(output, expected, "word {index} as {wrong:#x} is accepted");
                            accepted += 1;
                        }
                        None => rejected += 1,
                    }
                }
            }
        }
        assert_eq!(
            accepted, 0,
            "{accepted} wrong tapes are accepted, {rejected} rejected"
        );
        assert!(rejected > 2000, "{rejected} wrong tapes");
    }

    /// A tape cut short at any point panics: no missing word is made up
    #[test]
    fn truncated_tapes_are_rejected() {
        let _serial = serial();
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for (base, exp, modulus) in tamper_cases(&mut rng) {
            let tape = check(&base, &exp, &modulus);
            let step = tape.len().div_ceil(120);
            for len in (0..tape.len()).step_by(step).chain([tape.len() - 1]) {
                assert!(
                    replay(&base, &exp, &modulus, &tape[..len]).is_none(),
                    "a tape of {len} words of {} is accepted",
                    tape.len()
                );
            }
        }
    }

    /// The answers of a tape of the multi-digit path: each is its quotient and remainder
    /// lengths in words, then their words
    fn answers(tape: &[u32]) -> Vec<(Vec<u32>, Vec<u32>)> {
        let mut answers = Vec::new();
        let mut rest = tape;
        while !rest.is_empty() {
            let (q_len, r_len) = (rest[0] as usize, rest[1] as usize);
            let (quotient, tail) = rest[2..].split_at(q_len);
            let (remainder, tail) = tail.split_at(r_len);
            answers.push((quotient.to_vec(), remainder.to_vec()));
            rest = tail;
        }
        answers
    }

    /// The tape of `answers`
    fn tape_of(answers: &[(Vec<u32>, Vec<u32>)]) -> Vec<u32> {
        let mut tape = Vec::new();
        for (quotient, remainder) in answers {
            tape.extend([quotient.len() as u32, remainder.len() as u32]);
            tape.extend(quotient);
            tape.extend(remainder);
        }
        tape
    }

    fn words_of(x: &BigUint) -> Vec<u32> {
        if *x == BigUint::from(0u32) {
            Vec::new()
        } else {
            x.to_u32_digits()
        }
    }

    fn value_of(words: &[u32]) -> BigUint {
        BigUint::new(words.to_vec())
    }

    /// What a forgery answers, given the true quotient and remainder: their words
    type Forgery<'a> = dyn Fn(&BigUint, &BigUint) -> (Vec<u32>, Vec<u32>) + 'a;

    /// The answers of the multi-digit path replaced by wrong ones, or by other spellings of the
    /// right ones: lengths beyond the buffers, a quotient off by one, a remainder that is not
    /// reduced, non-zero and zero high limbs, answers cut short. A wrong value panics; a right
    /// one in another spelling gives the right result.
    #[test]
    fn forged_answers_on_the_tape() {
        let _serial = serial();
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        let mut odd = |len: usize| {
            let mut modulus = rng.bytes(len);
            modulus[0] |= 0x80;
            modulus[len - 1] |= 1;
            modulus
        };
        let cases = [
            (odd(40), vec![0x1d], odd(40)),
            (odd(64), vec![1, 0, 1], odd(64)),
            (odd(100), vec![0x0b], odd(72)),
            (odd(256), vec![3], odd(256)),
            (odd(1024), vec![2], odd(1024)),
        ];
        let one = BigUint::from(1u32);
        for (base, exp, modulus) in cases {
            let expected = reference(&base, &exp, &modulus);
            let m = BigUint::from_bytes_be(&modulus);
            let tape = check(&base, &exp, &modulus);
            let honest = answers(&tape);
            assert_eq!(tape_of(&honest), tape);
            assert!(honest.len() >= 2);
            let modulus_words = modulus.len().div_ceil(4);

            // `forge` on the answer `index`: the outcome of the call
            let forged = |index: usize, forge: &Forgery| {
                let mut answers = honest.clone();
                let (q, r) = (value_of(&honest[index].0), value_of(&honest[index].1));
                answers[index] = forge(&q, &r);
                replay(&base, &exp, &modulus, &tape_of(&answers))
            };
            let rejected = |index: usize, forge: &Forgery| {
                assert_eq!(
                    forged(index, forge),
                    None,
                    "answer {index} of {}",
                    honest.len()
                );
            };
            let last = honest.len() - 1;
            for index in [0, 1, last / 2, last] {
                // the honest answer, and its spellings with zero high words, limbs and digits
                // up to the length of the modulus
                assert_eq!(
                    forged(index, &|q, r| (words_of(q), words_of(r))),
                    Some((expected.clone(), true))
                );
                for zeroes in [1, 2, 8] {
                    let spelled = forged(index, &|q, r| {
                        let mut remainder = words_of(r);
                        let len = (remainder.len() + zeroes).min(modulus_words);
                        remainder.resize(len, 0);
                        (words_of(q), remainder)
                    });
                    assert_eq!(spelled, Some((expected.clone(), true)), "answer {index}");
                }
                // a quotient off by one, either way, and a remainder off by one
                rejected(index, &|q, r| (words_of(&(q + &one)), words_of(r)));
                if honest[index].0.iter().any(|word| *word != 0) {
                    rejected(index, &|q, r| (words_of(&(q - &one)), words_of(r)));
                }
                rejected(index, &|q, r| (words_of(q), words_of(&(r + &one))));
                // off by one with the remainder that makes it congruent, when that one has more
                // words than the modulus, or is negative
                rejected(index, &|q, r| {
                    (words_of(&(q + &one)), words_of(&((r + &m) >> 1usize)))
                });
                // a non-zero high limb on the quotient and on the remainder
                rejected(index, &|q, r| {
                    let mut quotient = words_of(q);
                    quotient.resize(quotient.len().next_multiple_of(2), 0);
                    quotient.extend([1, 0]);
                    (quotient, words_of(r))
                });
                rejected(index, &|q, r| {
                    let mut remainder = words_of(r);
                    remainder.resize(modulus_words.next_multiple_of(2) + 2, 0);
                    *remainder.last_mut().unwrap() = 1;
                    (words_of(q), remainder)
                });
                rejected(index, &|q, r| {
                    let mut remainder = words_of(r);
                    remainder.resize(modulus_words, 0);
                    *remainder.last_mut().unwrap() ^= 1 << 31;
                    (words_of(q), remainder)
                });
                // answers that are none, the dividend's worth of ones, and swapped
                rejected(index, &|_, _| (Vec::new(), Vec::new()));
                rejected(index, &|_, _| {
                    (vec![u32::MAX; modulus_words], vec![u32::MAX; modulus_words])
                });
                rejected(index, &|q, r| (words_of(r), words_of(&(q + &one))));
            }

            // a remainder that is congruent but not below the modulus, as the last answer, is the
            // result: the last check rejects it (in the middle of the walk the answers after it
            // are for the reduced one, and reject it too)
            for index in [0, last] {
                if honest[index].0.iter().any(|word| *word != 0) {
                    rejected(index, &|q, r| (words_of(&(q - &one)), words_of(&(r + &m))));
                }
            }

            // lengths beyond the answer and beyond the buffers: rejected before anything is
            // written, whatever follows on the tape
            for (q_len, r_len) in [
                (u32::MAX, 0),
                (0, u32::MAX),
                (u32::MAX, u32::MAX),
                (1 << 31, 1 << 31),
                (4096, 0),
                (0, 4096),
                (2 * modulus_words as u32 + 16, 0),
                (0, modulus_words as u32 + 8),
                (0x4000_0002, 0),
            ] {
                let mut wrong = vec![q_len, r_len];
                wrong.extend(std::iter::repeat_n(1u32, 20_000));
                assert_eq!(
                    replay(&base, &exp, &modulus, &wrong),
                    None,
                    "{q_len} and {r_len}"
                );
                let mut wrong = tape.clone();
                (wrong[0], wrong[1]) = (q_len, r_len);
                assert_eq!(
                    replay(&base, &exp, &modulus, &wrong),
                    None,
                    "{q_len} and {r_len}"
                );
            }
        }
    }
}
