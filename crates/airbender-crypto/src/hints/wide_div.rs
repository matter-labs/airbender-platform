// Ported from zksync-os 185a0568:basic_system/src/system_functions/u256_advice.rs
// Ported from zksync-os 185a0568:zk_ee/src/oracle/memory_io/query.rs
// Ported from zksync-os 185a0568:zk_ee/src/oracle/memory_io/composite.rs
// Ported from zksync-os 185a0568:zk_ee/src/oracle/memory_io/continuous.rs
// Ported from zksync-os 185a0568:zk_ee/src/oracle/memory_io/oracle.rs
//! The wide division advice (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`) of the single-digit modulus path
//! of ModExp: the quotient of a 512-bit dividend by a 256-bit divisor, and its checks.
//!
//! The query is zksync-os's `U256WideDivRemAdviceQuery`: a composite input, the address of the
//! array of the addresses of the dividend's low half, its high half and the divisor (`[usize; 3]`),
//! and a composite output, the low and the high halves of the quotient, 8 words each.

use super::{HintError, HintOracle, U256_WIDE_DIV_REM_ADVICE_QUERY_ID};
use crate::modexp::u256::U256;
use core::mem::{align_of, size_of, MaybeUninit};

/// Advice for `(dividend_hi * 2^256 + dividend_lo) / divisor` (operands passed by address): the low and
/// the high halves of the quotient.
pub(crate) struct U256WideDivRemAdviceQuery;

impl U256WideDivRemAdviceQuery {
    const QUERY_ID: u32 = U256_WIDE_DIV_REM_ADVICE_QUERY_ID;

    /// Runs the query, and returns the output as a new value (`OracleQuery::get` with the input
    /// `(dividend_lo, dividend_hi, divisor)` and the output `(U256, U256)`).
    #[inline(always)]
    fn get<O: HintOracle>(
        oracle: &mut O,
        input: (&U256, &U256, &U256),
    ) -> Result<(U256, U256), HintError> {
        send_composite(oracle, Self::QUERY_ID, input)?;
        let output = receive_value(oracle);
        finish(oracle, output)
    }
}

/// Number of `u32` words in a `U256` (`continuous_words`), which the oracle exchanges as whole
/// aligned words
const U256_WORDS: usize = {
    assert!(
        size_of::<U256>() > 0,
        "oracle values must not be zero-sized"
    );
    assert!(
        size_of::<U256>().is_multiple_of(size_of::<u32>()),
        "oracle values must consist of whole u32 words"
    );
    assert!(
        align_of::<U256>() >= align_of::<u32>(),
        "oracle values must be aligned for u32"
    );
    size_of::<U256>() / size_of::<u32>()
};

/// The address to give to the oracle for `value`. The provenance is exposed, as the oracle reads the value
/// through this address.
#[inline(always)]
fn exposed_address(value: &U256) -> usize {
    core::ptr::from_ref(value).expose_provenance()
}

/// Starts a query with a composite input, passed by the address of the array of element addresses
/// (`QueryInput::send` of a tuple: `send_composite` and `CompositeSerializable::with_address`).
#[inline(always)]
fn send_composite<O: HintOracle>(
    oracle: &mut O,
    query_id: u32,
    input: (&U256, &U256, &U256),
) -> Result<(), HintError> {
    let addresses = [
        exposed_address(input.0),
        exposed_address(input.1),
        exposed_address(input.2),
    ];
    // SAFETY: the input word is the address of the array of the addresses of the dividend's low
    // half, its high half and the divisor, the request of the wide division query, and the array
    // and the operands are alive during the call
    unsafe {
        oracle.send_query(
            query_id,
            core::ptr::from_ref(&addresses).expose_provenance(),
        )
    }
}

/// Receives the output as a new value (`QueryOutput::receive_value` of a tuple): writes and validates
/// each element in turn (`init_composite`).
#[inline(always)]
fn receive_value<O: HintOracle>(oracle: &mut O) -> Result<(U256, U256), HintError> {
    let mut value = MaybeUninit::<(U256, U256)>::uninit();
    let this = value.as_mut_ptr();
    // SAFETY: the elements are disjoint places inside `value`, and `MaybeUninit<E>` has the
    // layout of `E`
    let dst = unsafe {
        (
            &mut *(&raw mut (*this).0).cast::<MaybeUninit<U256>>(),
            &mut *(&raw mut (*this).1).cast::<MaybeUninit<U256>>(),
        )
    };
    // tuple expressions are evaluated left to right, i.e. in the order of the response
    let _ = (init(oracle, dst.0)?, init(oracle, dst.1)?);
    // SAFETY: every element was initialized and validated above
    Ok(unsafe { value.assume_init() })
}

