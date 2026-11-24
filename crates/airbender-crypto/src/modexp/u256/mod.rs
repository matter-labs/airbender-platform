//! The 256-bit values of the single-digit path, one arithmetic for each kind of target: over
//! the bigint delegation with `cfg(bigint_ops)` (RV32, and the hosts that emulate it), four plain
//! `u64` limbs without it (RV64 and the other hosts). Both have the same interface, and ModExp
//! asks the same hints on either. Tests check the plain one against `ruint`, and with
//! `bigint_ops` compare the two.

// `U256` is not `Copy`: with the delegation a clone is a delegated copy

#[cfg(any(not(bigint_ops), test))]
mod naive;

#[cfg(not(bigint_ops))]
pub(crate) use self::naive::U256;

#[cfg(bigint_ops)]
mod delegated;

#[cfg(bigint_ops)]
pub(crate) use self::delegated::U256;

#[cfg(test)]
mod naive_tests {
    use super::naive::U256;
    use core::cmp::Ordering;
    use core::mem::MaybeUninit;
    use proptest::{prop_assert, prop_assert_eq, proptest};
    use ruint::aliases::{U256 as HostU256, U512};

    /// Limbs that are often zero or all ones: long carry and borrow chains, equal values
    fn limbs(seed: [u64; 4], shape: u8) -> [u64; 4] {
        core::array::from_fn(|i| match (shape >> (2 * i)) & 3 {
            0 | 1 => seed[i],
            2 => 0,
            _ => u64::MAX,
        })
    }

    /// Every operation of the plain `U256` against `ruint`
    #[test]
    fn naive_matches_reference() {
        proptest!(|(x: [u64; 4], y: [u64; 4], shapes: (u8, u8), borrow: bool, same: bool)| {
            let x = limbs(x, shapes.0);
            let y = if same { x } else { limbs(y, shapes.1) };
            let (n, ny) = (U256::from_limbs(x), U256::from_limbs(y));
            let (host_x, host_y) = (HostU256::from_limbs(x), HostU256::from_limbs(y));

            let mut sum = n.clone();
            let carry = sum.overflowing_add_assign(&ny);
            let (expected, overflow) = host_x.overflowing_add(host_y);
            prop_assert_eq!((sum.as_limbs(), carry), (expected.as_limbs(), overflow));

            let mut difference = n.clone();
            let borrowed = difference.overflowing_sub_assign(&ny);
            let (expected, overflow) = host_x.overflowing_sub(host_y);
            prop_assert_eq!((difference.as_limbs(), borrowed), (expected.as_limbs(), overflow));

            let mut difference = n.clone();
            let borrowed = difference.overflowing_sub_assign_with_borrow_propagation(&ny, borrow);
            let (expected, of1) = host_x.overflowing_sub(host_y);
            let (expected, of2) = expected.overflowing_sub(HostU256::from(borrow as u64));
            prop_assert_eq!((difference.as_limbs(), borrowed), (expected.as_limbs(), of1 | of2));

            let product: U512 = host_x.widening_mul(host_y);
            let (mut low, mut high) = (n.clone(), U256::zero());
            low.widening_mul_assign_into(&mut high, &ny);
            prop_assert_eq!(&low.as_limbs()[..], &product.as_limbs()[..4]);
            prop_assert_eq!(&high.as_limbs()[..], &product.as_limbs()[4..]);
            let (mut low, mut slot) = (n.clone(), MaybeUninit::uninit());
            let high = low.widening_mul_assign_into_slot(&mut slot, &ny);
            prop_assert_eq!(&low.as_limbs()[..], &product.as_limbs()[..4]);
            prop_assert_eq!(&high.as_limbs()[..], &product.as_limbs()[4..]);

            let square: U512 = host_x.widening_mul(host_x);
            let (mut low, mut slot) = (n.clone(), MaybeUninit::uninit());
            let high = low.widening_square_assign_into_slot(&mut slot);
            prop_assert_eq!(&low.as_limbs()[..], &square.as_limbs()[..4]);
            prop_assert_eq!(&high.as_limbs()[..], &square.as_limbs()[4..]);

            prop_assert_eq!(n.cmp(&ny), host_x.cmp(&host_y));
            prop_assert_eq!(n.partial_cmp(&ny), Some(host_x.cmp(&host_y)));
            prop_assert_eq!(n == ny, host_x == host_y);
            prop_assert_eq!(n.cmp(&n.clone()), Ordering::Equal);
            prop_assert_eq!(n.is_zero(), host_x.is_zero());
            prop_assert_eq!(n.is_one(), host_x == HostU256::from(1u64));

            let mut slot = MaybeUninit::uninit();
            // SAFETY: the slot is aligned and writable
            let copy = unsafe {
                U256::write_into_ptr_unchecked(slot.as_mut_ptr(), &n);
                slot.assume_init()
            };
            prop_assert!(copy == n);
            let mut copy = U256::zero();
            copy.clone_from(&n);
            prop_assert!(copy == n);

            let bytes = host_x.to_be_bytes::<32>();
            let from_bytes = U256::from_be_bytes(&bytes);
            prop_assert_eq!(from_bytes.as_limbs(), &x);
            prop_assert_eq!(n.to_be_bytes(), bytes);
            let host: HostU256 = n.into();
            prop_assert_eq!(host, host_x);
            let from_host = U256::from(host_x);
            prop_assert_eq!(from_host.as_limbs(), &x);
        });

        assert!(U256::zero().is_zero() && U256::one().is_one());
        assert!(!U256::one().is_zero() && !U256::zero().is_one());
        assert!(U256::ZERO.is_zero() && U256::ONE.is_one());
        // the largest square: (2^256 - 1)^2 = 2^512 - 2^257 + 1
        let (mut low, mut slot) = (U256::from_limbs([u64::MAX; 4]), MaybeUninit::uninit());
        let high = low.widening_square_assign_into_slot(&mut slot);
        assert_eq!(low.as_limbs(), &[1, 0, 0, 0]);
        assert_eq!(
            high.as_limbs(),
            &[u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]
        );
    }
}

