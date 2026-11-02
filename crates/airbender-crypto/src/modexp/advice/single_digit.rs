//! ModExp for a modulus of up to 32 bytes: residues are `U256`s, and a modular product is one
//! 512-bit product reduced with an advised quotient.

use super::bigint::ModexpAdvisor;
use super::exponent::{exponentiate, ModMul};
use crate::hints::wide_div::{
    clone_into_slot, reduce_product_of_residues_with_quotient, verify_wide_div_rem_hint,
};
use crate::modexp::strip_leading_zeroes;
use crate::modexp::u256::U256;
use alloc::vec::Vec;
use core::alloc::Allocator;
use core::mem::MaybeUninit;

struct U256ModMul<'a, Adv: ModexpAdvisor> {
    modulus: &'a U256,
    advisor: &'a mut Adv,
}

impl<Adv: ModexpAdvisor> ModMul for U256ModMul<'_, Adv> {
    type Elem = U256;

    fn placeholder(&mut self) -> U256 {
        U256::zero()
    }

    fn clone_elem(&mut self, x: &U256) -> U256 {
        x.clone()
    }

    fn is_zero(&self, x: &U256) -> bool {
        x.is_zero()
    }

    #[inline(always)]
    fn square_assign(&mut self, x: &mut U256) {
        // the product needs the multiplier as a separate operand
        let mut y = MaybeUninit::uninit();
        let y = clone_into_slot(&mut y, x);
        self.mul_assign(x, y);
    }

    #[inline(always)]
    fn mul_assign(&mut self, x: &mut U256, y: &U256) {
        // x, y < m (`exponentiate` multiplies only the reduced base and checked remainders), so
        // the true quotient has q_hi = 0. The reducer accepts only q_hi = 0 and
        // x * y - q_lo * m in [0, m), which holds for the true quotient alone.
        debug_assert!(*x < *self.modulus && *y < *self.modulus);
        // the product lands in (x, hi)
        let mut hi = MaybeUninit::uninit();
        let hi = clone_into_slot(&mut hi, x);
        x.widening_mul_assign_into(hi, y);
        let (q_lo, q_hi) = self.advisor.wide_quotient(x, hi, self.modulus);
        reduce_product_of_residues_with_quotient(x, hi, self.modulus, q_lo, q_hi);
    }
}

/// `base^exp mod modulus` as big-endian bytes, for `modulus >= 2`
pub(crate) fn modexp<Adv: ModexpAdvisor, A: Allocator>(
    base: &[u8],
    exp: &[u8],
    modulus: &U256,
    advisor: &mut Adv,
    allocator: A,
) -> Vec<u8, A> {
    debug_assert!(!modulus.is_zero() && !modulus.is_one());

    let base = reduce_base(strip_leading_zeroes(base), modulus, advisor);
    let exp = strip_leading_zeroes(exp);

    let result = if exp.is_empty() {
        // base^0 = 1, including 0^0
        U256::one()
    } else if base.is_zero() || base.is_one() {
        base
    } else {
        let mut arithmetic = U256ModMul { modulus, advisor };
        exponentiate(&mut arithmetic, &base, exp)
    };

    // as the multi-digit path: no bytes for zero, else the whole digit
    let mut out = Vec::with_capacity_in(32, allocator);
    if !result.is_zero() {
        out.extend_from_slice(&result.to_be_bytes());
    }
    out
}

/// `base mod modulus`, one 32-byte digit at a time from the top: `acc = (acc * 2^256 + digit)
/// mod modulus` with an advised, checked quotient. A digit below the modulus while `acc` is zero
/// needs no hint.
fn reduce_base<Adv: ModexpAdvisor>(base: &[u8], modulus: &U256, advisor: &mut Adv) -> U256 {
    let mut acc = U256::zero();
    let (head, digits) = base.as_rchunks::<32>();
    if !head.is_empty() {
        let mut padded = [0u8; 32];
        padded[32 - head.len()..].copy_from_slice(head);
        reduce_digit(&mut acc, &padded, modulus, advisor);
    }
    for digit in digits {
        reduce_digit(&mut acc, digit, modulus, advisor);
    }
    acc
}

fn reduce_digit<Adv: ModexpAdvisor>(
    acc: &mut U256,
    digit: &[u8; 32],
    modulus: &U256,
    advisor: &mut Adv,
) {
    let mut lo = U256::from_be_bytes(digit);
    if acc.is_zero() {
        // lo < modulus: the subtraction on a scratch borrows
        let mut scratch = lo.clone();
        if scratch.overflowing_sub_assign(modulus) {
            *acc = lo;
            return;
        }
    }
    let (q_lo, q_hi) = advisor.wide_quotient(&lo, acc, modulus);
    assert!(
        verify_wide_div_rem_hint(&mut lo, acc, modulus, &q_lo, &q_hi),
        "modexp base reduction hint: wrong quotient"
    );
    *acc = lo;
}

#[cfg(test)]
mod tests {
    use super::super::bigint::{naive_wide_quotient, BigintRepr};
    use super::*;
    use num_bigint::BigUint;
    use ruint::aliases::{U256 as HostU256, U512};

    const BN254_R: &str = "30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001";

    fn host(value: &U256) -> HostU256 {
        value.clone().into()
    }

    fn reference(base: HostU256, exp: &[u8], modulus: HostU256) -> HostU256 {
        let result = BigUint::from_bytes_be(&base.to_be_bytes::<32>()).modpow(
            &BigUint::from_bytes_be(exp),
            &BigUint::from_bytes_be(&modulus.to_be_bytes::<32>()),
        );
        HostU256::from_be_slice(&result.to_bytes_be())
    }

