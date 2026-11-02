//! Hints ("advice"): values that the prover supplies through the oracle and the querier checks,
//! such as field inverses and square roots.
//!
//! A query is two oracle words: the query ID, then the address of a request struct of the querier.
//! The answer is raw `u32` words in the representation of the querier's target (see
//! [`HintTarget`]), with no framing. Field hints and the wide quotient have no length prefix; the
//! ModExp reduction answers with two length words (quotient and remainder, in `u32` words), which
//! are not field limbs, then their words.

mod answer;
mod encoding;
// The hinted operations (`hinted_operations`): the answers are read in the representation of
// the fields of the build.
#[cfg(any(feature = "hinted_operations", test))]
mod fields;
pub mod guest_layout;
mod oracle;
mod pairing;
mod request;
mod secp256k1;
#[cfg(test)]
pub(crate) mod test_support;
pub(crate) mod wide_div;

pub use self::answer::{read_hint_answer, HintAnswer, HintTarget};
pub use self::encoding::HintEncoding;
#[cfg(any(feature = "hinted_operations", test))]
pub use self::fields::{
    bls12_381_fq12_inverse, bls12_381_fq2_inverse, bls12_381_fq2_sqrt, bls12_381_fq_inverse,
    bls12_381_fq_sqrt, bn254_fq12_inverse, bn254_fq_inverse, Bls12381Fq2Divider, Bls12381FqDivider,
    Bn254FqDivider,
};
// The oracle side (`oracle_common`), kept here for the callers of the hints
pub use self::oracle::{HintError, NoHints};
#[cfg(any(feature = "hinted_operations", test))]
pub use self::pairing::{
    bls12_381_kzg_residue_witness, bls12_381_pairing_residue_witness, bn254_g2_pairing_slopes,
    bn254_pairing_residue_witness, G2SlopeHints, HintedChain,
};
pub use self::pairing::{checked_inverse, Bls12381Pair, PairingClaim};
pub use self::request::{
    query_field_hint, query_field_hint_into, read_field_hint_request, send_field_hint_query,
    FieldHintOp, FieldOpsHint, FieldOpsHint64, GenericFieldOpsHint,
};
pub use self::secp256k1::{RecoveryRequest, Secp256k1Element, Secp256k1HooksWithOracle};
#[cfg(all(
    any(feature = "oracle_implementations", test),
    target_pointer_width = "64"
))]
pub use crate::oracle_common::{
    answer_query, not_identity_claim, read_querier_u32_words, record_hints, NativeQuerierMemory,
    RecordingHintOracle,
};

pub use airbender_oracle::{
    read_querier_word, MemoryOracle, OracleError, QuerierMemory, QuerierMemoryError,
};

pub use airbender_oracle::query_ids::{
    ADVICE_SUBSPACE_MASK, FIELD_OPS_ADVISE_QUERY_ID, MODEXP_ADVICE_QUERY_ID,
    U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
};

#[cfg(test)]
mod tests {
    use super::test_support::TapeOracle;
    use super::*;
    use core::mem::{offset_of, size_of, MaybeUninit};

    #[test]
    fn query_ids_match_zksync_os() {
        assert_eq!(ADVICE_SUBSPACE_MASK, 0x4005_0000);
        assert_eq!(FIELD_OPS_ADVISE_QUERY_ID, 0x4005_0011);
        assert_eq!(MODEXP_ADVICE_QUERY_ID, 0x4005_0010);
        assert_eq!(U256_WIDE_DIV_REM_ADVICE_QUERY_ID, 0x4005_0031);
    }