#[cfg(all(test, bigint_ops))]
mod tests {
    use super::{delegated, naive};
    use crate::bigint_delegation::delegation::DELEGATION_CALLS_BY_OP;
    use crate::BigIntOps;
    use core::mem::MaybeUninit;
    use proptest::{prop_assert_eq, proptest};
    use ruint::aliases::{U256 as HostU256, U512};

    #[test]
    fn backends_agree() {
        proptest!(|(x: [u64; 4], y: [u64; 4], borrow: bool)| {
            let (n, ny) = (naive::U256::from_limbs(x), naive::U256::from_limbs(y));
            let (d, dy) = (delegated::U256::from_limbs(x), delegated::U256::from_limbs(y));
            let (host_x, host_y) = (HostU256::from_limbs(x), HostU256::from_limbs(y));

            let (mut n1, mut d1) = (n.clone(), d.clone());
            prop_assert_eq!(n1.overflowing_add_assign(&ny), d1.overflowing_add_assign(&dy));
            prop_assert_eq!(n1.as_limbs(), d1.as_limbs());

            let (mut n1, mut d1) = (n.clone(), d.clone());
            prop_assert_eq!(n1.overflowing_sub_assign(&ny), d1.overflowing_sub_assign(&dy));
            prop_assert_eq!(n1.as_limbs(), d1.as_limbs());

            let (mut n1, mut d1) = (n.clone(), d.clone());
            let borrow1 = n1.overflowing_sub_assign_with_borrow_propagation(&ny, borrow);
            let borrow2 = d1.overflowing_sub_assign_with_borrow_propagation(&dy, borrow);
            let (expected, of1) = host_x.overflowing_sub(host_y);
            let (expected, of2) = expected.overflowing_sub(HostU256::from(borrow as u64));
            prop_assert_eq!((n1.as_limbs(), borrow1), (expected.as_limbs(), of1 | of2));
            prop_assert_eq!((d1.as_limbs(), borrow2), (expected.as_limbs(), of1 | of2));

            // the delegated product reads the multiplicand from the high slot too, so callers
            // copy it there first
            let (mut n1, mut d1) = (n.clone(), d.clone());
            let (mut n_hi, mut d_hi) = (n.clone(), d.clone());
            n1.widening_mul_assign_into(&mut n_hi, &ny);
            d1.widening_mul_assign_into(&mut d_hi, &dy);
            let product: U512 = host_x.widening_mul(host_y);
            prop_assert_eq!(&d1.as_limbs()[..], &product.as_limbs()[..4]);
            prop_assert_eq!(&d_hi.as_limbs()[..], &product.as_limbs()[4..]);
            prop_assert_eq!((n1.as_limbs(), n_hi.as_limbs()), (d1.as_limbs(), d_hi.as_limbs()));

            // the same product and the square, with the high half in a fresh slot
            let (mut n1, mut d1) = (n.clone(), d.clone());
            let (mut n_slot, mut d_slot) = (MaybeUninit::uninit(), MaybeUninit::uninit());
            let n_hi = n1.widening_mul_assign_into_slot(&mut n_slot, &ny);
            let d_hi = d1.widening_mul_assign_into_slot(&mut d_slot, &dy);
            prop_assert_eq!(&n1.as_limbs()[..], &product.as_limbs()[..4]);
            prop_assert_eq!(&n_hi.as_limbs()[..], &product.as_limbs()[4..]);
            prop_assert_eq!((n1.as_limbs(), n_hi.as_limbs()), (d1.as_limbs(), d_hi.as_limbs()));
            let (mut n1, mut d1) = (n.clone(), d.clone());
            let (mut n_slot, mut d_slot) = (MaybeUninit::uninit(), MaybeUninit::uninit());
            let n_hi = n1.widening_square_assign_into_slot(&mut n_slot);
            let d_hi = d1.widening_square_assign_into_slot(&mut d_slot);
            let square: U512 = host_x.widening_mul(host_x);
            prop_assert_eq!(&n1.as_limbs()[..], &square.as_limbs()[..4]);
            prop_assert_eq!(&n_hi.as_limbs()[..], &square.as_limbs()[4..]);
            prop_assert_eq!((n1.as_limbs(), n_hi.as_limbs()), (d1.as_limbs(), d_hi.as_limbs()));

            prop_assert_eq!(n.cmp(&ny), d.cmp(&dy));
            prop_assert_eq!(n == ny, d == dy);
            prop_assert_eq!(d.cmp(&d.clone()), core::cmp::Ordering::Equal);
            prop_assert_eq!((n.is_zero(), n.is_one()), (d.is_zero(), d.is_one()));

            let (mut slot1, mut slot2) = (MaybeUninit::uninit(), MaybeUninit::uninit());
            // SAFETY: the slots are aligned and writable
            let (copy1, copy2) = unsafe {
                naive::U256::write_into_ptr_unchecked(slot1.as_mut_ptr(), &n);
                delegated::U256::write_into_ptr_unchecked(slot2.as_mut_ptr(), &d);
                (slot1.assume_init(), slot2.assume_init())
            };
            prop_assert_eq!((copy1.as_limbs(), copy2.as_limbs()), (&x, &x));

            let bytes = host_x.to_be_bytes::<32>();
            let n1 = naive::U256::from_be_bytes(&bytes);
            let d1 = delegated::U256::from_be_bytes(&bytes);
            prop_assert_eq!((n1.as_limbs(), d1.as_limbs()), (&x, &x));
            prop_assert_eq!((n.to_be_bytes(), d.to_be_bytes()), (bytes, bytes));
            let (host1, host2): (HostU256, HostU256) = (n.into(), d.into());
            prop_assert_eq!((host1, host2), (host_x, host_x));
            let (n1, d1) = (naive::U256::from(host_x), delegated::U256::from(host_x));
            prop_assert_eq!((n1.as_limbs(), d1.as_limbs()), (&x, &x));
        });

        assert!(naive::U256::zero().is_zero() && naive::U256::one().is_one());
        assert!(delegated::U256::zero().is_zero() && delegated::U256::one().is_one());
        assert!(!naive::U256::one().is_zero() && !naive::U256::zero().is_one());
        assert!(!delegated::U256::one().is_zero() && !delegated::U256::zero().is_one());
    }

