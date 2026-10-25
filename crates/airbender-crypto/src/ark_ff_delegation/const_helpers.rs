use ark_serialize::{Read, Write};
use ark_std::ops::{Index, IndexMut};

use super::BigInt;

/// A helper macro for emulating `for` loops in a `const` context.
/// # Usage
/// ```rust
/// # use ark_ff::const_for;
/// const fn for_in_const() {
///     let mut array = [0usize; 4];
///     const_for!((i in 0..(array.len())) { // We need to wrap the `array.len()` in parenthesis.
///         array[i] = i;
///     });
///     assert!(array[0] == 0);
///     assert!(array[1] == 1);
///     assert!(array[2] == 2);
///     assert!(array[3] == 3);
/// }
/// ```
#[macro_export]
macro_rules! const_for {
    (($i:ident in $start:tt..$end:tt)  $code:expr ) => {{
        let mut $i = $start;
        while $i < $end {
            $code
            $i += 1;
        }
    }};
}

/// A buffer to hold values of size 8 * N + 1 bytes. This is mostly
/// a hack that's necessary until `generic_const_exprs` is stable.
#[derive(Copy, Clone)]
#[repr(C, align(1))]
pub(super) struct SerBuffer<const N: usize> {
    pub(super) buffers: [[u8; 8]; N],
    pub(super) last: u8,
}

impl<const N: usize> SerBuffer<N> {
    pub(super) const fn zeroed() -> Self {
        Self {
            buffers: [[0u8; 8]; N],
            last: 0u8,
        }
    }

    #[inline(always)]
    pub(super) const fn get(&self, index: usize) -> &u8 {
        if index == 8 * N {
            &self.last
        } else {
            let part = index / 8;
            let in_buffer_index = index % 8;
            &self.buffers[part][in_buffer_index]
        }
    }

    #[inline(always)]
    pub(super) fn get_mut(&mut self, index: usize) -> &mut u8 {
        if index == 8 * N {
            &mut self.last
        } else {
            let part = index / 8;
            let in_buffer_index = index % 8;
            &mut self.buffers[part][in_buffer_index]
        }
    }

    #[inline(always)]
    pub(super) fn copy_from_u8_slice(&mut self, other: &[u8]) {
        other.chunks(8).enumerate().for_each(|(i, chunk)| {
            if i < N {
                self.buffers[i][..chunk.len()].copy_from_slice(chunk);
            } else {
                self.last = chunk[0]
            }
        });
    }

    #[inline(always)]
    pub(super) fn copy_from_u64_slice(&mut self, other: &[u64; N]) {
        other
            .iter()
            .zip(&mut self.buffers)
            .for_each(|(other, this)| *this = other.to_le_bytes());
    }

    #[inline(always)]
    pub(super) fn to_bigint(self) -> BigInt<N> {
        let mut self_integer = BigInt::from(0u64);
        self_integer
            .0
            .iter_mut()
            .zip(self.buffers)
            .for_each(|(other, this)| *other = u64::from_le_bytes(this));
        self_integer
    }

    #[inline(always)]
    /// Write up to `num_bytes` bytes from `self` to `other`.
    /// `num_bytes` may range from zero to `8 * N + 1`, including padded fields.
    pub(super) fn write_up_to(
        &self,
        mut other: impl Write,
        num_bytes: usize,
    ) -> ark_std::io::Result<()> {
        debug_assert!(num_bytes <= 8 * N + 1, "index too large");
        // Delegated fields may have more storage limbs than their canonical encoding.
        let mut remaining = num_bytes;
        for buffer in &self.buffers {
            if remaining == 0 {
                break;
            }
            let len = remaining.min(8);
            other.write_all(&buffer[..len])?;
            remaining -= len;
        }
        if remaining != 0 {
            other.write_all(&[self.last])?;
        }
        Ok(())
    }

    #[inline(always)]
    /// Read up to `num_bytes` bytes from `other` to `self`.
    /// `num_bytes` may range from zero to `8 * N + 1`, including padded fields.
    pub(super) fn read_exact_up_to(
        &mut self,
        mut other: impl Read,
        num_bytes: usize,
    ) -> ark_std::io::Result<()> {
        debug_assert!(num_bytes <= 8 * N + 1, "index too large");
        // Delegated fields may have more storage limbs than their canonical encoding.
        let mut remaining = num_bytes;
        for buffer in &mut self.buffers {
            if remaining == 0 {
                break;
            }
            let len = remaining.min(8);
            other.read_exact(&mut buffer[..len])?;
            remaining -= len;
        }
        if remaining != 0 {
            let mut last = [0];
            other.read_exact(&mut last)?;
            self.last = last[0];
        }
        Ok(())
    }
}

impl<const N: usize> Index<usize> for SerBuffer<N> {
    type Output = u8;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}

impl<const N: usize> IndexMut<usize> for SerBuffer<N> {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
    }
}

pub(super) struct RBuffer<const N: usize>(pub [u64; N], pub u64);

impl<const N: usize> RBuffer<N> {
    /// Find the number of bits in the binary decomposition of `self`.
    pub(super) const fn num_bits(&self) -> u32 {
        (N * 64) as u32 + (64 - self.1.leading_zeros())
    }

