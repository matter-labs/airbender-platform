// Ported from zksync-os 185a0568:supporting_crates/u256/src/lib.rs
//! The 256-bit digits of the single-digit modulus path of ModExp (zksync-os `u256::U256`): the
//! delegated `U256` on RV32 with the bigint delegation, whose delegation sequences the cycle and
//! delegation counts of the guest depend on, and the naive one otherwise. Test builds compile
//! both, compare them, and run ModExp on the delegated one (on the host emulation of the
//! delegation, which counts its calls).

// Custom types below are NOT Copy in Rust's sense, even though Clone internally would use copy

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
    use ruint::aliases::U256 as HostU256;

    fn from_limbs(limbs: [u64; 4]) -> (naive::U256, delegated::U256) {
        (
            naive::U256::from_limbs(limbs),
            delegated::U256::from_limbs(limbs),
        )
    }

    #[test]
    fn compare_arithmetic() {
        proptest!(|(x_limbs: [u64; 4], y_limbs: [u64; 4])| {
            let (mut x1, mut x2) = from_limbs(x_limbs);
            let (y1, y2) = from_limbs(y_limbs);

            let carry1 = x1.overflowing_add_assign(&y1);
            let carry2 = x2.overflowing_add_assign(&y2);

            prop_assert_eq!(x1.as_limbs(), x2.as_limbs());
            prop_assert_eq!(carry1, carry2);
        });

        proptest!(|(x_limbs: [u64; 4], y_limbs: [u64; 4])| {
            let (mut x1, mut x2) = from_limbs(x_limbs);
            let (y1, y2) = from_limbs(y_limbs);

            let borrow1 = x1.overflowing_sub_assign(&y1);
            let borrow2 = x2.overflowing_sub_assign(&y2);

            prop_assert_eq!(x1.as_limbs(), x2.as_limbs());
            prop_assert_eq!(borrow1, borrow2);
        });

        proptest!(|(x_limbs: [u64; 4], y_limbs: [u64; 4], borrow: bool)| {
            let (mut x1, mut x2) = from_limbs(x_limbs);
            let (y1, y2) = from_limbs(y_limbs);

            let borrow1 = x1.overflowing_sub_assign_with_borrow_propagation(&y1, borrow);
            let borrow2 = x2.overflowing_sub_assign_with_borrow_propagation(&y2, borrow);

            prop_assert_eq!(x1.as_limbs(), x2.as_limbs());
            prop_assert_eq!(borrow1, borrow2);
            let (expected, of1) = HostU256::from_limbs(x_limbs).overflowing_sub(HostU256::from_limbs(y_limbs));
            let (expected, of2) = expected.overflowing_sub(HostU256::from(borrow as u64));
            prop_assert_eq!(x1.as_limbs(), expected.as_limbs());
            prop_assert_eq!(borrow1, of1 | of2);
        });

        // the delegated multiplication reads the multiplicand from the slot of the high half too (the
        // naive one does not): its callers copy it there first
        proptest!(|(x_limbs: [u64; 4], y_limbs: [u64; 4])| {
            let (mut x1, mut x2) = from_limbs(x_limbs);
            let (y1, y2) = from_limbs(y_limbs);
            let (mut high1, mut high2) = (x1.clone(), x2.clone());

            x1.widening_mul_assign_into(&mut high1, &y1);
            x2.widening_mul_assign_into(&mut high2, &y2);

            prop_assert_eq!(x1.as_limbs(), x2.as_limbs());
            prop_assert_eq!(high1.as_limbs(), high2.as_limbs());
            let product = HostU256::from_limbs(x_limbs).widening_mul::<256, 4, 512, 8>(HostU256::from_limbs(y_limbs));
            prop_assert_eq!(&x1.as_limbs()[..], &product.as_limbs()[..4]);
            prop_assert_eq!(&high1.as_limbs()[..], &product.as_limbs()[4..]);
        });

        proptest!(|(x_limbs: [u64; 4], y_limbs: [u64; 4])| {
            let (x1, x2) = from_limbs(x_limbs);
            let (y1, y2) = from_limbs(y_limbs);

            prop_assert_eq!(x1 < y1, x2 < y2);
            prop_assert_eq!(x1 == y1, x2 == y2);
            prop_assert_eq!(x1.cmp(&y1), x2.cmp(&y2));
            prop_assert_eq!(x2.cmp(&x2.clone()), core::cmp::Ordering::Equal);
        });

        proptest!(|(x_limbs: [u64; 4])| {
            let (x1, x2) = from_limbs(x_limbs);
            let (mut slot1, mut slot2) = (MaybeUninit::uninit(), MaybeUninit::uninit());
            // SAFETY: the slots are aligned and writable
            let (copy1, copy2) = unsafe {
                naive::U256::write_into_ptr_unchecked(slot1.as_mut_ptr(), &x1);
                delegated::U256::write_into_ptr_unchecked(slot2.as_mut_ptr(), &x2);
                (slot1.assume_init(), slot2.assume_init())
            };
            prop_assert_eq!(copy1.as_limbs(), &x_limbs);
            prop_assert_eq!(copy2.as_limbs(), &x_limbs);
            let (clone1, clone2) = (x1.clone(), x2.clone());
            prop_assert_eq!(clone1.as_limbs(), clone2.as_limbs());
            let (host1, host2): (HostU256, HostU256) = (x1.into(), x2.into());
            prop_assert_eq!(host1, HostU256::from_limbs(x_limbs));
            prop_assert_eq!(host2, HostU256::from_limbs(x_limbs));
            let (back1, back2) = (naive::U256::from(host1), delegated::U256::from(host2));
            prop_assert_eq!(back1.as_limbs(), &x_limbs);
            prop_assert_eq!(back2.as_limbs(), &x_limbs);
        });
    }

    #[test]
    fn compare_bytes() {
        proptest!(|(bytes: [u8; 32])| {
            let bytes1 = naive::U256::from_be_bytes(&bytes).to_be_bytes();
            let bytes2 = delegated::U256::from_be_bytes(&bytes).to_be_bytes();

            prop_assert_eq!(bytes1, bytes);
            prop_assert_eq!(bytes1, bytes2);
            let (x2, host) = (delegated::U256::from_be_bytes(&bytes), HostU256::from_be_bytes(bytes));
            prop_assert_eq!(x2.as_limbs(), host.as_limbs());
        });
    }

    #[test]
    fn compare_is_zero_is_one() {
        proptest!(|(limbs: [u64; 4])| {
            let (x1, x2) = from_limbs(limbs);

            prop_assert_eq!(x1.is_one(), x2.is_one());
            prop_assert_eq!(x1.is_zero(), x2.is_zero());
        });

        assert!(naive::U256::zero().is_zero());
        assert!(delegated::U256::zero().is_zero());

        assert!(naive::U256::one().is_one());
        assert!(delegated::U256::one().is_one());
        assert!(!naive::U256::one().is_zero() && !naive::U256::zero().is_one());
        assert!(!delegated::U256::one().is_zero() && !delegated::U256::zero().is_one());
    }

    /// The delegation calls of `f`, by operation
    fn delegations(f: impl FnOnce()) -> [u64; 8] {
        DELEGATION_CALLS_BY_OP.with(|calls| *calls.borrow_mut() = [0; 8]);
        f();
        DELEGATION_CALLS_BY_OP.with(|calls| *calls.borrow())
    }

    /// `n` calls of the operation `op`
    fn calls(ops: &[(BigIntOps, u64)]) -> [u64; 8] {
        let mut calls = [0; 8];
        for (op, n) in ops {
            calls[*op as usize] += n;
        }
        calls
    }

    /// The delegation sequences of zksync-os's RISC-V `U256`, which the cycle and delegation
    /// counts of the guest depend on
    #[test]
    fn delegated_operations_take_the_delegations_of_zksync_os() {
        use BigIntOps::*;
        let x = delegated::U256::from_limbs([5, 6, 7, 8]);
        let y = delegated::U256::from_limbs([1, 2, 3, 4]);
        assert_eq!(delegations(|| drop(x.clone())), calls(&[(MemCpy, 1)]));
        assert_eq!(delegations(|| assert!(!x.is_zero())), calls(&[(Eq, 1)]));
        assert_eq!(delegations(|| assert!(!x.is_one())), calls(&[(Eq, 1)]));
        // a copy into the scratch, a comparison, and a subtraction unless equal
        assert_eq!(
            delegations(|| assert!(y < x)),
            calls(&[(MemCpy, 1), (Eq, 1), (Sub, 1)])
        );
        assert_eq!(
            delegations(|| assert!(x >= x.clone())),
            calls(&[(MemCpy, 2), (Eq, 1)])
        );
        assert_eq!(delegations(|| assert!(x != y)), calls(&[(Eq, 1)]));
        let (mut low, mut high) = (x.clone(), x.clone());
        assert_eq!(
            delegations(|| low.widening_mul_assign_into(&mut high, &y)),
            calls(&[(MulLow, 1), (MulHigh, 1)])
        );
        let mut z = x.clone();
        assert_eq!(
            delegations(|| assert!(!z.overflowing_sub_assign(&y))),
            calls(&[(Sub, 1)])
        );
        assert_eq!(
            delegations(|| assert!(!z.overflowing_sub_assign_with_borrow_propagation(&y, true))),
            calls(&[(Sub, 1)])
        );
        assert_eq!(
            delegations(|| assert!(!z.overflowing_add_assign(&y))),
            calls(&[(Add, 1)])
        );
        let mut slot = MaybeUninit::uninit();
        assert_eq!(
            // SAFETY: the slot is aligned and writable
            delegations(|| unsafe {
                delegated::U256::write_into_ptr_unchecked(slot.as_mut_ptr(), &x)
            }),
            calls(&[(MemCpy, 1)])
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

    /// The comparisons of threads running at the same time each use their own scratch
    #[test]
    fn comparisons_on_parallel_threads() {
        let threads: std::vec::Vec<_> = (0..8u64)
            .map(|seed| {
                std::thread::spawn(move || {
                    for i in 0..2000u64 {
                        let a = delegated::U256::from_limbs([i ^ seed, seed, i, 1]);
                        let b = delegated::U256::from_limbs([i, seed, i ^ seed, 1]);
                        let (host_a, host_b): (HostU256, HostU256) =
                            (a.clone().into(), b.clone().into());
                        assert_eq!(a.cmp(&b), host_a.cmp(&host_b));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }
}
