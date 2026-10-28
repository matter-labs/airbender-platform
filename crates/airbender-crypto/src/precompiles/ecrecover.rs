//! ecrecover: the public key of a secp256k1 signature.

use super::WordAligned;
use crate::hints::HintOracle;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use crate::hints::Secp256k1HooksWithOracle;

/// The public key (`x || y`, big-endian) that signed `digest` with `(r, s)`, for the signature
/// point with `x = r` and the given `y` parity. Fails if `r` or `s` is not in `[1, order - 1]`,
/// or there is no such point or key. With an oracle, recovery runs in affine coordinates with
/// checked hinted divisions, inverses and square root.
// The caller maps every failure to an empty output
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
    use crate::hints::{answer_query, record_hints, NoHints, QuerierMemory};
    use crate::precompiles::HintedCases;
    use crate::MiniDigest;
    use core::cell::RefCell;
    use hex_literal::hex;
    use proptest::test_runner::Config as ProptestConfig;
    use proptest::{prop_assert_eq, proptest};
    use std::rc::Rc;
    use std::vec::Vec;

    /// The recovered key, comparable
    fn key(result: Result<WordAligned<64>, ()>) -> Result<[u8; 64], ()> {
        result.map(|key| key.0)
    }

    /// The digest, `y` parity, `r` and `s` of an EVM ecrecover input (`v` is 27 or 28)
    fn fields(input: &[u8; 128]) -> (&[u8; 32], bool, &[u8; 32], &[u8; 32]) {
        let ([digest, v, r, s], []) = input.as_chunks::<32>() else {
            unreachable!("128 bytes are 4 chunks of 32")
        };
        (digest, v[31] == 28, r, s)
    }

    /// The address recovered from an EVM ecrecover input, checked to be the same with hints
    fn address(input: &[u8; 128]) -> Option<[u8; 20]> {
        let (digest, odd, r, s) = fields(input);
        let (hinted, _) =
            record_hints(|oracle| key(secp256k1_ecrecover(digest, r, s, odd, Some(oracle))));
        let plain = key(secp256k1_ecrecover(digest, r, s, odd, None::<&mut NoHints>));
        assert_eq!(hinted, plain);
        Some(
            crate::sha3::Keccak256::digest(plain.ok()?)[12..]
                .try_into()
                .unwrap(),
        )
    }

    const GETH_INPUT: [u8; 128] = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e000000000000000000000000000000000000000000000000000000000000001b38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");

    #[test]
    fn known_answers() {
        let other = hex!(
            "22bd0731d4bffa88402625b5ba39e04ee9add6534c31da6c119d825a39822b29"
            "000000000000000000000000000000000000000000000000000000000000001c"
            "66667463d40ac441662188ed3e66329c21aca165133392cc1a14b84485600a87"
            "5087ffc16905cc6ceaef174630ce9dd0c40b3f4e94ff00ee3658a6a67fec2613"
        );
        // the recovered point is at infinity
        let infinity = hex!("6b8d2c81b11b2d699528dde488dbdf2f94293d0d33c32e347f255fa4a6c1f0a9000000000000000000000000000000000000000000000000000000000000001b79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f817986b8d2c81b11b2d699528dde488dbdf2f94293d0d33c32e347f255fa4a6c1f0a9");
        // no point of the curve has `x = r`
        let no_point = hex!("00c547e4f7b0f325ad1e56f57e26c745b09a3e503d86e00e5255ff7f715d3d1c000000000000000000000000000000000000000000000000000000000000001c00b1693892219d736caba55bdb67216e485557ea6b6af75f37096c9aa6a5a75f00b940b1d03b21e36b0e47e79769f095fe2ab855bd91e3a38756b7d75a9c4549");
        let expected = hex!("ceaccac640adf55b2028469bd36ba501f28b699d");
        assert_eq!(address(&GETH_INPUT), Some(expected));
        let expected = hex!("63f9725f107358c9115bc9d86c72dd5823e9b1e6");
        assert_eq!(address(&other), Some(expected));
        assert_eq!(address(&infinity), None);
        assert_eq!(address(&no_point), None);
    }

    #[test]
    fn advice_matches_no_advice() {
        let hinted_cases = HintedCases::default();
        let recover = |digest: &[u8; 32], r: &[u8; 32], s: &[u8; 32], odd: bool| {
            let hinted = hinted_cases
                .record(|oracle| key(secp256k1_ecrecover(digest, r, s, odd, Some(oracle))));
            let plain = key(secp256k1_ecrecover(digest, r, s, odd, None::<&mut NoHints>));
            assert_eq!(hinted, plain);
            plain
        };
        proptest!(ProptestConfig::with_cases(64), |(digest: [u8; 32], key: [u8; 32], r: [u8; 32], s: [u8; 32], odd: bool)| {
            // random bytes: mostly no point of the curve, or a key nobody signed with
            let _ = recover(&digest, &r, &s, odd);

            if let Ok(key) = crate::k256::ecdsa::SigningKey::from_bytes(&key.into()) {
                let (signature, recovery_id) = key.sign_prehash_recoverable(&digest).unwrap();
                if !recovery_id.is_x_reduced() {
                    let signature = signature.to_bytes();
                    let (r, s) = signature.split_at(32);
                    let recovered = recover(&digest, r.try_into().unwrap(), s.try_into().unwrap(), recovery_id.is_y_odd());
                    let public_key = key.verifying_key().to_encoded_point(false);
                    prop_assert_eq!(recovered.map(|key| key.to_vec()), Ok(public_key.as_bytes()[1..].to_vec()));
                }
            }
        });
        hinted_cases.check();
    }

    /// Both answers of every query: the one this process reads, and the one taped for the guest
    type Answers = Rc<RefCell<Vec<(Vec<u32>, Vec<u32>)>>>;

    /// An honest oracle that keeps both answers of every query in `kept`
    fn answer_keeping_oracle(
        kept: Answers,
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                let (mut native, mut guest) = (Vec::new(), Vec::new());
                // SAFETY: the handler runs during the query, in this process, while the request
                // and its operand are live
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

    /// The secp256k1 answers of this process (canonical integers) differ from the guest's
    /// (Montgomery words) but have the same length, so replaying this process's answers through a
    /// tape reader checks that the guest reads the whole tape and no more.
    #[test]
    fn recorded_tape_replays() {
        const NEXT_ANSWER: u32 = 0x5eed_5eed;
        let (digest, odd, r, s) = fields(&GETH_INPUT);

        let (recorded, tape) =
            record_hints(|oracle| key(secp256k1_ecrecover(digest, r, s, odd, Some(oracle))));
        assert!(recorded.is_ok());

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

        let native = answers.iter().flat_map(|(native, _)| native).copied();
        let mut replay = TapeOracle::new(native.chain([NEXT_ANSWER]));
        assert_eq!(
            key(secp256k1_ecrecover(digest, r, s, odd, Some(&mut replay))),
            recorded
        );
        assert_eq!(replay.words, [NEXT_ANSWER]);
        assert_eq!(replay.queries.len(), answers.len());
    }
}