/// Writes the next value of the response to `dst`, and validates it in place (`MemoryOracle::init`).
#[inline(always)]
fn init<'a, O: HintOracle>(
    oracle: &mut O,
    dst: &'a mut MaybeUninit<U256>,
) -> Result<&'a mut U256, HintError> {
    let this = dst.as_mut_ptr();
    // SAFETY: `dst` is aligned and valid for writes of `U256`, which is aligned for `u32` and consists of
    // `U256_WORDS` words
    unsafe { oracle.write_words(this.cast::<u32>(), U256_WORDS)? };
    // SAFETY: all bytes of `dst` were just written, any bytes form a valid `U256` (its validation is the
    // identity), and `dst` is exclusively borrowed for `'a`
    Ok(unsafe { &mut *this })
}

/// Ends the current query, whether its output was received or rejected: the oracle drops the words of
/// a rejected output that were not read, and stays in step with the querier (as the proving target's
/// oracle does when the next query starts).
#[inline(always)]
fn finish<O: HintOracle, T>(oracle: &mut O, output: Result<T, HintError>) -> Result<T, HintError> {
    let finished = oracle.finish_query();
    let output = output?;
    finished?;
    Ok(output)
}

/// Verifies a wide div_rem hint. On success, `dividend_lo` is modified in-place
/// to hold the remainder. `dividend_hi` is also modified.
#[must_use]
pub(crate) fn verify_wide_div_rem_hint(
    dividend_lo: &mut U256,
    dividend_hi: &mut U256,
    divisor: &U256,
    q_lo: &U256,
    q_hi: &U256,
) -> bool {
    // Compute q * divisor as 512-bit
    let mut qd_lo = q_lo.clone();
    let mut qd_mid = q_lo.clone();
    qd_lo.widening_mul_assign_into(&mut qd_mid, divisor);

    let mut qd_hi_lo = q_hi.clone();
    let mut qd_hi_hi = q_hi.clone();
    qd_hi_lo.widening_mul_assign_into(&mut qd_hi_hi, divisor);

    // Accumulate into (qd_lo, qd_mid) — the 512-bit product q*d
    let c1 = qd_mid.overflowing_add_assign(&qd_hi_lo);
    if c1 || !qd_hi_hi.is_zero() {
        return false;
    }

    // Compute r = dividend - q*d in-place (512-bit subtraction)
    let borrow_lo = dividend_lo.overflowing_sub_assign(&qd_lo);
    let borrow_mid = dividend_hi.overflowing_sub_assign(&qd_mid);
    let borrow_final = if borrow_lo {
        dividend_hi.overflowing_sub_assign(&U256::one())
    } else {
        false
    };

    // r must be non-negative (no borrow) and fit in 256 bits (r_hi == 0)
    if (borrow_mid | borrow_final) || !dividend_hi.is_zero() {
        return false;
    }

    *dividend_lo < *divisor
}

/// Asks the oracle for `(dividend_hi * 2^256 + dividend_lo) / divisor`; the answer is the quotient, low
/// half first.
#[inline(always)]
pub(crate) fn query_wide_div_rem_hint<O: HintOracle>(
    dividend_lo: &U256,
    dividend_hi: &U256,
    divisor: &U256,
    oracle: &mut O,
) -> (U256, U256) {
    U256WideDivRemAdviceQuery::get(oracle, (dividend_lo, dividend_hi, divisor))
        .expect("wide_div_rem oracle query failed")
}