    /// The delegation calls of `f`, by operation
    fn delegations(f: impl FnOnce()) -> [u64; 8] {
        DELEGATION_CALLS_BY_OP.with(|calls| *calls.borrow_mut() = [0; 8]);
        f();
        DELEGATION_CALLS_BY_OP.with(|calls| *calls.borrow())
    }

    fn calls(ops: &[(BigIntOps, u64)]) -> [u64; 8] {
        let mut calls = [0; 8];
        for (op, n) in ops {
            calls[*op as usize] += n;
        }
        calls
    }

    /// The delegated `U256` takes zksync-os's delegation sequences, which the guest's costs
    /// depend on
    #[test]
    fn delegated_operations_take_the_delegations_of_zksync_os() {
        use BigIntOps::*;
        let x = delegated::U256::from_limbs([5, 6, 7, 8]);
        let y = delegated::U256::from_limbs([1, 2, 3, 4]);
        assert_eq!(delegations(|| drop(x.clone())), calls(&[(MemCpy, 1)]));
        assert_eq!(delegations(|| assert!(!x.is_zero())), calls(&[(Eq, 1)]));
        // a copy into the scratch, a comparison, and a subtraction unless equal
        assert_eq!(
            delegations(|| assert!(y < x)),
            calls(&[(MemCpy, 1), (Eq, 1), (Sub, 1)])
        );
        assert_eq!(
            delegations(|| assert!(x >= x.clone())),
            calls(&[(MemCpy, 2), (Eq, 1)])
        );
        let (mut low, mut high) = (x.clone(), x.clone());
        assert_eq!(
            delegations(|| low.widening_mul_assign_into(&mut high, &y)),
            calls(&[(MulLow, 1), (MulHigh, 1)])
        );
        // with the high half in a fresh slot: a copy of the multiplicand first, and for a square
        // a copy of the multiplier before it
        let (mut low, mut slot) = (x.clone(), MaybeUninit::uninit());
        assert_eq!(
            delegations(|| {
                let _ = low.widening_mul_assign_into_slot(&mut slot, &y);
            }),
            calls(&[(MemCpy, 1), (MulLow, 1), (MulHigh, 1)])
        );
        let (mut low, mut slot) = (x.clone(), MaybeUninit::uninit());
        assert_eq!(
            delegations(|| {
                let _ = low.widening_square_assign_into_slot(&mut slot);
            }),
            calls(&[(MemCpy, 2), (MulLow, 1), (MulHigh, 1)])
        );
        let mut z = x.clone();
        assert_eq!(
            delegations(|| assert!(!z.overflowing_sub_assign_with_borrow_propagation(&y, true))),
            calls(&[(Sub, 1)])
        );
        assert_eq!(
            delegations(|| assert!(!z.overflowing_add_assign(&y))),
            calls(&[(Add, 1)])
        );
        // constants and the byte conversions take none
        let bytes = [0xa5; 32];
        assert_eq!(
            delegations(|| {
                let _ = delegated::U256::zero();
                let _ = delegated::U256::one();
                assert_eq!(delegated::U256::from_be_bytes(&bytes).to_be_bytes(), bytes);
            }),
            [0; 8]
        );
    }
}
