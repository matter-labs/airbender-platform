//! The bn254 and bls12-381 field elements in the hints: their component order ([`HintEncoding`])
//! and their encoding in answers (the `HintAnswer` impls).

use super::answer::HintAnswer;
#[cfg(any(feature = "alloc", test))]
use super::answer::HintTarget;
use super::oracle::{HintError, HintOracle};
#[cfg(any(feature = "alloc", test))]
use alloc::vec::Vec;
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
use ark_ff::{BigInteger, PrimeField};
use ark_ff::{CubicExtConfig, CubicExtField, Field, QuadExtConfig, QuadExtField};
use core::mem::MaybeUninit;

/// The base prime field components of an element in a fixed order (`c0`, `c1`, `c2`,
/// recursively), for guest operands and for answers on every target. Not the arkworks
/// `to_base_prime_field_elements` order, which the delegated and native towers disagree on.
pub trait HintEncoding: Field {
    /// 64-bit limbs of a base field element on the guest with `bigint_ops`: 4 for bn254, 8 for
    /// bls12-381 (381 bits in 512)
    const LIMBS: usize;
    /// Base prime field components of an element
    const COMPONENTS: usize;
    /// Limbs of a whole element on the RISC-V guest
    const TOTAL_LIMBS: usize = Self::LIMBS * Self::COMPONENTS;

    fn from_components(components: &[Self::BasePrimeField]) -> Self;
}

macro_rules! prime_field_encoding {
    ($field:ty, $limbs:expr) => {
        impl HintEncoding for $field {
            const LIMBS: usize = $limbs;
            const COMPONENTS: usize = 1;

            fn from_components(components: &[Self::BasePrimeField]) -> Self {
                components[0]
            }
        }
    };
}

macro_rules! fp12_encoding {
    ($field:ty, $fp6:ty, $fp2:ty, $limbs:expr) => {
        impl HintEncoding for $field {
            const LIMBS: usize = $limbs;
            const COMPONENTS: usize = 12;

            fn from_components(c: &[Self::BasePrimeField]) -> Self {
                let fp2 = |i: usize| <$fp2>::new(c[i], c[i + 1]);
                let fp6 = |i: usize| <$fp6>::new(fp2(i), fp2(i + 2), fp2(i + 4));
                Self::new(fp6(0), fp6(6))
            }
        }
    };
}

macro_rules! fp6_encoding {
    ($field:ty, $fp2:ty, $limbs:expr) => {
        impl HintEncoding for $field {
            const LIMBS: usize = $limbs;
            const COMPONENTS: usize = 6;

            fn from_components(c: &[Self::BasePrimeField]) -> Self {
                let fp2 = |i: usize| <$fp2>::new(c[i], c[i + 1]);
                Self::new(fp2(0), fp2(2), fp2(4))
            }
        }
    };
}

macro_rules! fp2_encoding {
    ($field:ty, $limbs:expr) => {
        impl HintEncoding for $field {
            const LIMBS: usize = $limbs;
            const COMPONENTS: usize = 2;

            fn from_components(c: &[Self::BasePrimeField]) -> Self {
                Self::new(c[0], c[1])
            }
        }
    };
}

prime_field_encoding!(crate::bn254::Fq, 4);
fp2_encoding!(crate::bn254::Fq2, 4);
fp2_encoding!(crate::bls12_381::Fq2, 8);
prime_field_encoding!(crate::bls12_381::Fq, 8);
fp6_encoding!(crate::bn254::Fq6, crate::bn254::Fq2, 4);
fp6_encoding!(crate::bls12_381::Fq6, crate::bls12_381::Fq2, 8);
fp12_encoding!(crate::bn254::Fq12, crate::bn254::Fq6, crate::bn254::Fq2, 4);
fp12_encoding!(
    crate::bls12_381::Fq12,
    crate::bls12_381::Fq6,
    crate::bls12_381::Fq2,
    8
);

/// Whether the little-endian `value` is below the little-endian `modulus`
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
fn is_below(value: &[u64], modulus: &[u64]) -> bool {
    let (low, high) = value.split_at(value.len().min(modulus.len()));
    if high.iter().any(|limb| *limb != 0) {
        return false;
    }
    for (value, modulus) in low.iter().zip(modulus).rev() {
        if value != modulus {
            return value < modulus;
        }
    }
    false
}

/// Receives a base field element: `x R mod p` on the querier's target (`R = 2^(64 limbs)`), as
/// `wire_limbs` canonical little-endian limbs. It is written into the low limbs of the destination,
/// the limbs above are zeroed, and it is checked to be canonical.
///
/// # Safety
///
/// `limbs` must point to the limbs of an `F`, all of its bytes, valid for writes.
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
#[inline(always)]
unsafe fn read_prime_field<F: PrimeField, O: HintOracle>(
    oracle: &mut O,
    limbs: *mut u64,
    wire_limbs: usize,
) -> Result<(), HintError> {
    let num_limbs = <F::BigInt as BigInteger>::NUM_LIMBS;
    debug_assert!(wire_limbs <= num_limbs);
    // SAFETY: the limbs are aligned for `u32`, and `num_limbs` long
    unsafe {
        oracle.write_words(limbs.cast::<u32>(), 2 * wire_limbs)?;
        limbs.add(wire_limbs).write_bytes(0, num_limbs - wire_limbs);
    }
    // SAFETY: just initialized
    let value = unsafe { core::slice::from_raw_parts(limbs, num_limbs) };
    if !is_below(value, F::MODULUS.as_ref()) {
        return Err(HintError("the hint is not a canonical field element"));
    }
    Ok(())
}

