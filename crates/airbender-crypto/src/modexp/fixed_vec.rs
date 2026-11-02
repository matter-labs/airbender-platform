//! A vector of fixed capacity on the stack, for the operands of ModExp, whose sizes EIP-7823
//! bounds: no allocator is needed.

use core::ops::{Deref, DerefMut};

/// Up to `N` items of `T`, stored inline. Growing past `N` panics. Not `Copy`: at the sizes of
/// ModExp a copy is a memcpy worth seeing.
#[derive(Clone)]
pub struct FixedVec<T: Copy + Default, const N: usize> {
    items: [T; N],
    len: usize,
}

impl<T: Copy + Default, const N: usize> FixedVec<T, N> {
    /// The capacity, `N`
    pub const CAPACITY: usize = N;

    pub fn new() -> Self {
        Self {
            items: [T::default(); N],
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
        self.items[self.len] = item;
        self.len += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.items[self.len])
    }

    pub fn extend_from_slice(&mut self, items: &[T]) {
        let end = self.len + items.len();
        assert!(end <= N, "the fixed vector is full");
        self.items[self.len..end].copy_from_slice(items);
        self.len = end;
    }

    /// Sets the length to `n`, with new items set to `item`
    pub fn resize(&mut self, n: usize, item: T) {
        assert!(n <= N, "the fixed vector is full");
        if n > self.len {
            self.items[self.len..n].fill(item);
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

impl<T: Copy + Default, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, const N: usize> Deref for FixedVec<T, N> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &[T] {
        &self.items[..self.len]
    }
}

impl<T: Copy + Default, const N: usize> DerefMut for FixedVec<T, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.items[..self.len]
    }
}

impl<T: Copy + Default, const N: usize> AsRef<[T]> for FixedVec<T, N> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T: Copy + Default + PartialEq, const N: usize, U: AsRef<[T]> + ?Sized> PartialEq<U>
    for FixedVec<T, N>
{
    fn eq(&self, other: &U) -> bool {
        **self == *other.as_ref()
    }
}

impl<T: Copy + Default + Eq, const N: usize> Eq for FixedVec<T, N> {}

impl<T: Copy + Default + core::fmt::Debug, const N: usize> core::fmt::Debug for FixedVec<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: Copy + Default, const N: usize> super::TryExtend<T> for FixedVec<T, N> {
    type Error = ();

    fn try_extend<I>(&mut self, iter: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = T>,
    {
        for item in iter {
            if self.len == N {
                return Err(());
            }
            self.items[self.len] = item;
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

    #[test]
    #[should_panic(expected = "the fixed vector is full")]
    fn pushing_past_the_capacity_panics() {
        let mut v = FixedVec::<u8, 1>::new();
        v.push(1);
        v.push(2);
    }
}
