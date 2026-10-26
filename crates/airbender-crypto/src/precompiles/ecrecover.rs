// Ported from zksync-os e855bca2:basic_system/src/system_functions/ecrecover.rs
//! ecrecover: the public key of a secp256k1 signature.

use super::WordAligned;
use crate::hints::HintOracle;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints::Secp256k1HooksWithOracle;

/// The public key (its coordinates `x` and `y`, big-endian) of the signature `(r, s)` of
/// `digest`, for the point of the signature with the coordinate `x = r` and the `y` of the
/// given parity. Fails if `r` or `s` is not in `[1, order - 1]`, or there is no such point or
/// key.
///
/// With an oracle, the recovery works in affine coordinates, with its field divisions, inverses
/// and square root from checked hints.
// zksync-os's signature: the wrapper turns any failure into an empty output
#[allow(clippy::result_unit_err)]
pub fn secp256k1_ecrecover<O: HintOracle>(
    digest: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    y_is_odd: bool,
    oracle: Option<&mut O>,
) -> Result<WordAligned<64>, ()> {
    use crate::secp256k1::{hooks::DefaultSecp256k1Hooks, recover_from_bytes_with_hooks};

    let public_key = match oracle {
        #[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
        Some(oracle) => recover_from_bytes_with_hooks(
            digest,
            r,
            s,
            y_is_odd,
            &mut Secp256k1HooksWithOracle::new(oracle),
        ),
        #[cfg(all(target_arch = "riscv32", not(feature = "bigint_ops")))]
        Some(_) => super::hints_unavailable(),
        None => recover_from_bytes_with_hooks(digest, r, s, y_is_odd, &mut DefaultSecp256k1Hooks),
    }
    .map_err(|_| ())?;

    // represent as bytes, and we do not need compression
    let mut encoded = WordAligned([0u8; 64]);
    let ([x, y], []) = encoded.0.as_chunks_mut::<32>() else {
        unreachable!("64 bytes are 2 chunks of 32")
    };
    public_key.write_coordinates(x, y);

    Ok(encoded)
}

