//! What the oracle implementations share: the residue witnesses of the pairing checks
//! ([`residue_witness`], whose checks the querier runs too), and with `oracle_implementations`
//! the oracle side of the hints on a native host (`native`): the dispatch of a query to the
//! implementation of its primitive (`secp256k1::oracle_implementations`,
//! `bn254::oracle_implementations`, `bls12_381::oracle_implementations`,
//! `modexp::oracle_implementations`), the operands read from the querier's memory
//! ([`Operand`]), and the recorder of the hint tape of the guest ([`RecordingHintOracle`]).
//!
//! The oracle side reads native requests as `FieldOpsHint64` (and `ModExpAdviceParams64`), so it
//! is for 64-bit hosts. It answers a querier in this process (`HintTarget::Native`) and the RISC-V
//! guest with `bigint_ops` (`HintTarget::Guest`, the delegated representation of the fields).

pub mod residue_witness;

#[cfg(all(
    any(feature = "oracle_implementations", test),
    target_pointer_width = "64"
))]
mod native;
#[cfg(all(
    any(feature = "oracle_implementations", test),
    target_pointer_width = "64"
))]
pub use self::native::{
    answer_query, not_identity_claim, read_querier_u32_words, record_hints, NativeQuerierMemory,
    Operand, RecordingHintOracle, Responses,
};

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use airbender_oracle::{read_querier_word, QuerierMemory, QuerierMemoryError};
    use std::vec::Vec;

    #[test]
    fn native_querier_memory_reads_this_process() {
        let words = [0x0102_0304u32, 0xa0b0_c0d0, 7];
        // SAFETY: reads `words`, which is alive
        let memory = unsafe { NativeQuerierMemory::new() };
        let address = words.as_ptr().expose_provenance();
        assert_eq!(
            read_querier_u32_words(&memory, address, 3),
            Ok(words.to_vec())
        );
        // a querier word is two `u32`s, low first
        assert_eq!(
            read_querier_word(&memory, address),
            Ok(0xa0b0_c0d0_0102_0304)
        );
        for address in [0, address + 2] {
            assert_eq!(
                memory.read_u32(address),
                Err(QuerierMemoryError("invalid querier address"))
            );
        }
    }

    /// A querier memory of zeroes, which it does not dereference
    struct Zeroes;

    impl QuerierMemory for Zeroes {
        fn word_size(&self) -> usize {
            4
        }

        fn read_u32(&self, _address: usize) -> Result<u32, QuerierMemoryError> {
            Ok(0)
        }
    }

    #[test]
    fn querier_words_do_not_wrap_around() {
        assert_eq!(
            read_querier_u32_words(&Zeroes, usize::MAX - 3, 2),
            Err(crate::hints::HintError("querier address overflows"))
        );
    }

    #[test]
    fn unknown_query_ids_are_not_answered() {
        // SAFETY: nothing is read for an unknown query
        let memory = unsafe { NativeQuerierMemory::new() };
        let (mut native, mut guest) = (Vec::new(), Vec::new());
        // SAFETY: as above
        let answered = unsafe { answer_query(0, 0, &memory, Some(&mut native), Some(&mut guest)) };
        assert_eq!(answered, Ok(false));
        assert!(native.is_empty() && guest.is_empty());
    }
}
