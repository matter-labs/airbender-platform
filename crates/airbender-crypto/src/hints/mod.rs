// Ported from zksync-os e855bca2:zk_ee/src/oracle/query_ids.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/field_ops.rs
// Ported from zksync-os e855bca2:basic_system/src/system_functions/modexp/mod.rs
//! Hints ("advice"): values that the prover supplies through the oracle and the querier checks,
//! such as field inverses and square roots.
//!
//! A query is two oracle words, the query ID and then the address of a request struct of the
//! querier. The answer is raw `u32` words in the representation of the target of the querier (see
//! [`HintTarget`]); the protocol adds no framing. The field hints (`FIELD_OPS_ADVISE_QUERY_ID`) and
//! the wide quotient of ModExp (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`) have no length prefix. The
//! ModExp reduction (`MODEXP_ADVICE_QUERY_ID`) answers with two length words (the lengths of the
//! quotient and of the remainder, in `u32` words), which are not field limbs, and then their words.

mod answer;
#[cfg(target_arch = "riscv32")]
mod csr;
mod encoding;
// The hints of the bn254 and bls12-381 fields answer straight into the representation of the
// RISC-V guest, which is the delegated one: on RV32 they need `bigint_ops` (which `proving` turns
// on), and are not there without it
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
mod fields;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub mod guest_layout;
// The oracle side, which reads the requests of a querier in this process as `FieldOpsHint64`
#[cfg(all(any(feature = "alloc", test), target_pointer_width = "64"))]
mod host;
mod oracle;
mod pairing;
mod request;
mod secp256k1;
#[cfg(test)]
pub(crate) mod test_support;
// The wide division advice of the single-digit ModExp path, on its `U256`s
#[cfg(any(feature = "alloc", test))]
pub(crate) mod wide_div;

pub use self::answer::{read_hint_answer, HintAnswer, HintTarget};
#[cfg(target_arch = "riscv32")]
pub use self::csr::CsrHintOracle;
pub use self::encoding::HintEncoding;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub use self::fields::{
    bls12_381_fq12_inverse, bls12_381_fq2_inverse, bls12_381_fq2_sqrt, bls12_381_fq_inverse,
    bls12_381_fq_sqrt, bn254_fq12_inverse, bn254_fq_inverse, Bls12381Fq2Divider, Bls12381FqDivider,
    Bn254FqDivider,
};
#[cfg(all(any(feature = "alloc", test), target_pointer_width = "64"))]
pub use self::host::{
    answer_query, not_identity_claim, read_querier_u32_words, record_hints, NativeQuerierMemory,
    RecordingHintOracle,
};
pub use self::oracle::{HintError, HintOracle, NoHints};
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
pub use self::pairing::{
    bls12_381_kzg_residue_witness, bls12_381_pairing_residue_witness, bn254_g2_pairing_slopes,
    bn254_pairing_residue_witness, G2SlopeHints, HintedChain,
};
pub use self::pairing::{checked_inverse, Bls12381Pair, PairingClaim};
pub use self::request::{
    query_field_hint, query_field_hint_into, read_field_hint_request, read_querier_word,
    send_field_hint_query, FieldHintOp, FieldOpsHint, FieldOpsHint64, GenericFieldOpsHint,
    QuerierMemory,
};
pub use self::secp256k1::{Secp256k1Element, Secp256k1HooksWithOracle};

/// Computational advice queries (e.g. division/modexp advice)
pub const ADVICE_SUBSPACE_MASK: u32 = 0x4005_0000;

/// Field hints (see [`FieldHintOp`])
pub const FIELD_OPS_ADVISE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x11;

/// ModExp advice
pub const MODEXP_ADVICE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x10;

