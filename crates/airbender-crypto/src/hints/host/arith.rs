// Ported from zksync-os 185a0568:callable_oracles/src/arithmetic/mod.rs
// Ported from zksync-os 185a0568:zk_ee/src/oracle/memory_io/host.rs
// Ported from zksync-os 185a0568:oracle_provider/src/lib.rs
//! The answers to the arithmetic advice of ModExp: the quotient of the wide division of its
//! single-digit path (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`), and the quotient and the remainder of its
//! multi-digit path (`MODEXP_ADVICE_QUERY_ID`). The operands are read through the querier memory, from
//! the guest memory or from this process alike, and the answer is the same for both runs.

use super::read_querier_u32_words;
use crate::hints::{
    read_querier_word, HintError, QuerierMemory, MODEXP_ADVICE_QUERY_ID,
    U256_WIDE_DIV_REM_ADVICE_QUERY_ID,
};
use crate::modexp::read_modexp_advice_params;
use alloc::vec::Vec;

/// Limbs of a 256-bit operand, as the querier keeps them: 8 `u32` words, little-endian
type Limbs = [u64; 4];

/// Number of `u32` words in a 256-bit operand
const OPERAND_WORDS: usize = 8;

/// The operand at `address` of the querier (`read_continuous` of a `u256::U256`, whose validation is
/// the identity)
fn read_operand<M: QuerierMemory + ?Sized>(memory: &M, address: usize) -> Result<Limbs, HintError> {
    let words = read_querier_u32_words(memory, address, OPERAND_WORDS)?;
    Ok(core::array::from_fn(|i| {
        u64::from(words[2 * i]) | (u64::from(words[2 * i + 1]) << 32)
    }))
}

/// The three operands of a composite input (`ReadQueryInput` of a 3-tuple): the input word is the
/// address of the array of their addresses, querier words
fn read_three_operands<M: QuerierMemory + ?Sized>(
    memory: &M,
    input_word: usize,
) -> Result<(Limbs, Limbs, Limbs), HintError> {
    let word_size = memory.word_size();
    let element = |index: usize| {
        let address = index
            .checked_mul(word_size)
            .and_then(|offset| input_word.checked_add(offset))
            .ok_or(HintError("querier address overflows"))?;
        read_operand(memory, read_querier_word(memory, address)?)
    };
    Ok((element(0)?, element(1)?, element(2)?))
}

/// Appends the words of `limbs` as the querier receives them (`WriteQueryOutput` of a `u256::U256`):
/// each limb as its low and its high `u32`
fn write_operand(limbs: &Limbs, response: &mut Vec<u32>) {
    for limb in limbs {
        response.push(*limb as u32);
        response.push((*limb >> 32) as u32);
    }
}

/// Serves the U256 division advice (`u256_advice::U256WideDivRemAdviceQuery`): the operands are read
/// through the input word, from the guest memory or from this process alike, and the answer is the
/// quotient.
fn process_u256_advice_query(
    query_id: u32,
    input_word: usize,
    memory: &dyn QuerierMemory,
) -> Vec<u32> {
    let mut response = Vec::new();
    match query_id {
        U256_WIDE_DIV_REM_ADVICE_QUERY_ID => {
            let (dividend_lo, dividend_hi, mut divisor) = read_three_operands(memory, input_word)
                .expect("must read the wide div_rem operands");
            let mut quotient = [0u64; 8];
            quotient[..4].copy_from_slice(&dividend_lo);
            quotient[4..].copy_from_slice(&dividend_hi);
            ruint::algorithms::div(&mut quotient, &mut divisor);
            write_operand(&quotient[..4].try_into().unwrap(), &mut response);
            write_operand(&quotient[4..].try_into().unwrap(), &mut response);
        }
        _ => unreachable!("not a U256 advice query: 0x{query_id:08x}"),
    }
    response
}