/// Reduces the 512-bit product `(product_lo, product_hi)` modulo `modulus` with its advised
/// quotient `(q_lo, q_hi)`: the remainder `product - q * modulus` is formed in `product_lo`
/// and checked to be in `[0, modulus)`. Panics on a wrong quotient. `product_hi` is
/// consumed as scratch.
#[cfg(test)]
#[inline]
pub(crate) fn reduce_product_with_quotient(
    product_lo: &mut U256,
    product_hi: &mut U256,
    modulus: &U256,
    mut q_lo: U256,
    mut q_hi: U256,
) {
    // q * modulus as 512 bits: q_lo * m = (qd_lo, qd_mid), q_hi * m must fit in 256 bits
    let qd_lo = &mut q_lo;
    let mut qd_mid = qd_lo.clone();
    qd_lo.widening_mul_assign_into(&mut qd_mid, modulus);
    let qd_hi_lo = &mut q_hi;
    let mut qd_hi_hi = qd_hi_lo.clone();
    qd_hi_lo.widening_mul_assign_into(&mut qd_hi_hi, modulus);
    let carry = qd_mid.overflowing_add_assign(qd_hi_lo);
    assert!(
        !carry && qd_hi_hi.is_zero(),
        "mulmod hint: quotient too large"
    );

    // remainder = product - q * modulus, formed in the low half; must not borrow and
    // must fit in 256 bits
    let borrow_lo = product_lo.overflowing_sub_assign(qd_lo);
    let borrow_mid = product_hi.overflowing_sub_assign_with_borrow_propagation(&qd_mid, borrow_lo);
    assert!(
        !borrow_mid && product_hi.is_zero(),
        "mulmod hint: remainder out of range"
    );

    // remainder < modulus: a subtraction on scratch must borrow
    let mut check = product_lo.clone();
    let borrow = check.overflowing_sub_assign(modulus);
    assert!(borrow, "mulmod hint: remainder not reduced");
}

/// Copies `x` into `slot` and returns the copy. On the proving target `x.clone()` copies into a
/// temporary and then moves it to its place (8 loads and 8 stores), as LLVM can't merge the
/// temporary the copy delegation writes with the destination.
#[inline(always)]
pub(crate) fn clone_into_slot<'a>(slot: &'a mut MaybeUninit<U256>, x: &U256) -> &'a mut U256 {
    // SAFETY: `slot` is an exclusively borrowed `U256` place, so it is aligned (by type) and
    // disjoint from `x`, and the copy initialises all of it
    unsafe {
        U256::write_into_ptr_unchecked(slot.as_mut_ptr(), x);
        slot.assume_init_mut()
    }
}

/// Reduces the 512-bit product `(product_lo, product_hi) = x * y` of two residues
/// `x, y < modulus` with its advised quotient `(q_lo, q_hi)`, like `reduce_product_with_quotient`
/// but with one multiplication by the modulus: the quotient is at most
/// `(modulus - 1)^2 / modulus < modulus`, so `q_hi` must be zero and `q_lo * modulus` is the whole
/// of `q * modulus`. For such a product this accepts exactly the quotient the general check
/// accepts (the right one) and leaves the same remainder in `product_lo`. For operands out of
/// range it may reject the right quotient, but never accepts a wrong one. Panics on a wrong
/// quotient. `product_hi` is consumed as scratch.
#[inline]
pub(crate) fn reduce_product_of_residues_with_quotient(
    product_lo: &mut U256,
    product_hi: &mut U256,
    modulus: &U256,
    mut q_lo: U256,
    q_hi: U256,
) {
    // the high half is received like the low half (same query, same words), and must be zero
    assert!(q_hi.is_zero(), "mulmod hint: quotient too large");

    // q * modulus = q_lo * modulus = (qd_lo, qd_mid), exact in 512 bits
    let mut qd_mid = MaybeUninit::uninit();
    let qd_mid = clone_into_slot(&mut qd_mid, &q_lo);
    let qd_lo = &mut q_lo;
    qd_lo.widening_mul_assign_into(qd_mid, modulus);

    // remainder = product - q * modulus, formed in the low half; must not borrow and must fit
    // in 256 bits
    let borrow_lo = product_lo.overflowing_sub_assign(qd_lo);
    let borrow_mid = product_hi.overflowing_sub_assign_with_borrow_propagation(qd_mid, borrow_lo);
    assert!(
        !borrow_mid && product_hi.is_zero(),
        "mulmod hint: remainder out of range"
    );

    // remainder < modulus: a subtraction on scratch must borrow
    let mut check = MaybeUninit::uninit();
    let check = clone_into_slot(&mut check, product_lo);
    let borrow = check.overflowing_sub_assign(modulus);
    assert!(borrow, "mulmod hint: remainder not reduced");
}