/// Query to get wide div_rem hint (quotient) for a 512-bit dividend and 256-bit divisor.
/// Guest sends pointers to dividend_lo, dividend_hi, and divisor; host returns q_lo and q_hi.
/// Guest derives remainder r = dividend - q*divisor and verifies 0 <= r < divisor.
pub const U256_WIDE_DIV_REM_ADVICE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x31;

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
        // `e855bca2:basic_system/src/system_functions/field_ops.rs:42-91`, in declaration order
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
            (FieldHintOp::Bls12381PairingResidueWitness, 14),
        ];
        for (op, number) in ops {
            assert_eq!(op as u32, number, "{op:?}");
            assert_eq!(FieldHintOp::parse_u32(number), Some(op));
        }
        assert_eq!(FieldHintOp::parse_u32(15), None);
        assert_eq!(FieldHintOp::parse_u32(u32::MAX), None);
    }

    #[test]
    fn no_hints_is_the_oracle_of_none() {
        fn hinted<O: HintOracle>(oracle: Option<&mut O>) -> bool {
            oracle.is_some()
        }
        assert!(!hinted::<NoHints>(None));
        assert!(hinted(Some(&mut TapeOracle::new([]))));
        // through `&mut O` too
        assert!(hinted(Some(&mut &mut TapeOracle::new([]))));
    }

    #[test]
    fn request_layout() {
        assert_eq!(offset_of!(GenericFieldOpsHint<u32>, op), 0);
        assert_eq!(offset_of!(GenericFieldOpsHint<u32>, src_ptr), 4);
        assert_eq!(offset_of!(GenericFieldOpsHint<u32>, src_len_u32_words), 8);
        assert_eq!(size_of::<GenericFieldOpsHint<u32>>(), 12);
        assert_eq!(offset_of!(GenericFieldOpsHint<u64>, op), 0);
        assert_eq!(offset_of!(GenericFieldOpsHint<u64>, src_ptr), 8);
        assert_eq!(offset_of!(GenericFieldOpsHint<u64>, src_len_u32_words), 16);
        assert_eq!(size_of::<GenericFieldOpsHint<u64>>(), 24);
    }

    /// A raw word answer received with `write_words`, the way the field element answers of later
    /// hints are
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(transparent)]
    struct Word(u32);

    impl HintAnswer for Word {
        fn read<'a, O: HintOracle>(
            oracle: &mut O,
            dst: &'a mut MaybeUninit<Self>,
        ) -> Result<&'a mut Self, HintError> {
            // SAFETY: `Word` is one aligned `u32`
            unsafe { oracle.write_words(dst.as_mut_ptr().cast::<u32>(), 1)? };
            // SAFETY: just written
            Ok(unsafe { dst.assume_init_mut() })
        }

        fn write(&self, _target: HintTarget, response: &mut Vec<u32>) {
            response.push(self.0);
        }
    }

    fn guest_words<T: HintAnswer>(value: &T) -> Vec<u32> {
        let mut response = Vec::new();
        value.write(HintTarget::Guest, &mut response);
        response
    }

    #[test]
    fn answers_round_trip() {
        let flag = true;
        let words = [Word(7), Word(u32::MAX), Word(0)];
        let pair = (false, [Word(1), Word(2)]);

        assert_eq!(guest_words(&flag), [1]);
        assert_eq!(guest_words(&false), [0]);
        assert_eq!(guest_words(&words), [7, u32::MAX, 0]);
        assert_eq!(guest_words(&pair), [0, 1, 2]);

        let mut tape = guest_words(&flag);
        tape.extend(guest_words(&words));
        tape.extend(guest_words(&pair));
        let mut oracle = TapeOracle::new(tape);
        assert_eq!(read_hint_answer::<_, bool>(&mut oracle), Ok(flag));
        assert_eq!(read_hint_answer::<_, [Word; 3]>(&mut oracle), Ok(words));
        assert_eq!(
            read_hint_answer::<_, (bool, [Word; 2])>(&mut oracle),
            Ok(pair)
        );
        assert!(oracle.words.is_empty());
        assert!(oracle.queries.is_empty());

        // any non-zero word is `true`
        let mut oracle = TapeOracle::new([2, 0]);
        assert_eq!(
            read_hint_answer::<_, [bool; 2]>(&mut oracle),
            Ok([true, false])
        );
        // a short tape is an error, not a made-up word
        assert!(read_hint_answer::<_, bool>(&mut oracle).is_err());
    }

    /// The memory of this process, whose addresses are exposed pointers
    struct ProcessMemory;

    impl QuerierMemory for ProcessMemory {
        fn word_size(&self) -> usize {
            size_of::<usize>()
        }

        fn read_u32(&self, address: usize) -> Result<u32, HintError> {
            // SAFETY: only reads the request and the operand of a query while it is sent
            Ok(unsafe { core::ptr::with_exposed_provenance::<u32>(address).read() })
        }
    }

    /// Reads the request and the operand of each query while it is sent, as the oracle does, and
    /// answers from a tape
    struct RequestReader {
        tape: TapeOracle,
        requests: Vec<(u32, usize, Vec<u32>)>,
    }

    impl HintOracle for RequestReader {
        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
            let (op, src_ptr, src_len_u32_words) =
                read_field_hint_request(&ProcessMemory, input_word)?;
            let operand = (0..src_len_u32_words as usize)
                .map(|i| ProcessMemory.read_u32(src_ptr + 4 * i))
                .collect::<Result<_, _>>()?;
            self.requests.push((op, src_ptr, operand));
            // SAFETY: guaranteed by the caller (and the tape reads nothing)
            unsafe { self.tape.send_query(query_id, input_word) }
        }

        fn read_word(&mut self) -> Result<u32, HintError> {
            self.tape.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
            // SAFETY: guaranteed by the caller
            unsafe { self.tape.write_words(dst, num_words) }
        }
    }

    #[test]
    fn field_hint_query_sends_the_request_and_reads_the_answer() {
        let operand: [u32; 8] = core::array::from_fn(|i| 100 + i as u32);
        let mut oracle = RequestReader {
            tape: TapeOracle::new([1, 9]),
            requests: Vec::new(),
        };
        // SAFETY: the oracle reads the request and the operand as words only, which `operand` is
        let answer: (bool, Word) =
            unsafe { query_field_hint(&mut oracle, FieldHintOp::Bn254BaseFieldInverse, &operand) };
        assert_eq!(answer, (true, Word(9)));
        assert_eq!(
            oracle.requests,
            [(
                FieldHintOp::Bn254BaseFieldInverse as u32,
                operand.as_ptr() as usize,
                operand.to_vec()
            )]
        );
        assert_eq!(oracle.tape.queries.len(), 1);
        assert_eq!(oracle.tape.queries[0].0, FIELD_OPS_ADVISE_QUERY_ID);
        assert!(oracle.tape.words.is_empty());
    }

    /// Querier memory over a buffer of words, at the addresses `base + 4 * i`
    struct WordsMemory {
        base: usize,
        words: Vec<u32>,
        word_size: usize,
    }

    impl QuerierMemory for WordsMemory {
        fn word_size(&self) -> usize {
            self.word_size
        }

        fn read_u32(&self, address: usize) -> Result<u32, HintError> {
            let offset = address.checked_sub(self.base).ok_or(HintError("below"))?;
            if !offset.is_multiple_of(4) {
                return Err(HintError("unaligned"));
            }
            self.words
                .get(offset / 4)
                .copied()
                .ok_or(HintError("above"))
        }
    }

    #[test]
    fn field_hint_request_is_read_for_either_querier_word_size() {
        // `FieldOpsHint`: op, src_ptr, src_len_u32_words
        let guest = WordsMemory {
            base: 0x1000,
            words: vec![FieldHintOp::Bls12381Fq12Inverse as u32, 0x2000, 144],
            word_size: 4,
        };
        assert_eq!(
            read_field_hint_request(&guest, 0x1000),
            Ok((FieldHintOp::Bls12381Fq12Inverse as u32, 0x2000, 144))
        );
        // `FieldOpsHint64`: op and padding, src_ptr (two words, low first), src_len_u32_words
        let native = WordsMemory {
            base: 0x1000,
            words: vec![
                FieldHintOp::Bn254G2PairingSlopes as u32,
                0,
                0x3000,
                0,
                32,
                0,
            ],
            word_size: 8,
        };
        assert_eq!(
            read_field_hint_request(&native, 0x1000),
            Ok((FieldHintOp::Bn254G2PairingSlopes as u32, 0x3000, 32))
        );
        let other = WordsMemory {
            base: 0x1000,
            words: vec![0; 6],
            word_size: 2,
        };
        assert_eq!(
            read_field_hint_request(&other, 0x1000),
            Err(HintError("unsupported querier word size"))
        );
        // a request at the top of the address space
        let top = WordsMemory {
            base: usize::MAX - 3,
            words: vec![0],
            word_size: 4,
        };
        assert_eq!(
            read_field_hint_request(&top, usize::MAX - 3),
            Err(HintError("querier address overflows"))
        );
    }
}
