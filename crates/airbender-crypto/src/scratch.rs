//! Scratch buffers of fixed capacity, for the inputs the pairing checks and the MSMs parse: the
//! transaction gas cap (EIP-7825, see `precompiles::MAX_TX_GAS`) bounds them, so each endpoint
//! takes a buffer of that capacity on the stack ([`with_buffer`]) instead of allocating.

use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

/// A vector of fixed capacity over borrowed slots: the `len` first ones are initialized.
pub struct SliceVec<'a, T: Copy> {
    slots: &'a mut [MaybeUninit<T>],
    len: usize,
}

impl<'a, T: Copy> SliceVec<'a, T> {
    pub fn new(slots: &'a mut [MaybeUninit<T>]) -> Self {
        Self { slots, len: 0 }
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends `item`; panics if the buffer is full.
    #[inline(always)]
    pub fn push(&mut self, item: T) {
        assert!(self.len < self.slots.len(), "the scratch buffer is full");
        self.slots[self.len].write(item);
        self.len += 1;
    }

    /// Sets the length to `n`, every item `item`; panics if `n` exceeds the capacity.
    pub fn fill(&mut self, item: T, n: usize) {
        assert!(n <= self.slots.len(), "the scratch buffer is full");
        for slot in &mut self.slots[..n] {
            slot.write(item);
        }
        self.len = n;
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<T: Copy> Deref for SliceVec<'_, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &[T] {
        // SAFETY: the first `len` slots are initialized (`push` and `fill`)
        unsafe { core::slice::from_raw_parts(self.slots.as_ptr().cast::<T>(), self.len) }
    }
}

impl<T: Copy> DerefMut for SliceVec<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as for `deref`
        unsafe { core::slice::from_raw_parts_mut(self.slots.as_mut_ptr().cast::<T>(), self.len) }
    }
}

/// Runs `f` on an empty [`SliceVec`] of `N` items of `T`, uninitialized on the stack.
#[inline(always)]
pub fn with_buffer<T: Copy, const N: usize, R>(f: impl FnOnce(&mut SliceVec<'_, T>) -> R) -> R {
    let mut slots = [const { MaybeUninit::<T>::uninit() }; N];
    f(&mut SliceVec::new(&mut slots))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_buffer_is_a_fixed_capacity_vector() {
        let sum = with_buffer::<u32, 4, _>(|words| {
            assert!(words.is_empty() && words.capacity() == 4);
            words.push(1);
            words.push(2);
            assert_eq!(&**words, [1, 2]);
            words[0] = 5;
            words.fill(3, 4);
            assert_eq!(words.len(), 4);
            let sum: u32 = words.iter().sum();
            words.clear();
            assert!(words.is_empty());
            sum
        });
        assert_eq!(sum, 12);
    }

    #[test]
    #[should_panic(expected = "the scratch buffer is full")]
    fn pushing_past_the_capacity_panics() {
        with_buffer::<u32, 4, _>(|words| words.fill(0, 5));
    }
}