// The tests use the host side (`hints::host`), which serves 64-bit hosts only.
#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::hints::test_support::{InProcessMemoryOracle, TapeOracle};
    use crate::hints::{answer_query, record_hints, NoHints, QuerierMemory, RecordingHintOracle};
    use crate::precompiles::HintedCases;
    use crate::MiniDigest;
    use core::cell::RefCell;
    use hex_literal::hex;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert, prop_assert_eq, proptest};
    use std::rc::Rc;
    use std::vec;
    use std::vec::Vec;

    /// The EVM half of zksync-os's `ecrecover_as_system_function_inner` (`ecrecover.rs:60-93`),
    /// without the gas: the address of the recovered key as 32 bytes, or no bytes for an invalid
    /// input.
    fn ecrecover_as_system_function<O: HintOracle>(src: &[u8], oracle: Option<&mut O>) -> Vec<u8> {
        // digest, v, r, s in ABI
        let mut buffer = WordAligned([0u8; 128]);
        for (dst, src) in buffer.0.iter_mut().zip(src.iter()) {
            *dst = *src;
        }

        let ([digest, v, r, s], []) = buffer.0.as_chunks::<32>() else {
            unreachable!("128 bytes are 4 chunks of 32")
        };

        if v[..31].iter().any(|el| *el != 0) {
            return vec![];
        }

        let rec_id = v[31].wrapping_sub(27);
        if !(rec_id == 0 || rec_id == 1) {
            return vec![];
        }

        let Ok(public_key) = secp256k1_ecrecover(digest, r, s, rec_id == 1, oracle) else {
            return vec![];
        };

        let address_hash = crate::sha3::Keccak256::digest(public_key.0);

        core::iter::repeat_n(0, 12)
            .chain(address_hash.iter().copied().skip(12))
            .collect()
    }

    /// The bytes of a recovered key, comparable
    fn key(result: Result<WordAligned<64>, ()>) -> Result<[u8; 64], ()> {
        result.map(|key| key.0)
    }

    /// Both paths: without the oracle (as zksync-os's vector tests run), and with the hints of an
    /// honest oracle, which must give the same output
    fn run(input: &[u8; 128]) -> Vec<u8> {
        let output = ecrecover_as_system_function(input.as_slice(), None::<&mut NoHints>);
        let hinted =
            ecrecover_as_system_function(input.as_slice(), Some(&mut RecordingHintOracle::new()));
        assert_eq!(hinted, output);
        output
    }

    const GETH_INPUT: [u8; 128] = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e000000000000000000000000000000000000000000000000000000000000001b38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");

    #[test]
    fn test_geth_ecrecover() {
        let expected_pubkey: [u8; 32] =
            hex!("000000000000000000000000ceaccac640adf55b2028469bd36ba501f28b699d");

        let pubkey = run(&GETH_INPUT);
        assert_eq!(pubkey.len(), 32, "Size should be 32");
        assert_eq!(
            pubkey, expected_pubkey,
            "pubkey should be equal to reference"
        )
    }

    #[test]
    fn test_empty_input() {
        let input = [0u8; 128];
        let pubkey = run(&input);
        assert_eq!(pubkey.len(), 0, "Size should be 0");
    }

    #[test]
    fn test_point_of_infinity_in_result() {
        let input: [u8; 128] = hex!("6b8d2c81b11b2d699528dde488dbdf2f94293d0d33c32e347f255fa4a6c1f0a9000000000000000000000000000000000000000000000000000000000000001b79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f817986b8d2c81b11b2d699528dde488dbdf2f94293d0d33c32e347f255fa4a6c1f0a9");

        let pubkey = run(&input);
        assert_eq!(pubkey.len(), 0, "Size should be 0 in case of error");
    }

    #[test]
    fn test_affine_point_decompression_regression() {
        let input: [u8; 128] = hex!("00c547e4f7b0f325ad1e56f57e26c745b09a3e503d86e00e5255ff7f715d3d1c000000000000000000000000000000000000000000000000000000000000001c00b1693892219d736caba55bdb67216e485557ea6b6af75f37096c9aa6a5a75f00b940b1d03b21e36b0e47e79769f095fe2ab855bd91e3a38756b7d75a9c4549");

        let pubkey = run(&input);
        assert_eq!(pubkey.len(), 0, "Size should be 0 in case of error");
    }

    #[test]
    fn test_regressions() {
        let input: [u8; 128] = [
            34, 189, 7, 49, 212, 191, 250, 136, 64, 38, 37, 181, 186, 57, 224, 78, 233, 173, 214,
            83, 76, 49, 218, 108, 17, 157, 130, 90, 57, 130, 43, 41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 102, 102, 116, 99,
            212, 10, 196, 65, 102, 33, 136, 237, 62, 102, 50, 156, 33, 172, 161, 101, 19, 51, 146,
            204, 26, 20, 184, 68, 133, 96, 10, 135, 80, 135, 255, 193, 105, 5, 204, 108, 234, 239,
            23, 70, 48, 206, 157, 208, 196, 11, 63, 78, 148, 255, 0, 238, 54, 88, 166, 166, 127,
            236, 38, 19,
        ];

        let expected_pubkey: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 99, 249, 114, 95, 16, 115, 88, 201, 17, 91, 201,
            216, 108, 114, 221, 88, 35, 233, 177, 230,
        ];
        let pubkey = run(&input);
        assert_eq!(pubkey.len(), 32, "Size should be 32");
        assert_eq!(
            pubkey, expected_pubkey,
            "pubkey should be equal to reference"
        )
    }

    /// With the advice of the oracle (affine arithmetic with hinted divisions) the result is the
    /// one without it, for signatures that recover a key and for the inputs that do not
    #[test]
    fn advice_matches_no_advice() {
        // the endpoint itself, with and without the advice
        let hinted_cases = HintedCases::default();
        let recover = |digest: &[u8; 32], r: &[u8; 32], s: &[u8; 32], odd: bool| {
            let hinted = hinted_cases
                .record(|oracle| key(secp256k1_ecrecover(digest, r, s, odd, Some(oracle))));
            let plain = key(secp256k1_ecrecover(digest, r, s, odd, None::<&mut NoHints>));
            assert_eq!(hinted, plain);
            plain
        };

        fn run(input: &[u8; 128], advice: bool) -> Vec<u8> {
            if advice {
                record_hints(|oracle| ecrecover_as_system_function(input.as_slice(), Some(oracle)))
                    .0
            } else {
                ecrecover_as_system_function(input.as_slice(), None::<&mut NoHints>)
            }
        }

        proptest!(ProptestConfig::with_cases(64), |(digest: [u8; 32], key: [u8; 32], r: [u8; 32], s: [u8; 32], odd: bool)| {
            // the endpoint itself, on any bytes: mostly no point of the curve, or a key nobody
            // signed with
            let _ = recover(&digest, &r, &s, odd);

            let mut input = [0u8; 128];
            input[..32].copy_from_slice(&digest);

            // any bytes: mostly no point of the curve, or a key nobody signed with
            input[63] = 27 + u8::from(odd);
            input[64..96].copy_from_slice(&r);
            input[96..].copy_from_slice(&s);
            prop_assert_eq!(run(&input, true), run(&input, false));

            if let Ok(key) = crate::k256::ecdsa::SigningKey::from_bytes(&key.into()) {
                let (signature, recovery_id) = key.sign_prehash_recoverable(&digest).unwrap();
                if !recovery_id.is_x_reduced() {
                    input[63] = 27 + u8::from(recovery_id.is_y_odd());
                    input[64..].copy_from_slice(&signature.to_bytes());
                    let recovered = run(&input, true);
                    prop_assert_eq!(&recovered, &run(&input, false));

                    let public_key = key.verifying_key().to_encoded_point(false);
                    let address = crate::sha3::Keccak256::digest(&public_key.as_bytes()[1..]);
                    prop_assert!(recovered[..12].iter().all(|byte| *byte == 0));
                    prop_assert_eq!(&recovered[12..], &address[12..]);

                    let r: [u8; 32] = input[64..96].try_into().unwrap();
                    let s: [u8; 32] = input[96..].try_into().unwrap();
                    prop_assert_eq!(
                        recover(&digest, &r, &s, recovery_id.is_y_odd()).map(|key| key.to_vec()),
                        Ok(public_key.as_bytes()[1..].to_vec())
                    );
                }
            }
        });
        hinted_cases.check();
    }

    /// The answers of every query: the one this process reads, and the one the recorder puts on
    /// the tape for the guest
    type Answers = Rc<RefCell<Vec<(Vec<u32>, Vec<u32>)>>>;

    /// An honest oracle (the recorder's `answer_query`) that keeps both answers of every query in
    /// `kept`
    fn answer_keeping_oracle(
        kept: Answers,
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                let (mut native, mut guest) = (Vec::new(), Vec::new());
                // SAFETY: the handler of an `InProcessMemoryOracle` runs during the query, with the
                // memory of this process, where the request and its operand are live, of the
                // types of the op
                assert!(unsafe {
                    answer_query(
                        query_id,
                        input_word,
                        memory,
                        Some(&mut native),
                        Some(&mut guest),
                    )
                }
                .expect("must answer the field hint query"));
                kept.borrow_mut().push((native.clone(), guest));
                native
            },
        )
    }

    /// The guest reads a recorded tape the way this process reads its own answers: as many words
    /// of every answer, in the same order. The secp256k1 answers of this process (canonical
    /// integers) are not those of the guest (Montgomery words), so this process replays its own
    /// answers through the reader of a tape, and the tape is checked to be the guest answers.
    #[test]
    fn recorded_tape_replays() {
        const NEXT_ANSWER: u32 = 0x5eed_5eed;
        let input = &GETH_INPUT;
        let digest: &[u8; 32] = input[..32].try_into().unwrap();
        let r: &[u8; 32] = input[64..96].try_into().unwrap();
        let s: &[u8; 32] = input[96..].try_into().unwrap();
        let odd = input[63] == 28;

        let (recorded, tape) =
            record_hints(|oracle| key(secp256k1_ecrecover(digest, r, s, odd, Some(oracle))));
        assert!(recorded.is_ok());
        assert!(!tape.is_empty());

        let answers = Answers::default();
        let mut oracle = answer_keeping_oracle(answers.clone());
        assert_eq!(
            key(secp256k1_ecrecover(digest, r, s, odd, Some(&mut oracle))),
            recorded
        );
        let answers = answers.take();
        let guest: Vec<u32> = answers
            .iter()
            .flat_map(|(_, guest)| guest)
            .copied()
            .collect();
        assert_eq!(guest, tape);
        for (native, guest) in &answers {
            assert_eq!(native.len(), guest.len());
        }

        let mut replay = TapeOracle::new(
            answers
                .iter()
                .flat_map(|(native, _)| native)
                .copied()
                .chain([NEXT_ANSWER]),
        );
        assert_eq!(
            key(secp256k1_ecrecover(digest, r, s, odd, Some(&mut replay))),
            recorded
        );
        assert_eq!(replay.words, [NEXT_ANSWER]);
        assert_eq!(replay.queries.len(), answers.len());
    }
}
