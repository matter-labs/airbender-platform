//! The 256-bit digits of the single-digit path: delegated on RV32 with `bigint_ops`, naive
//! otherwise. Tests compile both, compare them, and run ModExp on the delegated one.

// `U256` is not `Copy`: on RV32 a clone is a delegated copy

#[cfg(any(not(all(target_arch = "riscv32", feature = "bigint_ops")), test))]
mod naive;

#[cfg(not(any(all(target_arch = "riscv32", feature = "bigint_ops"), test)))]
pub(crate) use self::naive::U256;

#[cfg(any(all(target_arch = "riscv32", feature = "bigint_ops"), test))]
mod delegated;

#[cfg(any(all(target_arch = "riscv32", feature = "bigint_ops"), test))]
pub(crate) use self::delegated::U256;

#[cfg(test)]
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
