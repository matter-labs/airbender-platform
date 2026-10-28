//! The wide division advice (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`) of the single-digit ModExp
//! path: the quotient of a 512-bit dividend by a 256-bit divisor, and its checks.
//!
//! The input word is the address of a `[usize; 3]` array of the addresses of the dividend's low
//! half, its high half and the divisor; the answer is the quotient's low and high halves, 8 words
//! each.

use super::{HintError, HintOracle, U256_WIDE_DIV_REM_ADVICE_QUERY_ID};
use crate::modexp::u256::U256;
use core::mem::{align_of, size_of, MaybeUninit};

/// Advice for `(dividend_hi * 2^256 + dividend_lo) / divisor`: the quotient's low and high halves
pub(crate) struct U256WideDivRemAdviceQuery;

impl U256WideDivRemAdviceQuery {
    const QUERY_ID: u32 = U256_WIDE_DIV_REM_ADVICE_QUERY_ID;

    /// Runs the query on `(dividend_lo, dividend_hi, divisor)`
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

/// Number of `u32` words in a `U256`, exchanged as whole aligned words
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

/// The address of `value`, exposed for the oracle to read through
#[inline(always)]
fn exposed_address(value: &U256) -> usize {
    core::ptr::from_ref(value).expose_provenance()
}

/// Starts a query whose input word is the address of the array of the operands' addresses
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
    // SAFETY: the input word is the address of the wide division request, which, like the
    // operands, is alive during the call
    unsafe {
        oracle.send_query(
            query_id,
            core::ptr::from_ref(&addresses).expose_provenance(),
        )
    }
}

/// Receives the two halves of the quotient in turn
#[inline(always)]
fn receive_value<O: HintOracle>(oracle: &mut O) -> Result<(U256, U256), HintError> {
    let mut value = MaybeUninit::<(U256, U256)>::uninit();
    let this = value.as_mut_ptr();
    // SAFETY: the elements are disjoint places inside `value`
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

/// Writes the next `U256` of the response to `dst`
#[inline(always)]
fn init<'a, O: HintOracle>(
    oracle: &mut O,
    dst: &'a mut MaybeUninit<U256>,
) -> Result<&'a mut U256, HintError> {
    let this = dst.as_mut_ptr();
    // SAFETY: `dst` is a `U256`, `U256_WORDS` words aligned for `u32`
    unsafe { oracle.write_words(this.cast::<u32>(), U256_WORDS)? };
    // SAFETY: all bytes of `dst` were just written, and any bytes are a valid `U256`
    Ok(unsafe { &mut *this })
}

/// Ends the query, whether its output was received or rejected, so the oracle stays in step
#[inline(always)]
fn finish<O: HintOracle, T>(oracle: &mut O, output: Result<T, HintError>) -> Result<T, HintError> {
    let finished = oracle.finish_query();
    let output = output?;
    finished?;
    Ok(output)
}

/// Checks a wide quotient: `dividend - q * divisor` must be in `[0, divisor)`. Leaves the
/// remainder in `dividend_lo` on success; both halves of the dividend are overwritten.
#[must_use]
pub(crate) fn verify_wide_div_rem_hint(
    dividend_lo: &mut U256,
    dividend_hi: &mut U256,
    divisor: &U256,
    q_lo: &U256,
    q_hi: &U256,
) -> bool {
    // q * divisor as 512 bits
    let mut qd_lo = q_lo.clone();
    let mut qd_mid = q_lo.clone();
    qd_lo.widening_mul_assign_into(&mut qd_mid, divisor);

    let mut qd_hi_lo = q_hi.clone();
    let mut qd_hi_hi = q_hi.clone();
    qd_hi_lo.widening_mul_assign_into(&mut qd_hi_hi, divisor);

    // (qd_lo, qd_mid) = q * divisor, which must fit in 512 bits
    let c1 = qd_mid.overflowing_add_assign(&qd_hi_lo);
    if c1 || !qd_hi_hi.is_zero() {
        return false;
    }

    // r = dividend - q * divisor, in place
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

/// Asks the oracle for `(dividend_hi * 2^256 + dividend_lo) / divisor`, low half first
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

/// Reduces the 512-bit `(product_lo, product_hi)` modulo `modulus` with its advised quotient: the
/// remainder is formed in `product_lo` and checked to be in `[0, modulus)`. Panics on a wrong
/// quotient; `product_hi` is scratch.
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

/// Copies `x` into `slot`. On the guest `x.clone()` would copy through a temporary that LLVM
/// cannot merge with the destination.
#[inline(always)]
pub(crate) fn clone_into_slot<'a>(slot: &'a mut MaybeUninit<U256>, x: &U256) -> &'a mut U256 {
    // SAFETY: `slot` is an exclusively borrowed `U256` place, so it is aligned (by type) and
    // disjoint from `x`, and the copy initialises all of it
    unsafe {
        U256::write_into_ptr_unchecked(slot.as_mut_ptr(), x);
        slot.assume_init_mut()
    }
}