/// zksync-os's `ReadQueryInput` and `WriteQueryOutput` (`zk_ee/src/oracle/memory_io/host.rs`) for the
/// input and the output of the wide division query, for tests: the operands read where the querier
/// keeps them, and the words of the answer as the querier receives them
#[cfg(all(test, target_pointer_width = "64"))]
pub(crate) mod test_io {
    use super::U256;
    use crate::hints::{read_querier_u32_words, read_querier_word, HintError, QuerierMemory};
    use std::vec::Vec;

    /// Oracle side of the input of a query: reads it from the memory of the querier, the way the
    /// querier sends it.
    pub(crate) trait ReadQueryInput: Sized {
        fn read_input<M: QuerierMemory + ?Sized>(
            memory: &M,
            input_word: usize,
        ) -> Result<Self, HintError>;
    }

    /// Oracle side of the output of a query: encodes it the way the querier receives it.
    pub(crate) trait WriteQueryOutput {
        fn write_output(&self, response: &mut Vec<u32>);
    }

    /// The `U256` at `address` of the querier (`read_continuous`)
    fn read_continuous<M: QuerierMemory + ?Sized>(
        memory: &M,
        address: usize,
    ) -> Result<U256, HintError> {
        let words = read_querier_u32_words(memory, address, 8)?;
        Ok(U256::from_limbs(core::array::from_fn(|i| {
            u64::from(words[2 * i]) | (u64::from(words[2 * i + 1]) << 32)
        })))
    }

    impl ReadQueryInput for (U256, U256, U256) {
        fn read_input<M: QuerierMemory + ?Sized>(
            memory: &M,
            input_word: usize,
        ) -> Result<Self, HintError> {
            let word_size = memory.word_size();
            let element = |index: usize| {
                let address = input_word
                    .checked_add(index * word_size)
                    .ok_or(HintError("querier address overflows"))?;
                read_continuous(memory, read_querier_word(memory, address)?)
            };
            Ok((element(0)?, element(1)?, element(2)?))
        }
    }

    impl WriteQueryOutput for U256 {
        fn write_output(&self, response: &mut Vec<u32>) {
            for limb in self.as_limbs() {
                response.push(*limb as u32);
                response.push((*limb >> 32) as u32);
            }
        }
    }

    impl WriteQueryOutput for (U256, U256) {
        fn write_output(&self, response: &mut Vec<u32>) {
            self.0.write_output(response);
            self.1.write_output(response);
        }
    }
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::test_io::{ReadQueryInput, WriteQueryOutput};
    use super::*;
    use crate::hints::test_support::InProcessMemoryOracle;
    use crate::hints::QuerierMemory;
    use ruint::aliases::{U256 as HostU256, U512};
    use std::cell::Cell;
    use std::rc::Rc;

    fn host(value: &U256) -> HostU256 {
        value.clone().into()
    }

    /// Answers the division query the way the host does: reads the operands through the input word
    /// and returns the quotient.
    fn host_division(query_id: u32, input_word: usize, memory: &dyn QuerierMemory) -> Vec<u32> {
        let mut response = vec![];
        if query_id == U256_WIDE_DIV_REM_ADVICE_QUERY_ID {
            let (lo, hi, divisor) = <(U256, U256, U256)>::read_input(memory, input_word).unwrap();
            let dividend = U512::from(host(&lo)) + (U512::from(host(&hi)) << 256);
            let q: U512 = dividend / U512::from(host(&divisor));
            let limbs = q.as_limbs();
            let q_lo: U256 = HostU256::from_limbs(limbs[..4].try_into().unwrap()).into();
            let q_hi: U256 = HostU256::from_limbs(limbs[4..].try_into().unwrap()).into();
            (q_lo, q_hi).write_output(&mut response);
        } else {
            panic!("unexpected query {query_id:#x}")
        }
        response
    }

    fn samples() -> [HostU256; 12] {
        [
            HostU256::ZERO,
            HostU256::from(1u64),
            HostU256::from(2u64),
            HostU256::from(7u64),
            HostU256::from(1u64) << 128,
            HostU256::from_limbs([u64::MAX, u64::MAX, 0, 0]),
            HostU256::from_limbs([0, 0, 0, 0xdead_beef_cafe_babe]),
            HostU256::from_limbs([
                0x0123_4567_89ab_cdef,
                0xfedc_ba98_7654_3210,
                0x0011_2233_4455_6677,
                0x7f00_0000_0000_0000,
            ]),
            HostU256::from_limbs([3, 0, 0, 1u64 << 63]),
            HostU256::MAX >> 1,
            HostU256::MAX - HostU256::from(1u64),
            HostU256::MAX,
        ]
    }