    #[test]
    fn field_hint_op_numbers_match_zksync_os() {
        let ops = [
            (FieldHintOp::Secp256k1BaseFieldSqrt, 0),
            (FieldHintOp::Secp256k1BaseFieldInverse, 1),
            (FieldHintOp::Secp256k1ScalarFieldInverse, 2),
            (FieldHintOp::Bn254BaseFieldInverse, 3),
            (FieldHintOp::Bn254Fq12Inverse, 4),
            (FieldHintOp::Bls12381BaseFieldSqrt, 5),
            (FieldHintOp::Bls12381BaseFieldInverse, 6),
            (FieldHintOp::Bls12381Fq12Inverse, 7),
            (FieldHintOp::Bn254PairingResidueWitness, 8),
            (FieldHintOp::Bn254G2PairingSlopes, 9),
            (FieldHintOp::Bls12381KzgResidueWitness, 10),
            (FieldHintOp::Secp256k1BaseFieldDivision, 11),
            (FieldHintOp::Bn254BaseFieldDivision, 12),
            (FieldHintOp::Bls12381BaseFieldDivision, 13),
            (FieldHintOp::Secp256k1RecoveryWitness, 14),
            (FieldHintOp::Bls12381PairingResidueWitness, 15),
        ];
        for (op, number) in ops {
            assert_eq!(op as u32, number, "{op:?}");
            assert_eq!(FieldHintOp::parse_u32(number), Some(op));
        }
        assert_eq!(FieldHintOp::parse_u32(16), None);
    }

    #[test]
    fn request_layout() {
        assert_eq!(offset_of!(GenericFieldOpsHint<u32>, src_ptr), 4);
        assert_eq!(offset_of!(GenericFieldOpsHint<u32>, src_len_u32_words), 8);
        assert_eq!(size_of::<GenericFieldOpsHint<u32>>(), 12);
        assert_eq!(offset_of!(GenericFieldOpsHint<u64>, src_ptr), 8);
        assert_eq!(offset_of!(GenericFieldOpsHint<u64>, src_len_u32_words), 16);
        assert_eq!(size_of::<GenericFieldOpsHint<u64>>(), 24);
    }

    /// One raw word, received with `write_words` like a field element
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(transparent)]
    struct Word(u32);

    impl HintAnswer for Word {
        fn read<'a, O: MemoryOracle>(
            oracle: &mut O,
            dst: &'a mut MaybeUninit<Self>,
        ) -> Result<&'a mut Self, O::Error> {
            // SAFETY: `Word` is one aligned `u32`
            unsafe { oracle.write_words(dst.as_mut_ptr().cast::<u32>(), 1)? };
            // SAFETY: just written
            Ok(unsafe { dst.assume_init_mut() })
        }

        fn write(&self, _target: HintTarget, response: &mut Vec<u32>) {
            response.push(self.0);
        }
    }

    #[test]
    fn answers_round_trip() {
        let pair = (false, [Word(1), Word(u32::MAX)]);
        let mut tape = Vec::new();
        pair.write(HintTarget::Guest, &mut tape);
        assert_eq!(tape, [0, 1, u32::MAX]);
        tape.extend([2, 0]);
        let mut oracle = TapeOracle::new(tape);
        assert_eq!(read_hint_answer(&mut oracle), Ok(pair));
        // any non-zero word is `true`
        assert_eq!(read_hint_answer(&mut oracle), Ok([true, false]));
        // a short tape is an error, not a made-up word
        assert!(read_hint_answer::<_, bool>(&mut oracle).is_err());
    }

    /// A querier memory of zeroes, with any word size
    struct Zeroes(usize);

    impl QuerierMemory for Zeroes {
        fn word_size(&self) -> usize {
            self.0
        }

        fn read_u32(&self, _address: usize) -> Result<u32, QuerierMemoryError> {
            Ok(0)
        }
    }

    #[test]
    fn malformed_field_hint_requests_are_errors() {
        assert_eq!(
            read_field_hint_request(&Zeroes(2), 0x1000),
            Err(HintError("unsupported querier word size"))
        );
        assert_eq!(
            read_querier_word(&Zeroes(2), 0x1000),
            Err(QuerierMemoryError("unsupported querier word size"))
        );
        assert_eq!(
            read_field_hint_request(&Zeroes(4), usize::MAX - 3),
            Err(HintError("querier address overflows"))
        );
    }
}