/// Oracle side of `read_prime_field`: `limbs` are `element`'s limbs in this build, `guest_limbs`
/// the limb count on the guest
#[cfg(all(
    any(feature = "alloc", test),
    any(not(target_arch = "riscv32"), feature = "bigint_ops")
))]
fn write_prime_field<F: PrimeField>(
    element: &F,
    limbs: &[u64],
    wire_limbs: usize,
    guest_limbs: usize,
    target: HintTarget,
    response: &mut Vec<u32>,
) {
    let mut push = |limbs: &[u64]| {
        debug_assert!(limbs[wire_limbs..].iter().all(|limb| *limb == 0));
        for limb in &limbs[..wire_limbs] {
            response.push(*limb as u32);
            response.push((*limb >> 32) as u32);
        }
    };
    // the target's Montgomery form: `R = 2^(64 limbs)`
    let target_limbs = match target {
        HintTarget::Native => limbs.len(),
        HintTarget::Guest => guest_limbs,
    };
    if limbs.len() == target_limbs && is_below(limbs, F::MODULUS.as_ref()) {
        // the representation of this build
        push(limbs);
    } else {
        let montgomery_r = F::from(2u64).pow([64 * target_limbs as u64]);
        push((*element * montgomery_r).into_bigint().as_ref());
    }
}

/// `$wire_limbs`: the width of the modulus; `$guest_limbs`: the limbs on the guest
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
macro_rules! prime_field_answer {
    ($field:ty, $wire_limbs:expr, $guest_limbs:expr) => {
        const _: () = {
            // the modulus fits in the answer's limbs (`MODULUS_BIT_SIZE` counts the delegated
            // representation's zero top limbs)
            let modulus = <$field as PrimeField>::MODULUS.0;
            assert!($wire_limbs <= modulus.len());
            let mut i = $wire_limbs;
            while i < modulus.len() {
                assert!(modulus[i] == 0);
                i += 1;
            }
        };

        impl HintAnswer for $field {
            #[inline(always)]
            fn read<'a, O: HintOracle>(
                oracle: &mut O,
                dst: &'a mut MaybeUninit<Self>,
            ) -> Result<&'a mut Self, HintError> {
                let this = dst.as_mut_ptr();
                // SAFETY: the limbs (the other field is zero-sized) are all of `dst`
                unsafe {
                    let limbs = (&raw mut (*this).0 .0).cast::<u64>();
                    read_prime_field::<Self, O>(oracle, limbs, $wire_limbs)?;
                    Ok(dst.assume_init_mut())
                }
            }

            #[cfg(any(feature = "alloc", test))]
            fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
                write_prime_field(
                    self,
                    &self.0 .0,
                    $wire_limbs,
                    $guest_limbs,
                    target,
                    response,
                );
            }
        }
    };
}

#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
prime_field_answer!(crate::bn254::Fq, 4, 4);
// 8 limbs on the guest (the delegation works on 256-bit limbs), 6 natively
#[cfg(any(not(target_arch = "riscv32"), feature = "bigint_ops"))]
prime_field_answer!(crate::bls12_381::Fq, 6, 8);

/// The components in order, as in `HintEncoding` (`c0` before `c1`, recursively).
impl<P: QuadExtConfig> HintAnswer for QuadExtField<P>
where
    P::BaseField: HintAnswer,
{
    #[inline(always)]
    fn read<'a, O: HintOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, HintError> {
        let this = dst.as_mut_ptr();
        // SAFETY: the components are disjoint places inside `dst`, which they initialize
        unsafe {
            P::BaseField::read(oracle, &mut *(&raw mut (*this).c0).cast())?;
            P::BaseField::read(oracle, &mut *(&raw mut (*this).c1).cast())?;
            Ok(dst.assume_init_mut())
        }
    }

    #[cfg(any(feature = "alloc", test))]
    fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
        self.c0.write(target, response);
        self.c1.write(target, response);
    }
}

/// The components in order, as in `HintEncoding` (`c0` before `c1` before `c2`, recursively).
impl<P: CubicExtConfig> HintAnswer for CubicExtField<P>
where
    P::BaseField: HintAnswer,
{
    #[inline(always)]
    fn read<'a, O: HintOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, HintError> {
        let this = dst.as_mut_ptr();
        // SAFETY: as for the quadratic extension
        unsafe {
            P::BaseField::read(oracle, &mut *(&raw mut (*this).c0).cast())?;
            P::BaseField::read(oracle, &mut *(&raw mut (*this).c1).cast())?;
            P::BaseField::read(oracle, &mut *(&raw mut (*this).c2).cast())?;
            Ok(dst.assume_init_mut())
        }
    }

    #[cfg(any(feature = "alloc", test))]
    fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
        self.c0.write(target, response);
        self.c1.write(target, response);
        self.c2.write(target, response);
    }
}
