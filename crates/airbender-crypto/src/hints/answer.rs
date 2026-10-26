// Ported from zksync-os e855bca2:basic_system/src/system_functions/field_ops.rs
//! The answers of field hints: how the querier receives them, and how the oracle writes them.

use super::oracle::{HintError, HintOracle};
#[cfg(any(feature = "alloc", test))]
use alloc::vec::Vec;
use core::mem::MaybeUninit;

/// The target of a querier, whose representation of the answers the oracle writes. The
/// representations of field elements differ between the RISC-V guest and native builds, so the
/// answers do, and a native run that records the prover input is answered in both: it reads the
/// answers of its own target, and records those of the guest, which the guest reads in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintTarget {
    /// A querier in this process, i.e. of this build (a native run)
    Native,
    /// The RISC-V guest (the proving target)
    Guest,
}

/// A value that field hints answer with. The oracle writes it straight into its destination, as
/// the `u32` words of its representation on the target of the querier, which
/// [`read`](Self::read) then checks to be well-formed. Whether the value is the right answer is up
/// to the caller to check.
///
/// # Implementations outside this crate
///
/// This trait is meant for the types of this crate. `write` is only there with the
/// `alloc` feature (or in this crate's tests), so the set of items of the trait changes with a
/// feature, which is not additive: if any crate in the build turns `alloc` on, an implementation
/// that does not define `write` stops compiling, and one that does stops compiling without it.
/// The trait is not sealed.
pub trait HintAnswer: Sized {
    /// Receives the value from the response to the current hint query into `dst`, and validates it.
    fn read<'a, O: HintOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, HintError>;

    /// Oracle side: appends the words of the value, as [`read`](Self::read) receives them on
    /// `target`.
    #[cfg(any(feature = "alloc", test))]
    fn write(&self, target: HintTarget, response: &mut Vec<u32>);
}

/// A flag: one word, any non-zero word is `true`.
impl HintAnswer for bool {
    #[inline(always)]
    fn read<'a, O: HintOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, HintError> {
        Ok(dst.write(oracle.read_word()? != 0))
    }

    #[cfg(any(feature = "alloc", test))]
    fn write(&self, _target: HintTarget, response: &mut Vec<u32>) {
        response.push(u32::from(*self));
    }
}

/// The elements in turn.
impl<T: HintAnswer, const N: usize> HintAnswer for [T; N] {
    #[inline(always)]
    fn read<'a, O: HintOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, HintError> {
        let first = dst.as_mut_ptr().cast::<MaybeUninit<T>>();
        for i in 0..N {
            // SAFETY: element `i` is inside `dst`, and `MaybeUninit<T>` has the layout of `T`
            T::read(oracle, unsafe { &mut *first.add(i) })?;
        }
        // SAFETY: every element was just initialized
        Ok(unsafe { dst.assume_init_mut() })
    }

    #[cfg(any(feature = "alloc", test))]
    fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
        for element in self {
            element.write(target, response);
        }
    }
}

macro_rules! impl_hint_answer_for_tuple {
    ($($element:ident $index:tt),+) => {
        /// The elements in turn.
        impl<$($element: HintAnswer),+> HintAnswer for ($($element,)+) {
            #[inline(always)]
            fn read<'a, O: HintOracle>(
                oracle: &mut O,
                dst: &'a mut MaybeUninit<Self>,
            ) -> Result<&'a mut Self, HintError> {
                let this = dst.as_mut_ptr();
                // SAFETY: the elements are disjoint places inside `dst`, and `MaybeUninit<E>` has the
                // layout of `E`
                unsafe {
                    $(
                        $element::read(
                            oracle,
                            &mut *(&raw mut (*this).$index).cast::<MaybeUninit<$element>>(),
                        )?;
                    )+
                    // SAFETY: every element was just initialized
                    Ok(dst.assume_init_mut())
                }
            }

            #[cfg(any(feature = "alloc", test))]
            fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
                $(self.$index.write(target, response);)+
            }
        }
    };
}

impl_hint_answer_for_tuple!(A 0, B 1);
impl_hint_answer_for_tuple!(A 0, B 1, C 2);
impl_hint_answer_for_tuple!(A 0, B 1, C 2, D 3);

/// Receives a value of the answer to the current hint query.
#[inline(always)]
pub fn read_hint_answer<O: HintOracle, R: HintAnswer>(oracle: &mut O) -> Result<R, HintError> {
    let mut value = MaybeUninit::uninit();
    R::read(oracle, &mut value)?;
    // SAFETY: initialized by `read`
    Ok(unsafe { value.assume_init() })
}
