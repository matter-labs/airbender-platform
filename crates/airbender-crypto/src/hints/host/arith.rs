//! The answers to the ModExp advice: the wide quotient of the single-digit path
//! (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`), and the quotient and remainder of the multi-digit path
//! (`MODEXP_ADVICE_QUERY_ID`). Operands are read through the querier memory of either target, and
//! both runs get the same answer.

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

/// The operand at `address` of the querier
fn read_operand<M: QuerierMemory + ?Sized>(memory: &M, address: usize) -> Result<Limbs, HintError> {
    let words = read_querier_u32_words(memory, address, OPERAND_WORDS)?;
    Ok(core::array::from_fn(|i| {
        u64::from(words[2 * i]) | (u64::from(words[2 * i + 1]) << 32)
    }))
}

/// The three operands whose addresses (querier words) are in the array at `input_word`
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

/// Appends the words of `limbs`, each limb low half first
fn write_operand(limbs: &Limbs, response: &mut Vec<u32>) {
    for limb in limbs {
        response.push(*limb as u32);
        response.push((*limb >> 32) as u32);
    }
}

/// Serves the wide division advice: the answer is the quotient.
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

/// Serves the multi-digit ModExp division advice: the request is a `ModExpAdviceParamsGeneric` of
/// querier words pointing to 256-bit digits. The answer is the lengths of the quotient and of the
/// remainder in `u32` words, then their words.
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

