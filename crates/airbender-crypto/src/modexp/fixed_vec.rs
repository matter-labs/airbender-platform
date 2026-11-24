//! A vector of fixed capacity on the stack, for the operands of ModExp, whose sizes EIP-7823
//! bounds: no allocator is needed.

use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

/// Up to `N` items of `T`, stored inline. Growing past `N` panics. Not `Copy`: at the sizes of
/// ModExp a copy is a memcpy worth seeing.
///
/// Only the first `len` slots are initialized: a new vector writes nothing (at the sizes of
/// ModExp, filling the whole capacity would be kilobytes per value), and a clone copies the items
/// alone. `T: Copy`, so no slot ever needs dropping.
pub struct FixedVec<T: Copy, const N: usize> {
    /// The first `len` slots are initialized
    items: [MaybeUninit<T>; N],
    len: usize,
}

impl<T: Copy, const N: usize> FixedVec<T, N> {
    /// The capacity, `N`
    pub const CAPACITY: usize = N;

    pub const fn new() -> Self {
        Self {
            items: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }

    /// `n` copies of `item`
    pub fn from_elem(item: T, n: usize) -> Self {
        let mut this = Self::new();
        this.resize(n, item);
        this
    }

    pub fn from_slice(items: &[T]) -> Self {
        let mut this = Self::new();
        this.extend_from_slice(items);
        this
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        N
    }

    #[inline(always)]
    pub fn push(&mut self, item: T) {
        assert!(self.len < N, "the fixed vector is full");
        self.items[self.len].write(item);
        self.len += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: the slot was within the length, so it is initialized
        Some(unsafe { self.items[self.len].assume_init() })
    }

    pub fn extend_from_slice(&mut self, items: &[T]) {
        let end = self.len + items.len();
        assert!(end <= N, "the fixed vector is full");
        // SAFETY: the slots `len..end` are within the capacity, and `items` is another object
        // (it is borrowed while `self` is borrowed mutably)
        unsafe {
            core::ptr::copy_nonoverlapping(
                items.as_ptr(),
                self.items.as_mut_ptr().add(self.len).cast::<T>(),
                items.len(),
            );
        }
        self.len = end;
    }

    /// Sets the length to `n`, with new items set to `item`
    pub fn resize(&mut self, n: usize, item: T) {
        assert!(n <= N, "the fixed vector is full");
        if n > self.len {
            for slot in &mut self.items[self.len..n] {
                slot.write(item);
            }
        }
        self.len = n;
    }

    pub fn truncate(&mut self, n: usize) {
        if n < self.len {
            self.len = n;
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<T: Copy, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> Clone for FixedVec<T, N> {
    /// Copies the items, not the spare capacity
    fn clone(&self) -> Self {
        Self::from_slice(self)
    }

    fn clone_from(&mut self, source: &Self) {
        self.clear();
        self.extend_from_slice(source);
    }
}

impl<T: Copy, const N: usize> Deref for FixedVec<T, N> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &[T] {
        // SAFETY: the first `len` slots are initialized, and `len <= N`
        unsafe { core::slice::from_raw_parts(self.items.as_ptr().cast::<T>(), self.len) }
    }
}

impl<T: Copy, const N: usize> DerefMut for FixedVec<T, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as for `deref`, through an exclusive borrow
        unsafe { core::slice::from_raw_parts_mut(self.items.as_mut_ptr().cast::<T>(), self.len) }
    }
}

impl<T: Copy, const N: usize> AsRef<[T]> for FixedVec<T, N> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T: Copy + PartialEq, const N: usize, U: AsRef<[T]> + ?Sized> PartialEq<U> for FixedVec<T, N> {
    fn eq(&self, other: &U) -> bool {
        **self == *other.as_ref()
    }
}

impl<T: Copy + Eq, const N: usize> Eq for FixedVec<T, N> {}

impl<T: Copy + core::fmt::Debug, const N: usize> core::fmt::Debug for FixedVec<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: Copy, const N: usize> super::TryExtend<T> for FixedVec<T, N> {
    type Error = ();

    fn try_extend<I>(&mut self, iter: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = T>,
    {
        for item in iter {
            if self.len == N {
                return Err(());
            }
            self.items[self.len].write(item);
            self.len += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::TryExtend;
    use super::FixedVec;

    #[test]
    fn grows_within_its_capacity() {
        let mut v = FixedVec::<u8, 4>::new();
        assert!(v.is_empty() && v.capacity() == 4);
        v.push(1);
        v.extend_from_slice(&[2, 3]);
        assert_eq!(v, [1, 2, 3]);
        assert_eq!(v.pop(), Some(3));
        v.resize(4, 9);
        assert_eq!(v, [1, 2, 9, 9]);
        v.truncate(1);
        assert_eq!(*v, [1]);
        assert!(v.try_extend([5, 6, 7]).is_ok());
        assert!(v.try_extend([8]).is_err());
        assert_eq!(v, FixedVec::<u8, 4>::from_slice(&[1, 5, 6, 7]));
        v.clear();
        assert_eq!(v.pop(), None);
        assert_eq!(FixedVec::<u8, 4>::from_elem(3, 2), [3, 3]);
    }

    /// Only the items are ever read: a clone, a comparison and a shrink-then-grow see the
    /// initialized prefix alone (Miri checks that no spare slot is read). The item type has
    /// invalid bit patterns, so a read of an unwritten slot would not go unnoticed.
    #[test]
    fn spare_capacity_is_never_read() {
        use core::num::NonZeroU32;
        let item = |value| NonZeroU32::new(value).unwrap();
        let mut v = FixedVec::<NonZeroU32, 8>::new();
        assert_eq!(v.clone(), [] as [NonZeroU32; 0]);
        v.extend_from_slice(&[item(1), item(2), item(3)]);
        let copy = v.clone();
        assert_eq!(copy, v);
        assert_eq!(copy.len(), 3);
        // shrinking forgets the items, growing writes new ones
        v.truncate(1);
        v.resize(5, item(7));
        assert_eq!(v, [item(1), item(7), item(7), item(7), item(7)]);
        assert_eq!(v.pop(), Some(item(7)));
        let mut target = FixedVec::<NonZeroU32, 8>::from_elem(item(9), 8);
        target.clone_from(&v);
        assert_eq!(target, v);
        v.clear();
        assert!(v.is_empty() && v.iter().next().is_none());
    }

    #[test]
    #[should_panic(expected = "the fixed vector is full")]
    fn extending_past_the_capacity_panics() {
        let mut v = FixedVec::<u8, 3>::from_slice(&[1, 2]);
        v.extend_from_slice(&[3, 4]);
    }

    #[test]
    #[should_panic(expected = "the fixed vector is full")]
    fn pushing_past_the_capacity_panics() {
        let mut v = FixedVec::<u8, 1>::new();
        v.push(1);
        v.push(2);
    }
}