    /// Answers wide quotients honestly, except call `forged_call`, which gets
    /// `forge(quotient, modulus)`
    struct ForgingAdvisor {
        calls: usize,
        forged_call: usize,
        forge: fn(U512, HostU256) -> U512,
        /// Whether the forged answer differs from the true one, once given
        forged: Option<bool>,
    }

    impl ForgingAdvisor {
        fn new(forged_call: usize, forge: fn(U512, HostU256) -> U512) -> Self {
            Self {
                calls: 0,
                forged_call,
                forge,
                forged: None,
            }
        }
    }

    impl ModexpAdvisor for ForgingAdvisor {
        fn get_reduction_op_advice<A: Allocator + Clone>(
            &mut self,
            _a: &BigintRepr<A>,
            _m: &BigintRepr<A>,
            _quotient_dst: &mut BigintRepr<A>,
            _remainder_dst: &mut BigintRepr<A>,
        ) {
            panic!("the single-digit path asks for no reduction advice")
        }

        fn wide_quotient(&mut self, lo: &U256, hi: &U256, modulus: &U256) -> (U256, U256) {
            let (q_lo, q_hi) = naive_wide_quotient(lo, hi, modulus);
            let call = self.calls;
            self.calls += 1;
            if call != self.forged_call {
                return (q_lo, q_hi);
            }
            let right = U512::from(host(&q_lo)) + (U512::from(host(&q_hi)) << 256);
            let forged = (self.forge)(right, host(modulus));
            self.forged = Some(forged != right);
            let limbs = forged.as_limbs();
            (
                HostU256::from_limbs(limbs[..4].try_into().unwrap()).into(),
                HostU256::from_limbs(limbs[4..].try_into().unwrap()).into(),
            )
        }
    }

    /// `base^exp mod modulus` with `advisor`: the result, or the panic message
    fn exponentiate_with(
        advisor: &mut ForgingAdvisor,
        base: HostU256,
        exp: &[u8],
        modulus: HostU256,
    ) -> Result<HostU256, String> {
        let (base, modulus): (U256, U256) = (base.into(), modulus.into());
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut arithmetic = U256ModMul {
                modulus: &modulus,
                advisor: &mut *advisor,
            };
            host(&exponentiate(&mut arithmetic, &base, exp))
        }))
        .map_err(|payload| match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(payload) => *payload.downcast::<String>().expect("a panic message"),
        })
    }

    /// Wrong answers for a quotient `q` mod `m`: neighbours, a non-zero high half, extremes, and
    /// for `m = 2^k` `q + 2^(512 - k)`, whose product with `m` is right mod 2^512. Some can equal
    /// the true quotient (e.g. 0); the test allows for that.
    const FORGERIES: [fn(U512, HostU256) -> U512; 10] = [
        |q, _| q.wrapping_add(U512::from(1u64)),
        |q, _| q.wrapping_sub(U512::from(1u64)),
        |q, _| q.wrapping_add(U512::from(1u64) << 256),
        |q, _| q.wrapping_add(U512::from(1u64) << 511),
        |q, _| q | (U512::from(HostU256::MAX) << 256),
        |q, _| {
            q.wrapping_add(U512::from(1u64) << 256)
                .wrapping_sub(U512::from(1u64))
        },
        |_, _| U512::ZERO,
        |_, _| U512::from(HostU256::MAX),
        |_, _| U512::MAX,
        |q, m| {
            if m.is_power_of_two() {
                q.wrapping_add(U512::from(1u64) << (512 - m.trailing_zeros()))
            } else {
                q.wrapping_add(U512::from(2u64))
            }
        },
    ];

    /// A forged quotient at any product of the walk (table, squaring, multiplication, last) panics
    /// in the reducer at that product; honest answers give the reference result
    #[test]
    fn forged_quotients_are_rejected_where_they_are_answered() {
        let r = HostU256::from_str_radix(BN254_R, 16).unwrap();
        let bn254_base = HostU256::from_str_radix(
            "01c9d6f3b0e2a4d5c8b7a69584736251403f2e1d0c9b8a79685746352413021f",
            16,
        )
        .unwrap();
        let cases = [
            // windowed walk
            (
                bn254_base,
                (r - HostU256::from(2u64)).to_be_bytes::<32>().to_vec(),
                r,
            ),
            // binary walk (0b11000)
            (bn254_base, vec![0x18], r),
            // power-of-two modulus
            (
                HostU256::from(3u64),
                vec![0xff; 9],
                HostU256::from(1u64) << 255,
            ),
            // the largest quotients: (m - 1)^2 / m = m - 2
            (
                HostU256::MAX - HostU256::from(1u64),
                vec![0xff; 32],
                HostU256::MAX,
            ),
        ];
        for (base, exp, modulus) in cases {
            let expected = reference(base, &exp, modulus);
            let mut honest = ForgingAdvisor::new(usize::MAX, |q, _| q);
            assert_eq!(
                exponentiate_with(&mut honest, base, &exp, modulus),
                Ok(expected)
            );
            let products = honest.calls;
            for forged_call in [0, 1, 13, 14, 22, products - 1] {
                if forged_call >= products {
                    continue;
                }
                for forge in FORGERIES {
                    let mut advisor = ForgingAdvisor::new(forged_call, forge);
                    let result = exponentiate_with(&mut advisor, base, &exp, modulus);
                    match advisor.forged {
                        Some(true) => {
                            let message = result.expect_err("a forged quotient is accepted");
                            assert!(message.starts_with("mulmod hint: "), "{message}");
                            assert_eq!(advisor.calls, forged_call + 1);
                        }
                        Some(false) => assert_eq!(result, Ok(expected)),
                        None => panic!("call {forged_call} of {products} is not answered"),
                    }
                }
            }
        }
    }
}