/// Serves the ModExp advice, appending the same answer to each response asked for
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
    use crate::hints::test_support::{GuestMemory, TestMemorySource};
    use crate::hints::{MODEXP_ADVICE_QUERY_ID, U256_WIDE_DIV_REM_ADVICE_QUERY_ID};
    use crate::modexp::{ModExpAdviceParams, ModExpAdviceParams64};
    use std::vec;
    use std::vec::Vec;

    /// The response to a guest query
    fn respond_to_guest(query_id: u32, input_word: usize, memory: &TestMemorySource) -> Vec<u32> {
        let mut response = Vec::new();
        // SAFETY: a guest querier is read through `GuestMemory` only
        let answered = unsafe {
            answer_query(
                query_id,
                input_word,
                &GuestMemory(memory),
                None,
                Some(&mut response),
            )
        };
        assert_eq!(answered, Ok(true));
        response
    }

    /// The native and guest responses to a native ModExp request, which must be the same
    fn respond_to_native(arg: &ModExpAdviceParams64) -> Vec<u32> {
        // SAFETY: the request and the operands it points to are alive during the call
        let memory = unsafe { NativeQuerierMemory::new() };
        let (mut native, mut guest) = (Vec::new(), Vec::new());
        // SAFETY: as above
        let answered = unsafe {
            answer_query(
                MODEXP_ADVICE_QUERY_ID,
                core::ptr::from_ref(arg).expose_provenance(),
                &memory,
                Some(&mut native),
                Some(&mut guest),
            )
        };
        assert_eq!(answered, Ok(true));
        assert_eq!(guest, native);
        native
    }

    impl TestMemorySource {
        fn insert_u64_words(&mut self, address: u32, values: &[u64]) {
            for (i, value) in values.iter().enumerate() {
                let address = address + 8 * i as u32;
                self.insert_u32(address, *value as u32);
                self.insert_u32(address + 4, (*value >> 32) as u32);
            }
        }

        /// A ModExp request at `address`, and its dividend and modulus after it
        fn insert_modexp_query(&mut self, address: u32, dividend: &[u64], modulus: &[u64]) {
            let (a_len, m_len) = (dividend.len().div_ceil(4), modulus.len().div_ceil(4));
            let a_ptr = address + 0x100;
            let modulus_ptr = a_ptr + 32 * a_len as u32;
            let params = ModExpAdviceParams {
                op: 0,
                a_ptr,
                a_len: a_len as u32,
                b_ptr: 0,
                b_len: 0,
                modulus_ptr,
                modulus_len: m_len as u32,
            };
            for (i, value) in [
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
                self.insert_u32(address + 4 * i as u32, value);
            }
            self.insert_u64_words(a_ptr, dividend);
            self.insert_u64_words(modulus_ptr, modulus);
        }
    }

    /// The quotient and the remainder of a ModExp advice response, as 64-bit limbs
    fn split_modexp_response(response: &[u32]) -> (Vec<u64>, Vec<u64>) {
        let (q_len, r_len) = (response[0] as usize, response[1] as usize);
        assert_eq!(response.len(), 2 + q_len + r_len);
        let limbs = |words: &[u32]| -> Vec<u64> {
            let (pairs, rest) = words.as_chunks::<2>();
            assert!(rest.is_empty());
            pairs
                .iter()
                .map(|[low, high]| u64::from(*low) | (u64::from(*high) << 32))
                .collect()
        };
        (
            limbs(&response[2..2 + q_len]),
            limbs(&response[2 + q_len..]),
        )
    }

    /// The quotient and the remainder of a guest ModExp query, without leading zero limbs
    fn guest_division(dividend: &[u64], modulus: &[u64]) -> (Vec<u64>, Vec<u64>) {
        let mut memory = TestMemorySource::default();
        memory.insert_modexp_query(0x100, dividend, modulus);
        split_modexp_response(&respond_to_guest(MODEXP_ADVICE_QUERY_ID, 0x100, &memory))
    }

    #[test]
    fn riscv_arithmetic_query_division() {
        let check = |dividend: &[u64], modulus: &[u64], q: &[u64], r: &[u64]| {
            assert_eq!(guest_division(dividend, modulus), (q.to_vec(), r.to_vec()));
        };
        check(&[10, 0, 0, 0], &[3, 0, 0, 0], &[3], &[1]);
        // zero limbs are stripped
        check(&[15, 0, 0, 0], &[5, 0, 0, 0], &[3], &[]);
        check(&[2, 0, 0, 0], &[7, 0, 0, 0], &[], &[2]);
        // a dividend of fewer digits than the modulus
        check(&[5, 0, 0, 0], &[3, 0, 0, 0, 1, 0, 0, 0], &[], &[5]);
        // 2^128 / 3
        check(
            &[0, 0, 1, 0],
            &[3, 0, 0, 0],
            &[0x5555_5555_5555_5555; 2],
            &[1],
        );
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn riscv_arithmetic_query_division_by_zero() {
        let _ = guest_division(&[10, 0, 0, 0], &[0, 0, 0, 0]);
    }

    #[test]
    fn u256_wide_div_rem_from_guest_memory() {
        // 2^256 / (2^128 + 1) = 2^128 - 1: the operands at 0x100, 0x200 and 0x300, and the
        // address array of the input at 0x400
        let mut memory = TestMemorySource::default();
        memory.insert_u64_words(0x100, &[0, 0, 0, 0]);
        memory.insert_u64_words(0x200, &[1, 0, 0, 0]);
        memory.insert_u64_words(0x300, &[1, 0, 1, 0]);
        for (i, address) in [0x100, 0x200, 0x300].into_iter().enumerate() {
            memory.insert_u32(0x400 + 4 * i as u32, address);
        }
        let response = respond_to_guest(U256_WIDE_DIV_REM_ADVICE_QUERY_ID, 0x400, &memory);
        // the 16 words of the quotient's halves
        let mut expected = vec![u32::MAX; 4];
        expected.extend([0; 12]);
        assert_eq!(response, expected);
    }

    /// A deterministic multi-digit operand with its top bit set
    fn patterned_u64_words(len: usize, mut state: u64) -> Vec<u64> {
        let mut words: Vec<u64> = (0..len)
            .map(|_| {
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                state.wrapping_mul(0x2545_F491_4F6C_DD1D)
            })
            .collect();
        words[len - 1] |= 1 << 63;
        words
    }

    #[test]
    fn native_and_riscv_arithmetic_queries_match_for_large_modexp_inputs() {
        let dividend = patterned_u64_words(12 * 4, 0x0123_4567_89AB_CDEF);
        let modulus = patterned_u64_words(8 * 4, 0x0FED_CBA9_8765_4321);
        let (q, r) = guest_division(&dividend, &modulus);
        assert!(
            q.len() > 1 && r.len() > 1,
            "multi-limb quotient and remainder"
        );

        let native = respond_to_native(&ModExpAdviceParams64 {
            op: 0,
            a_ptr: dividend.as_ptr().expose_provenance() as u64,
            a_len: 12,
            b_ptr: 0,
            b_len: 0,
            modulus_ptr: modulus.as_ptr().expose_provenance() as u64,
            modulus_len: 8,
        });
        assert_eq!(split_modexp_response(&native), (q, r));
    }

    #[test]
    #[should_panic(expected = "params.a_ptr > 0")]
    fn arithmetic_query_panics_on_null_request() {
        let _ = respond_to_guest(MODEXP_ADVICE_QUERY_ID, 0, &TestMemorySource::default());
    }

    #[test]
    #[should_panic(expected = "must read the modexp advice request")]
    fn arithmetic_query_panics_on_misaligned_pointer() {
        let _ = respond_to_guest(MODEXP_ADVICE_QUERY_ID, 0x101, &TestMemorySource::default());
    }
}