/// As `reduce_product_with_quotient`, for a product `x * y` of residues `x, y < modulus`, with one
/// multiplication by the modulus: the quotient is below `modulus`, so `q_hi` must be zero. For
/// such a product it accepts exactly the right quotient; for others it may reject the right one
/// but never accepts a wrong one. Panics on a wrong quotient; `product_hi` is scratch.
#[inline]
pub(crate) fn reduce_product_of_residues_with_quotient(
    product_lo: &mut U256,
    product_hi: &mut U256,
    modulus: &U256,
    mut q_lo: U256,
    q_hi: U256,
) {
    // the high half must be zero
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

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::hints::host::{answer_query, RecordingHintOracle};
    use crate::hints::test_support::InProcessMemoryOracle;
    use crate::hints::QuerierMemory;
    use ruint::aliases::{U256 as HostU256, U512};

    fn host(value: &U256) -> HostU256 {
        value.clone().into()
    }

    fn halves(value: U512) -> (HostU256, HostU256) {
        let limbs = value.as_limbs();
        (
            HostU256::from_limbs(limbs[..4].try_into().unwrap()),
            HostU256::from_limbs(limbs[4..].try_into().unwrap()),
        )
    }

    /// The result of `f`, or its panic message
    fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|payload| {
            match payload.downcast::<&'static str>() {
                Ok(message) => (*message).to_owned(),
                Err(payload) => *payload.downcast::<String>().expect("a panic message"),
            }
        })
    }

    fn samples() -> [HostU256; 7] {
        [
            HostU256::ZERO,
            HostU256::from(1u64),
            HostU256::from(7u64),
            HostU256::from(1u64) << 128,
            HostU256::from_limbs([3, 0, 0, 1u64 << 63]),
            HostU256::MAX >> 1,
            HostU256::MAX,
        ]
    }

    /// Each division is one query whose quotient passes the check, and the check rejects the
    /// quotient's neighbours and quotients whose product with the divisor overflows 512 bits
    #[test]
    fn wide_division_with_advice_matches_reference() {
        let mut oracle = RecordingHintOracle::new();
        let mut divisions = 0;
        let one = U512::from(1u64);
        for lo in samples() {
            for hi in samples() {
                for m in samples().into_iter().filter(|m| !m.is_zero()) {
                    let dividend = U512::from(lo) + (U512::from(hi) << 256);
                    let q: U512 = dividend / U512::from(m);
                    // the remainder, if the check accepts the quotient
                    let check = |(q_lo, q_hi): (U256, U256)| {
                        let (mut lo, mut hi): (U256, U256) = (lo.into(), hi.into());
                        verify_wide_div_rem_hint(&mut lo, &mut hi, &m.into(), &q_lo, &q_hi)
                            .then(|| host(&lo))
                    };
                    let answer =
                        query_wide_div_rem_hint(&lo.into(), &hi.into(), &m.into(), &mut oracle);
                    divisions += 1;
                    assert_eq!(
                        (host(&answer.0), host(&answer.1)),
                        halves(q),
                        "{dividend} / {m}"
                    );
                    assert_eq!(check(answer), Some(halves(dividend % U512::from(m)).0));
                    for wrong in [
                        q.wrapping_add(one),
                        q.wrapping_sub(one),
                        q.wrapping_add(one << 256),
                        q.wrapping_add(one << 511),
                    ] {
                        let (q_lo, q_hi) = halves(wrong);
                        let accepted = check((q_lo.into(), q_hi.into()));
                        assert_eq!(accepted, None, "{dividend} / {m}: accepted {wrong}");
                    }
                }
            }
        }
        // one 16-word answer per division
        assert_eq!(oracle.tape().len(), 16 * divisions);
    }

    /// The answer is exactly 16 words: a longer or a shorter one is rejected
    #[test]
    fn the_query_takes_exactly_its_answer() {
        let lo: U256 = HostU256::from(35u64).into();
        let hi = U256::zero();
        let divisor: U256 = HostU256::from(6u64).into();
        let answer = |extra: usize, missing: usize| {
            let mut oracle = InProcessMemoryOracle::new(
                move |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                    let mut response = Vec::new();
                    // SAFETY: the handler runs during the query, while its operands are live
                    let answered = unsafe {
                        answer_query(query_id, input_word, memory, Some(&mut response), None)
                    };
                    assert_eq!(answered, Ok(true));
                    response.extend(core::iter::repeat_n(0, extra));
                    response.truncate(response.len() - missing);
                    response
                },
            );
            catch(|| {
                let (q_lo, q_hi) = query_wide_div_rem_hint(&lo, &hi, &divisor, &mut oracle);
                (host(&q_lo), host(&q_hi))
            })
        };
        assert_eq!(answer(0, 0), Ok((HostU256::from(5u64), HostU256::ZERO)));
        for (extra, missing) in [(1, 0), (0, 1)] {
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

    /// Runs `reduce` on `product` with the quotient `(q_lo, q_hi)`: the remainder, or the panic
    /// message
    fn reduce_with(
        reduce: Reducer,
        product: U512,
        modulus: HostU256,
        (q_lo, q_hi): (HostU256, HostU256),
    ) -> Result<HostU256, String> {
        let (lo, hi) = halves(product);
        let (mut lo, mut hi): (U256, U256) = (lo.into(), hi.into());
        let modulus: U256 = modulus.into();
        catch(|| reduce(&mut lo, &mut hi, &modulus, q_lo.into(), q_hi.into())).map(|()| host(&lo))
    }

    /// Both reducers against each candidate answer for `product / modulus`: the general one
    /// accepts exactly the right quotient, the one for residues exactly the right quotient with a
    /// zero high half, and both leave the right remainder
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

    /// Answers an oracle could give for `product / modulus`: the right one, its neighbours, non-zero
    /// high halves, extremes, quotients right modulo `2^256` or (for a power-of-two modulus) whose
    /// product with the modulus is right modulo `2^512`, and random ones
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
            (rng.u256(), rng.u256()),
        ]);
        if modulus.is_power_of_two() {
            // (q + 2^(512 - k)) * 2^k = q * 2^k mod 2^512
            let k = modulus.trailing_zeros();
            candidates.push(halves(q.wrapping_add(one << (512 - k))));
        }
        candidates
    }

    /// Moduli with 1 to 4 significant limbs: small ones, around powers of two, common primes, and
    /// random ones (even and odd)
    fn test_moduli(rng: &mut Rng) -> Vec<HostU256> {
        let hex = |digits: &str| HostU256::from_str_radix(digits, 16).unwrap();
        let pow2 = |k: usize| HostU256::from(1u64) << k;
        let mut moduli: Vec<_> = [2u64, 3, 17, 256].map(HostU256::from).into();
        moduli.extend([
            pow2(64) - HostU256::from(59u64),
            pow2(64),
            pow2(128) + HostU256::from(51u64),
            pow2(255),
            // BN254 q and secp256k1 p
            hex("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            hex("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            HostU256::MAX,
        ]);
        for i in 0..8 {
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

    /// For residues `x, y < modulus` the reducer for residues accepts exactly what the general one
    /// accepts; for other operands it never accepts a wrong quotient (and rejects the right one
    /// when its high half is not zero)
    #[test]
    fn residue_product_reducer_accepts_what_the_general_one_accepts() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for modulus in test_moduli(&mut rng) {
            let one = HostU256::from(1u64);
            let mut operands = vec![HostU256::ZERO, one, modulus >> 1, modulus - one];
            operands.extend((0..3).map(|_| rng.u256() % modulus));
            // out of range
            operands.extend([
                modulus,
                modulus.wrapping_add(one),
                HostU256::MAX,
                rng.u256(),
            ]);
            for (i, x) in operands.iter().enumerate() {
                for y in &operands[i..] {
                    let product: U512 = x.widening_mul(*y);
                    if *x < modulus && *y < modulus {
                        let (_, q_hi) = halves(product / U512::from(modulus));
                        assert!(q_hi.is_zero(), "{x} * {y} / {modulus}");
                    }
                    let candidates = quotient_candidates(product, modulus, &mut rng);
                    check_reducers(product, modulus, &candidates);
                }
            }
        }
    }
}