/// Serves the modexp division advice (`modexp::advice::bigint::OracleAdvisor`): the request, a
/// `ModExpAdviceParamsGeneric` of querier words, and the operands it points to (256-bit digits) are
/// read through the input word, from the guest memory or from this process alike. The answer is the
/// lengths of the quotient and of the remainder in `u32` words, then their words.
fn process_modexp_query(input_word: usize, memory: &dyn QuerierMemory) -> Vec<u32> {
    let params =
        read_modexp_advice_params(memory, input_word).expect("must read the modexp advice request");
    assert!(params.a_ptr > 0);
    assert!(params.a_len > 0);
    assert_eq!(params.b_ptr, 0);
    assert_eq!(params.b_len, 0);
    assert!(params.modulus_ptr > 0);
    assert!(params.modulus_len > 0);

    // a 256-bit digit is 8 words
    let read_digits = |address: usize, digits: usize| -> Vec<u64> {
        let num_words = digits.checked_mul(8).expect("operand length overflow");
        read_querier_u32_words(memory, address, num_words)
            .expect("must read the modexp operand")
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[low, high]| u64::from(*low) | (u64::from(*high) << 32))
            .collect()
    };
    let mut n = read_digits(params.a_ptr, params.a_len);
    let mut d = read_digits(params.modulus_ptr, params.modulus_len);

    ruint::algorithms::div(&mut n, &mut d);

    // without the leading zero limbs
    fn significant(limbs: &[u64]) -> &[u64] {
        let zeroes = limbs.iter().rev().take_while(|limb| **limb == 0).count();
        &limbs[..limbs.len() - zeroes]
    }
    let (quotient, remainder) = (significant(&n), significant(&d));
    let mut response = Vec::with_capacity(2 + 2 * (quotient.len() + remainder.len()));
    for limbs in [quotient, remainder] {
        response.push(u32::try_from(2 * limbs.len()).expect("the modexp advice is too long"));
    }
    for limb in quotient.iter().chain(remainder) {
        response.push(*limb as u32);
        response.push((*limb >> 32) as u32);
    }
    response
}

fn process_arithmetic_query(
    query_id: u32,
    input_word: usize,
    memory: &dyn QuerierMemory,
) -> Vec<u32> {
    match query_id {
        MODEXP_ADVICE_QUERY_ID => process_modexp_query(input_word, memory),
        _ => process_u256_advice_query(query_id, input_word, memory),
    }
}