    /// Every wide division of the samples through the query and its check: one query each, and
    /// the remainder of the reference (from `check_all` and `arithmetic_with_advice_matches_reference`,
    /// for the wide division the ModExp path uses)
    #[test]
    fn wide_division_with_advice_matches_reference() {
        let queries = Rc::new(Cell::new(0));
        let counter = Rc::clone(&queries);
        let mut oracle = InProcessMemoryOracle::new(
            move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                counter.set(counter.get() + 1);
                host_division(query_id, input_word, memory)
            },
        );
        let mut expected_queries = 0;
        for lo in samples() {
            for hi in samples() {
                for m in samples() {
                    if m.is_zero() {
                        continue;
                    }
                    let (mut lo_u, mut hi_u, m_u): (U256, U256, U256) =
                        (lo.into(), hi.into(), m.into());
                    let (q_lo, q_hi) = query_wide_div_rem_hint(&lo_u, &hi_u, &m_u, &mut oracle);
                    expected_queries += 1;
                    let dividend = U512::from(lo) + (U512::from(hi) << 256);
                    let q: U512 = dividend / U512::from(m);
                    assert_eq!((host(&q_lo), host(&q_hi)), halves(q), "{dividend} / {m}");
                    assert!(
                        verify_wide_div_rem_hint(&mut lo_u, &mut hi_u, &m_u, &q_lo, &q_hi),
                        "{dividend} / {m}"
                    );
                    assert_eq!(host(&lo_u), halves(dividend % U512::from(m)).0);
                }
            }
        }
        assert_eq!(queries.get(), expected_queries);
    }

    /// The check rejects the neighbours of the quotient, and a quotient whose product with the
    /// divisor overflows 512 bits
    #[test]
    fn wrong_wide_quotients_are_rejected() {
        for lo in samples() {
            for hi in samples() {
                for m in samples() {
                    if m.is_zero() {
                        continue;
                    }
                    let dividend = U512::from(lo) + (U512::from(hi) << 256);
                    let q: U512 = dividend / U512::from(m);
                    let one = U512::from(1u64);
                    for wrong in [
                        q.wrapping_add(one),
                        q.wrapping_sub(one),
                        q.wrapping_add(one << 256),
                        q.wrapping_add(one << 511),
                    ] {
                        let (q_lo, q_hi) = halves(wrong);
                        let (mut lo_u, mut hi_u, m_u): (U256, U256, U256) =
                            (lo.into(), hi.into(), m.into());
                        assert!(
                            !verify_wide_div_rem_hint(
                                &mut lo_u,
                                &mut hi_u,
                                &m_u,
                                &q_lo.into(),
                                &q_hi.into()
                            ),
                            "{dividend} / {m}: accepted {wrong}"
                        );
                    }
                }
            }
        }
    }

    /// The query is one query with the address of the array of the addresses of the three
    /// operands, and an answer of 16 words: no more (the excess is rejected), no less
    #[test]
    fn the_query_takes_exactly_its_answer() {
        let lo: U256 = HostU256::from(35u64).into();
        let hi = U256::zero();
        let divisor: U256 = HostU256::from(6u64).into();
        let answer = |extra: usize, missing: usize| {
            let mut oracle = InProcessMemoryOracle::new(
                move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                    let mut response = host_division(query_id, input_word, memory);
                    response.extend(core::iter::repeat_n(0, extra));
                    response.truncate(response.len() - missing);
                    response
                },
            );
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let (q_lo, q_hi) = query_wide_div_rem_hint(&lo, &hi, &divisor, &mut oracle);
                (host(&q_lo), host(&q_hi))
            }))
            .map_err(|payload| match payload.downcast::<&'static str>() {
                Ok(message) => (*message).to_owned(),
                Err(payload) => *payload.downcast::<String>().expect("a panic message"),
            })
        };
        assert_eq!(answer(0, 0), Ok((HostU256::from(5u64), HostU256::ZERO)));
        for (extra, missing) in [(1, 0), (0, 1), (0, 9)] {
            let message = answer(extra, missing).expect_err("a malformed answer is accepted");
            assert!(
                message.starts_with("wide_div_rem oracle query failed"),
                "{message}"
            );
        }
    }

    type Reducer = fn(&mut U256, &mut U256, &U256, U256, U256);

    /// A small deterministic generator (xorshift64*)
    struct Rng(u64);

    impl Rng {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn u256(&mut self) -> HostU256 {
            HostU256::from_limbs([(); 4].map(|()| self.next_u64()))
        }
    }

    fn halves(value: U512) -> (HostU256, HostU256) {
        let limbs = value.as_limbs();
        (
            HostU256::from_limbs(limbs[..4].try_into().unwrap()),
            HostU256::from_limbs(limbs[4..].try_into().unwrap()),
        )
    }

    /// Runs `reduce` on a fresh copy of `product` with the quotient `(q_lo, q_hi)`: the remainder,
    /// or the panic message
    fn reduce_with(
        reduce: Reducer,
        product: U512,
        modulus: HostU256,
        (q_lo, q_hi): (HostU256, HostU256),
    ) -> Result<HostU256, String> {
        let (lo, hi) = halves(product);
        let (mut lo, mut hi): (U256, U256) = (lo.into(), hi.into());
        let modulus: U256 = modulus.into();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            reduce(&mut lo, &mut hi, &modulus, q_lo.into(), q_hi.into())
        }))
        .map(|()| host(&lo))
        .map_err(|payload| match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(payload) => *payload.downcast::<String>().expect("a panic message"),
        })
    }

    /// Checks both reducers against every answer in `candidates` for `product / modulus`: the
    /// general one accepts exactly the right quotient, the one for residues accepts exactly the
    /// right quotient with a zero high half, and both leave the right remainder
    fn check_reducers(product: U512, modulus: HostU256, candidates: &[(HostU256, HostU256)]) {
        let right = halves(product / U512::from(modulus));
        let remainder = halves(product % U512::from(modulus)).0;
        for &candidate in candidates {
            let is_right = candidate == right;
            for (reduce, accepts) in [
                (reduce_product_with_quotient as Reducer, is_right),
                (
                    reduce_product_of_residues_with_quotient as Reducer,
                    is_right && right.1.is_zero(),
                ),
            ] {
                match reduce_with(reduce, product, modulus, candidate) {
                    Ok(result) => {
                        assert!(accepts, "{product} / {modulus}: accepted {candidate:?}");
                        assert_eq!(result, remainder, "{product} mod {modulus}");
                    }
                    Err(message) => {
                        assert!(!accepts, "{product} / {modulus}: rejected {candidate:?}");
                        assert!(message.starts_with("mulmod hint: "), "{message}");
                    }
                }
            }
        }
    }

    /// Answers an oracle could give for `product / modulus`: the right one, its neighbours, high
    /// halves that are not zero, the extremes, quotients that are right modulo `2^256` or (for a
    /// power of two modulus) whose product with the modulus is right modulo `2^512`, and random ones
    fn quotient_candidates(
        product: U512,
        modulus: HostU256,
        rng: &mut Rng,
    ) -> Vec<(HostU256, HostU256)> {
        let q = product / U512::from(modulus);
        let (q_lo, _) = halves(q);
        let one = U512::from(1u64);
        let mut candidates: Vec<_> = [q, q.wrapping_add(one), q.wrapping_sub(one)]
            .into_iter()
            .chain([0, 1, 64, 128, 255].map(|k| q.wrapping_add(one << 256).wrapping_sub(one << k)))
            .map(halves)
            .collect();
        candidates.extend(
            [
                HostU256::from(1u64),
                HostU256::from(1u64) << 255,
                HostU256::MAX,
            ]
            .map(|hi| (q_lo, hi)),
        );
        candidates.extend([
            (HostU256::ZERO, HostU256::ZERO),
            (HostU256::MAX, HostU256::ZERO),
            (HostU256::MAX, HostU256::MAX),
            (rng.u256(), HostU256::ZERO),
            (rng.u256(), HostU256::ZERO),
            (rng.u256(), rng.u256()),
            (rng.u256(), rng.u256()),
        ]);
        if modulus.is_power_of_two() {
            // (q + 2^(512 - k)) * 2^k = q * 2^k mod 2^512
            let k = modulus.trailing_zeros();
            candidates.push(halves(q.wrapping_add(one << (512 - k))));
        }
        candidates
    }

    /// Moduli with 1 to 4 significant limbs: small ones, around powers of two, the common primes,
    /// and random ones (even and odd)
    fn test_moduli(rng: &mut Rng) -> Vec<HostU256> {
        let hex = |digits: &str| HostU256::from_str_radix(digits, 16).unwrap();
        let pow2 = |k: usize| HostU256::from(1u64) << k;
        let mut moduli: Vec<_> = [2u64, 3, 4, 5, 7, 8, 16, 17, 255, 256]
            .map(HostU256::from)
            .into();
        moduli.extend([
            pow2(64) - HostU256::from(59u64),
            pow2(64),
            pow2(64) + HostU256::from(13u64),
            pow2(128),
            pow2(128) + HostU256::from(51u64),
            pow2(192) + HostU256::from(133u64),
            pow2(255),
            pow2(255) + HostU256::from(1u64),
            // BN254 r and q
            hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001"),
            hex("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            // secp256k1 p and n
            hex("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            hex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"),
            HostU256::MAX - HostU256::from(188u64),
            HostU256::MAX - HostU256::from(1u64),
            HostU256::MAX,
        ]);
        for i in 0..32 {
            let mut limbs = [0u64; 4];
            let significant = 1 + i % 4;
            for limb in &mut limbs[..significant] {
                *limb = rng.next_u64();
            }
            limbs[significant - 1] |= 1 << 63;
            limbs[0] = if i % 2 == 0 {
                limbs[0] & !1
            } else {
                limbs[0] | 1
            };
            moduli.push(HostU256::from_limbs(limbs));
        }
        moduli
    }

    /// Residues `x, y < modulus` (the case the exponent walk of modexp reduces): the reducer for
    /// residues accepts exactly what the general reducer accepts, the right quotient, and its high
    /// half is always zero
    #[test]
    fn residue_product_reducer_accepts_what_the_general_one_accepts() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for modulus in test_moduli(&mut rng) {
            let one = HostU256::from(1u64);
            let two = HostU256::from(2u64);
            let mut operands = vec![
                HostU256::ZERO,
                one,
                two,
                modulus >> 1,
                modulus - two,
                modulus - one,
            ];
            operands.extend((0..4).map(|_| rng.u256() % modulus));
            operands.retain(|x| *x < modulus);
            operands.sort_unstable();
            operands.dedup();
            for (i, x) in operands.iter().enumerate() {
                for y in &operands[i..] {
                    let product: U512 = x.widening_mul(*y);
                    let (_, q_hi) = halves(product / U512::from(modulus));
                    assert!(q_hi.is_zero(), "{x} * {y} / {modulus}");
                    let candidates = quotient_candidates(product, modulus, &mut rng);
                    check_reducers(product, modulus, &candidates);
                }
            }
        }
    }

    /// Products of operands out of range: the reducer for residues never accepts a wrong quotient
    /// (it rejects the right one when its high half is not zero)
    #[test]
    fn residue_product_reducer_never_misreduces_other_products() {
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        for modulus in test_moduli(&mut rng) {
            let operands = [
                modulus,
                modulus.wrapping_add(HostU256::from(1u64)),
                HostU256::MAX,
                rng.u256(),
            ];
            for x in operands {
                for y in [modulus - HostU256::from(1u64), HostU256::MAX, rng.u256()] {
                    let product: U512 = x.widening_mul(y);
                    let candidates = quotient_candidates(product, modulus, &mut rng);
                    check_reducers(product, modulus, &candidates);
                }
            }
        }
    }

    /// Every product of residues and every quotient in a range around the right one, for the
    /// moduli 2 to 17
    #[test]
    fn residue_product_reducer_small_moduli_exhaustive() {
        for m in 2u64..=17 {
            let modulus = HostU256::from(m);
            for x in 0..m {
                for y in x..m {
                    let product = U512::from(x * y);
                    let candidates: Vec<_> = (0..=m + 1)
                        .flat_map(|a| {
                            [HostU256::ZERO, HostU256::from(1u64), HostU256::MAX]
                                .map(|b| (HostU256::from(a), b))
                        })
                        .collect();
                    check_reducers(product, modulus, &candidates);
                }
            }
        }
    }

    /// The copy is the value, in a slot that was never initialised
    #[test]
    fn clone_into_slot_copies_the_value() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for _ in 0..16 {
            let value = rng.u256();
            let source: U256 = value.into();
            let mut slot = MaybeUninit::uninit();
            let copy = clone_into_slot(&mut slot, &source);
            assert_eq!(host(copy), value);
            assert_eq!(host(&source), value);
        }
    }
}