    /// Returns the `i`-th bit where bit 0 is the least significant one.
    /// In other words, the bit with weight `2^i`.
    pub(super) const fn get_bit(&self, i: usize) -> bool {
        let d = i / 64;
        let b = i % 64;
        if d == N {
            (self.1 >> b) & 1 == 1
        } else {
            (self.0[d] >> b) & 1 == 1
        }
    }
}

pub(super) struct R2Buffer<const N: usize>(pub [u64; N], pub [u64; N], pub u64);

impl<const N: usize> R2Buffer<N> {
    /// Find the number of bits in the binary decomposition of `self`.
    pub(super) const fn num_bits(&self) -> u32 {
        ((2 * N) * 64) as u32 + (64 - self.2.leading_zeros())
    }

    /// Returns the `i`-th bit where bit 0 is the least significant one.
    /// In other words, the bit with weight `2^i`.
    pub(super) const fn get_bit(&self, i: usize) -> bool {
        let d = i / 64;
        let b = i % 64;
        if d == 2 * N {
            (self.2 >> b) & 1 == 1
        } else if d >= N {
            (self.1[d - N] >> b) & 1 == 1
        } else {
            (self.0[d] >> b) & 1 == 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_bls_storage_uses_reference_canonical_encoding() {
        use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
        let value = crate::bls12_381::Fq::from(17u64);
        let mut actual = Vec::new();
        value.serialize_compressed(&mut actual).unwrap();
        let mut expected = Vec::new();
        ark_bls12_381::Fq::from(17u64)
            .serialize_compressed(&mut expected)
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            crate::bls12_381::Fq::deserialize_compressed(actual.as_slice()).unwrap(),
            value
        );
    }

    #[test]
    fn padded_bls_sampling_and_random_bytes_match_reference() {
        use ark_ff::{BigInteger, Field, PrimeField, UniformRand};
        assert_eq!(crate::bls12_381::Fq::MODULUS_BIT_SIZE, 381);
        for bytes in [[0; 48], [1; 48], [0xff; 48], [0x42; 48]] {
            let ours = crate::bls12_381::Fq::from_random_bytes(&bytes)
                .map(|x| x.into_bigint().to_bytes_le());
            let reference =
                ark_bls12_381::Fq::from_random_bytes(&bytes).map(|x| x.into_bigint().to_bytes_le());
            assert_eq!(ours.as_deref().map(|x| &x[..48]), reference.as_deref());
        }
        let mut rng = ark_std::test_rng();
        for _ in 0..8 {
            let value = crate::bls12_381::Fq::rand(&mut rng);
            assert!(value.into_bigint() < crate::bls12_381::Fq::MODULUS);
        }
    }

    #[test]
    fn test_const_for_macro() {
        let mut array = [0usize; 4];
        const_for!((i in 0..(array.len())) {
            array[i] = i;
        });
        assert_eq!(array, [0, 1, 2, 3]);
    }

    #[test]
    fn test_ser_buffer_zeroed_and_get() {
        type Ser = SerBuffer<2>;
        let buf = Ser::zeroed();
        assert_eq!(*buf.get(0), 0);
        assert_eq!(*buf.get(15), 0);
        assert_eq!(*buf.get(16), 0); // Check the `last` byte
    }

    #[test]
    fn test_ser_buffer_copy_from_u8_slice() {
        type Ser = SerBuffer<2>;
        let mut buf = Ser::zeroed();
        let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
        buf.copy_from_u8_slice(data);

        assert_eq!(buf.buffers[0], [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(buf.buffers[1], [9, 10, 11, 12, 13, 14, 15, 16]);
        assert_eq!(buf.last, 17);
    }

    #[test]
    fn test_ser_buffer_copy_from_u64_slice() {
        type Ser = SerBuffer<2>;
        let mut buf = Ser::zeroed();
        let data: &[u64; 2] = &[0x123456789ABCDEF0, 0x0FEDCBA987654321];
        buf.copy_from_u64_slice(data);

        assert_eq!(buf.buffers[0], 0x123456789ABCDEF0u64.to_le_bytes());
        assert_eq!(buf.buffers[1], 0x0FEDCBA987654321u64.to_le_bytes());
    }

    #[test]
    fn test_rbuffer_get_bit() {
        // Create an instance of RBuffer
        let buf = RBuffer([0x0, 0x8000000000000000], 0x1); // Second value has MSB set, and last has LSB set

        assert!(!buf.get_bit(63)); // Check the 63rd bit of the first part
        assert!(buf.get_bit(127)); // Check the MSB of the second part
        assert!(buf.get_bit(128)); // Check the LSB of the third part
    }

    #[test]
    fn test_ser_buffer_write_and_read() {
        type Ser = SerBuffer<2>;
        let buf = Ser::zeroed();
        let mut data = vec![];
        buf.write_up_to(&mut data, 16)
            .expect("Failed to write buffer");

        let mut new_buf = Ser::zeroed();
        new_buf
            .read_exact_up_to(&data[..], 16)
            .expect("Failed to read buffer");

        assert_eq!(buf.buffers, new_buf.buffers);
        assert_eq!(buf.last, new_buf.last);
    }
}