/// Serves the arithmetic advice of a querier (`ArithmeticQuery` and `NativeArithmeticQuery`, which
/// are the same processor, as the querier memory abstracts the difference), and appends the answer
/// to the responses asked for (`oracle_provider::respond_to_every_target`): the same words for both
/// runs.
pub(super) fn process_memory_query(
    query_id: u32,
    input_word: usize,
    memory: &dyn QuerierMemory,
    native_run_responses: Option<&mut Vec<u32>>,
    guest_run_responses: Option<&mut Vec<u32>>,
) {
    let mut response = process_arithmetic_query(query_id, input_word, memory);
    match (native_run_responses, guest_run_responses) {
        (Some(native_run_responses), None) => native_run_responses.append(&mut response),
        (Some(native_run_responses), Some(guest_run_responses)) => {
            guest_run_responses.extend_from_slice(&response);
            native_run_responses.append(&mut response);
        }
        (None, Some(guest_run_responses)) => guest_run_responses.append(&mut response),
        (None, None) => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::hints::host::{answer_query, NativeQuerierMemory};
    use crate::hints::test_support::{GuestMemory, InProcessMemoryOracle, TestMemorySource};
    use crate::hints::wide_div::query_wide_div_rem_hint;
    use crate::hints::{QuerierMemory, MODEXP_ADVICE_QUERY_ID, U256_WIDE_DIV_REM_ADVICE_QUERY_ID};
    use crate::modexp::u256::U256;
    use crate::modexp::{ModExpAdviceParams, ModExpAdviceParams64};
    use std::vec;
    use std::vec::Vec;

    /// The response of the processor to its querier: the RISC-V guest for guest memory, one in this
    /// process otherwise
    ///
    /// # Safety
    ///
    /// For a querier in this process, `memory` is the memory of this process, and `input_word` the
    /// address of a live request whose operand pointers reference live values
    unsafe fn respond(query_id: u32, input_word: usize, memory: &dyn QuerierMemory) -> Vec<u32> {
        let guest = memory.word_size() == size_of::<u32>();
        let (mut native_run, mut guest_run) = (Vec::new(), Vec::new());
        // SAFETY: guaranteed by the caller
        let answered = unsafe {
            if guest {
                answer_query(query_id, input_word, memory, None, Some(&mut guest_run))
            } else {
                answer_query(query_id, input_word, memory, Some(&mut native_run), None)
            }
        };
        assert_eq!(answered, Ok(true), "the arithmetic queries are answered");
        if guest {
            assert!(native_run.is_empty());
            guest_run
        } else {
            assert!(guest_run.is_empty());
            native_run
        }
    }

    /// The response to the querier on the guest of `memory`
    fn respond_to_guest(query_id: u32, input_word: usize, memory: &TestMemorySource) -> Vec<u32> {
        // SAFETY: the querier is on the guest, its memory is read through `GuestMemory` only
        unsafe { respond(query_id, input_word, &GuestMemory(memory)) }
    }

    impl TestMemorySource {
        fn insert_u64_words(&mut self, address: u32, values: &[u64]) {
            for (idx, value) in values.iter().copied().enumerate() {
                let word_address = address + (idx as u32) * 8;
                self.insert_u32(word_address, value as u32);
                self.insert_u32(word_address + 4, (value >> 32) as u32);
            }
        }

        fn insert_modexp_params(&mut self, address: u32, params: ModExpAdviceParams) {
            for (idx, value) in [
                params.op,
                params.a_ptr,
                params.a_len,
                params.b_ptr,
                params.b_len,
                params.modulus_ptr,
                params.modulus_len,
            ]
            .into_iter()
            .enumerate()
            {
                self.insert_u32(address + (idx as u32) * 4, value);
            }
        }
    }

    fn patterned_u64_words(len: usize, seed: u64) -> Vec<u64> {
        let mut state = seed;
        let mut words = Vec::with_capacity(len);
        for idx in 0..len {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let mut word = state
                .wrapping_mul(0x2545_F491_4F6C_DD1D)
                .wrapping_add(idx as u64);
            if word == 0 {
                word = seed.wrapping_add(idx as u64 + 1);
            }
            words.push(word);
        }

        words[0] |= 1;
        *words.last_mut().expect("large input must be non-empty") |= 1 << 63;

        words
    }

    /// Helper: write a ModExpAdviceParams struct and data into TestMemorySource, then run the
    /// oracle query processor. Returns (quotient_u64_words, remainder_u64_words).
    fn run_division_query(dividend_u64: &[u64], modulus_u64: &[u64]) -> (Vec<u64>, Vec<u64>) {
        let a_digits = dividend_u64.len().div_ceil(4);
        let m_digits = modulus_u64.len().div_ceil(4);
        let a_u64_count = a_digits * 4;

        const PARAMS_ADDR: u32 = 0x100;
        const A_ADDR: u32 = 0x200;
        let m_addr: u32 = A_ADDR + (a_u64_count as u32) * 8;

        let mut memory = TestMemorySource::default();
        memory.insert_modexp_params(
            PARAMS_ADDR,
            ModExpAdviceParams {
                op: 0,
                a_ptr: A_ADDR,
                a_len: a_digits as u32,
                b_ptr: 0,
                b_len: 0,
                modulus_ptr: m_addr,
                modulus_len: m_digits as u32,
            },
        );
        memory.insert_u64_words(A_ADDR, dividend_u64);
        memory.insert_u64_words(m_addr, modulus_u64);

        let response = respond_to_guest(MODEXP_ADVICE_QUERY_ID, PARAMS_ADDR as usize, &memory);
        split_modexp_response(&response)
    }

    /// The quotient and the remainder of a modexp advice response, as 64-bit limbs
    fn split_modexp_response(response: &[u32]) -> (Vec<u64>, Vec<u64>) {
        let (q_len, r_len) = (response[0] as usize, response[1] as usize);
        assert_eq!(response.len(), 2 + q_len + r_len);
        let limbs = |words: &[u32]| -> Vec<u64> {
            assert!(words.len().is_multiple_of(2));
            words
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[low, high]| u64::from(*low) | (u64::from(*high) << 32))
                .collect()
        };
        (
            limbs(&response[2..2 + q_len]),
            limbs(&response[2 + q_len..]),
        )
    }

    #[test]
    fn riscv_arithmetic_query_basic_division() {
        // 10 / 3 = q=3, r=1
        let (q, r) = run_division_query(&[10, 0, 0, 0], &[3, 0, 0, 0]);
        assert_eq!(q, vec![3]);
        assert_eq!(r, vec![1]);
    }

    #[test]
    fn riscv_arithmetic_query_exact_division() {
        // 15 / 5 = q=3, r=0
        let (q, r) = run_division_query(&[15, 0, 0, 0], &[5, 0, 0, 0]);
        assert_eq!(q, vec![3]);
        assert!(r.is_empty(), "remainder should be zero (stripped)");
    }

    #[test]
    fn riscv_arithmetic_query_dividend_smaller_than_modulus() {
        // 2 / 7 = q=0, r=2
        let (q, r) = run_division_query(&[2, 0, 0, 0], &[7, 0, 0, 0]);
        assert!(q.is_empty(), "quotient should be zero (stripped)");
        assert_eq!(r, vec![2]);
    }

    #[test]
    fn riscv_arithmetic_query_dividend_fewer_digits_than_modulus() {
        // a=5 (1 DelegatedU256 digit), m=2^64+3 (2 DelegatedU256 digits)
        // 5 < 2^64+3, so q=0, r=5
        let (q, r) = run_division_query(&[5, 0, 0, 0], &[3, 0, 0, 0, 1, 0, 0, 0]);
        assert!(q.is_empty(), "quotient should be zero (stripped)");
        assert_eq!(r, vec![5]);
    }

    #[test]
    fn riscv_arithmetic_query_multi_digit_quotient() {
        // 2^128 / 3 = 0x55555555555555555555555555555555 remainder 1
        let (q, r) = run_division_query(&[0, 0, 1, 0], &[3, 0, 0, 0]);
        assert_eq!(q, vec![0x5555555555555555, 0x5555555555555555]);
        assert_eq!(r, vec![1]);
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn riscv_arithmetic_query_division_by_zero() {
        let _ = run_division_query(&[10, 0, 0, 0], &[0, 0, 0, 0]);
    }

    /// Runs the modexp advice query of a querier of this process with the request `arg`
    fn run_native_division_query(arg: &ModExpAdviceParams64) -> Vec<u32> {
        // SAFETY: the request and the operands it points to are alive during the call
        let memory = unsafe { NativeQuerierMemory::new() };
        // SAFETY: as above, and they are read through `memory` only
        unsafe {
            respond(
                MODEXP_ADVICE_QUERY_ID,
                core::ptr::from_ref(arg).expose_provenance(),
                &memory,
            )
        }
    }

    #[test]
    fn native_arithmetic_query_processes_valid_query() {
        let dividend = [10u64, 0, 0, 0];
        let modulus = [3u64, 0, 0, 0];
        let arg = ModExpAdviceParams64 {
            op: 0,
            a_ptr: dividend.as_ptr().expose_provenance() as u64,
            a_len: 1,
            b_ptr: 0,
            b_len: 0,
            modulus_ptr: modulus.as_ptr().expose_provenance() as u64,
            modulus_len: 1,
        };

        // lengths in words, then q = 3 and r = 1
        assert_eq!(run_native_division_query(&arg), vec![2, 2, 3, 0, 1, 0]);
    }

    /// An oracle of this process that answers with the arithmetic processor (zksync-os's
    /// `ZkEENonDeterminismSource` with `NativeArithmeticQuery`, a native run)
    fn native_oracle(
    ) -> InProcessMemoryOracle<impl FnMut(u32, usize, &dyn QuerierMemory) -> Vec<u32>> {
        InProcessMemoryOracle::new(
            |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                let mut response = Vec::new();
                // SAFETY: the handler of an `InProcessMemoryOracle` runs during the query, with the
                // memory of this process, where the request and its operands are live
                let answered = unsafe {
                    answer_query(query_id, input_word, memory, Some(&mut response), None)
                };
                assert_eq!(answered, Ok(true));
                response
            },
        )
    }

    #[test]
    fn u256_wide_div_rem_via_native_query() {
        // 35 / 6: q=5
        let dividend_lo = U256::from_limbs([35, 0, 0, 0]);
        let dividend_hi = U256::from_limbs([0, 0, 0, 0]);
        let divisor = U256::from_limbs([6, 0, 0, 0]);
        let (q_lo, q_hi) =
            query_wide_div_rem_hint(&dividend_lo, &dividend_hi, &divisor, &mut native_oracle());
        assert_eq!(*q_lo.as_limbs(), [5, 0, 0, 0]);
        assert_eq!(*q_hi.as_limbs(), [0, 0, 0, 0]);
    }

    #[test]
    fn u256_wide_div_rem_large_dividend() {
        // 2^256 / (2^128 + 1): q = 2^128 - 1
        let dividend_lo = U256::from_limbs([0, 0, 0, 0]);
        let dividend_hi = U256::from_limbs([1, 0, 0, 0]);
        let divisor = U256::from_limbs([1, 0, 1, 0]);
        let (q_lo, q_hi) =
            query_wide_div_rem_hint(&dividend_lo, &dividend_hi, &divisor, &mut native_oracle());
        assert_eq!(*q_lo.as_limbs(), [u64::MAX, u64::MAX, 0, 0]);
        assert_eq!(*q_hi.as_limbs(), [0, 0, 0, 0]);
    }

    #[test]
    fn u256_wide_div_rem_from_guest_memory() {
        // 2^256 / (2^128 + 1): q = 2^128 - 1, operands at 0x100, 0x200 and 0x300 of a 32-bit guest,
        // and the address array of the composite input at 0x400
        let mut memory = TestMemorySource::default();
        memory.insert_u64_words(0x100, &[0, 0, 0, 0]);
        memory.insert_u64_words(0x200, &[1, 0, 0, 0]);
        memory.insert_u64_words(0x300, &[1, 0, 1, 0]);
        for (i, address) in [0x100, 0x200, 0x300].into_iter().enumerate() {
            memory.insert_u32(0x400 + 4 * i as u32, address);
        }
        let response = respond_to_guest(U256_WIDE_DIV_REM_ADVICE_QUERY_ID, 0x400, &memory);
        // the two halves of the quotient, as the 16 words the guest reads: 2^128 - 1 fills the low 4
        let mut expected = vec![u32::MAX; 4];
        expected.extend([0; 12]);
        assert_eq!(response, expected);
    }

    #[test]
    fn native_and_riscv_arithmetic_queries_match_for_large_modexp_inputs() {
        const DIVIDEND_DIGITS: usize = 12;
        const MODULUS_DIGITS: usize = 8;
        const GUEST_ARG_ADDR: u32 = 0x1000;
        const GUEST_DIVIDEND_ADDR: u32 = 0x2000;
        const GUEST_MODULUS_ADDR: u32 = 0x4000;

        let mut dividend = patterned_u64_words(DIVIDEND_DIGITS * 4, 0x0123_4567_89AB_CDEF);
        let mut modulus = patterned_u64_words(MODULUS_DIGITS * 4, 0x0FED_CBA9_8765_4321);

        let mut memory = TestMemorySource::default();
        memory.insert_modexp_params(
            GUEST_ARG_ADDR,
            ModExpAdviceParams {
                op: 0,
                a_ptr: GUEST_DIVIDEND_ADDR,
                a_len: DIVIDEND_DIGITS as u32,
                b_ptr: 0,
                b_len: 0,
                modulus_ptr: GUEST_MODULUS_ADDR,
                modulus_len: MODULUS_DIGITS as u32,
            },
        );
        memory.insert_u64_words(GUEST_DIVIDEND_ADDR, &dividend);
        memory.insert_u64_words(GUEST_MODULUS_ADDR, &modulus);

        let riscv_output =
            respond_to_guest(MODEXP_ADVICE_QUERY_ID, GUEST_ARG_ADDR as usize, &memory);

        let host_arg = ModExpAdviceParams64 {
            op: 0,
            a_ptr: dividend.as_mut_ptr().expose_provenance() as u64,
            a_len: DIVIDEND_DIGITS as u64,
            b_ptr: 0,
            b_len: 0,
            modulus_ptr: modulus.as_mut_ptr().expose_provenance() as u64,
            modulus_len: MODULUS_DIGITS as u64,
        };
        let native_output = run_native_division_query(&host_arg);

        assert_eq!(native_output, riscv_output);

        let (quotient, remainder) = split_modexp_response(&native_output);
        assert!(
            quotient.len() > 1,
            "quotient should span multiple u64 limbs"
        );
        assert!(
            remainder.len() > 1,
            "remainder should span multiple u64 limbs"
        );
    }

    #[test]
    #[should_panic(expected = "params.a_ptr > 0")]
    fn arithmetic_query_panics_on_null_request() {
        let memory = TestMemorySource::default();
        let _ = respond_to_guest(MODEXP_ADVICE_QUERY_ID, 0, &memory);
    }

    #[test]
    #[should_panic(expected = "must read the modexp advice request")]
    fn arithmetic_query_panics_on_misaligned_pointer() {
        let memory = TestMemorySource::default();
        let _ = respond_to_guest(MODEXP_ADVICE_QUERY_ID, 0x101, &memory);
    }

    #[test]
    #[should_panic(expected = "must read the modexp advice request")]
    fn native_arithmetic_query_rejects_null_query_pointer() {
        // SAFETY: nothing is read at a null address
        let memory = unsafe { NativeQuerierMemory::new() };
        // SAFETY: as above
        let _ = unsafe { respond(MODEXP_ADVICE_QUERY_ID, 0, &memory) };
    }

    /// A native run that records the prover input answers both runs with the same words
    #[test]
    fn both_runs_get_the_same_answer() {
        let dividend = [10u64, 0, 0, 0];
        let modulus = [3u64, 0, 0, 0];
        let arg = ModExpAdviceParams64 {
            op: 0,
            a_ptr: dividend.as_ptr().expose_provenance() as u64,
            a_len: 1,
            b_ptr: 0,
            b_len: 0,
            modulus_ptr: modulus.as_ptr().expose_provenance() as u64,
            modulus_len: 1,
        };
        // SAFETY: the request and the operands it points to are alive during the call
        let memory = unsafe { NativeQuerierMemory::new() };
        let (mut native_run, mut guest_run) = (Vec::new(), Vec::new());
        // SAFETY: as above, and they are read through `memory` only
        let answered = unsafe {
            answer_query(
                MODEXP_ADVICE_QUERY_ID,
                core::ptr::from_ref(&arg).expose_provenance(),
                &memory,
                Some(&mut native_run),
                Some(&mut guest_run),
            )
        };
        assert_eq!(answered, Ok(true));
        assert_eq!(native_run, vec![2, 2, 3, 0, 1, 0]);
        assert_eq!(guest_run, native_run);
    }
}
