//! The operand of a hint query as the oracle reads it, in the querier's memory and
//! representation, and the responses it writes.

use super::read_querier_u32_words;
use crate::hints::{guest_layout, HintAnswer, HintEncoding, HintTarget, Secp256k1Element};
use airbender_oracle::QuerierMemory;
use alloc::vec::Vec;
use ark_ec::short_weierstrass::{Affine, SWCurveConfig};
use ark_ff::PrimeField;

/// The responses asked for: the answer as a native querier reads it, and as the guest does
pub struct Responses<'a> {
    pub native_run: Option<&'a mut Vec<u32>>,
    pub guest_run: Option<&'a mut Vec<u32>>,
}

impl Responses<'_> {
    pub fn write<T: HintAnswer>(&mut self, answer: &T) {
        if let Some(response) = self.native_run.as_deref_mut() {
            answer.write(HintTarget::Native, response);
        }
        if let Some(response) = self.guest_run.as_deref_mut() {
            answer.write(HintTarget::Guest, response);
        }
    }
}

/// The operand of a hint query, in the querier's memory and representation
pub struct Operand<'m> {
    memory: &'m dyn QuerierMemory,
    querier: HintTarget,
    address: usize,
    len_u32_words: u32,
}

impl<'m> Operand<'m> {
    /// # Safety
    ///
    /// For a native querier, `address` must point to `len_u32_words` words of live values of the
    /// op's operand types, alive and unchanged while the operand is used: they are read in place.
    /// This says where the values are, not what they are: each read that names a type
    /// ([`native_value`](Self::native_value) and the readers over it) is `unsafe` for that.
    pub unsafe fn new(
        memory: &'m dyn QuerierMemory,
        querier: HintTarget,
        address: usize,
        len_u32_words: u32,
    ) -> Self {
        assert!(address != 0, "the operand is not at the null address");
        Self {
            memory,
            querier,
            address,
            len_u32_words,
        }
    }

    /// The target of the querier
    pub fn querier(&self) -> HintTarget {
        self.querier
    }

    /// The size of the operand in bytes
    pub fn size(&self) -> usize {
        self.len_u32_words as usize * size_of::<u32>()
    }

    /// The address of the operand of a querier in this process as values of `T`, checked to be
    /// aligned for them
    fn native_ptr<T>(&self) -> *const T {
        const { assert!(size_of::<T>() != 0, "operands are not zero-sized") };
        assert_eq!(
            self.querier,
            HintTarget::Native,
            "the querier is in this process"
        );
        assert!(
            self.address.is_multiple_of(align_of::<T>()),
            "the operand is aligned for the type"
        );
        core::ptr::with_exposed_provenance::<T>(self.address)
    }

    /// The operand of a querier in this process, a copy of its values of `T`.
    ///
    /// # Safety
    ///
    /// The whole operand must be live, initialized, valid values of `T`: the type the querier
    /// wrote there, which is the operand type of the op (the promise made to [`new`](Self::new),
    /// for the type named here). Its size and alignment are checked, its content cannot be: a
    /// `bool`, an enum or a pointer inside `T` is taken at the querier's word.
    ///
    /// `T: 'static` keeps a borrow from being copied out of the querier's memory with a lifetime
    /// that nothing ties to it. It does not make a `'static` reference inside `T` any safer than
    /// the promise above.
    pub unsafe fn native_values<T: Copy + 'static>(&self) -> Vec<T> {
        let values = self.native_ptr::<T>();
        assert!(
            self.size().is_multiple_of(size_of::<T>()),
            "the operand is values of the type"
        );
        // SAFETY: the operand is live values of `T` (the caller's obligation), aligned (checked
        // by `native_ptr`), and this many of them fill its size
        unsafe { core::slice::from_raw_parts(values, self.size() / size_of::<T>()) }.to_vec()
    }

    /// The operand of a querier in this process, a copy of its one `T`, read in place (no
    /// intermediate vector).
    ///
    /// # Safety
    ///
    /// As for [`native_values`](Self::native_values), for an operand that is one `T`.
    pub unsafe fn native_value<T: Copy + 'static>(&self) -> T {
        let value = self.native_ptr::<T>();
        assert_eq!(
            self.size(),
            size_of::<T>(),
            "the operand is a value of the type"
        );
        // SAFETY: the operand is a live `T` (the caller's obligation), aligned (checked by
        // `native_ptr`) and of its size (checked above)
        unsafe { value.read() }
    }

    /// `len` words at `offset` bytes into the operand of the RISC-V guest
    pub fn guest_words(&self, offset: usize, len: usize) -> Vec<u32> {
        assert_eq!(self.querier, HintTarget::Guest);
        assert!(offset + 4 * len <= self.size(), "within the operand");
        read_querier_u32_words(self.memory, self.address + offset, len)
            .expect("must read the operand")
    }

    /// The byte at `offset` into the operand of the RISC-V guest
    pub fn guest_byte(&self, offset: usize) -> u8 {
        self.guest_words(offset & !3, 1)[0].to_le_bytes()[offset & 3]
    }

    /// The field element at `offset` bytes into the operand of the RISC-V guest (see
    /// `hints::guest_layout`)
    pub fn guest_element<F: HintEncoding>(&self, offset: usize) -> F {
        let words = self.guest_words(offset, 2 * F::TOTAL_LIMBS);
        let components: Vec<F::BasePrimeField> = words
            .chunks(2 * F::LIMBS)
            .map(from_guest_montgomery)
            .collect();
        F::from_components(&components)
    }

    /// The affine point at `offset` bytes into the operand of the RISC-V guest
    pub fn guest_affine<P: SWCurveConfig>(
        &self,
        offset: usize,
        layout: guest_layout::AffinePoint,
    ) -> Affine<P>
    where
        P::BaseField: HintEncoding,
    {
        if self.guest_byte(offset + layout.infinity) != 0 {
            return Affine::identity();
        }
        Affine::new_unchecked(
            self.guest_element(offset + layout.x),
            self.guest_element(offset + layout.y),
        )
    }

    /// A field element operand
    ///
    /// # Safety
    ///
    /// For a querier in this process, as for [`native_value`](Self::native_value): the operand
    /// must be an `F`. A guest operand is read through the guest's memory only.
    pub unsafe fn element<F: HintEncoding + Copy>(&self) -> F {
        match self.querier {
            // SAFETY: the caller's obligation
            HintTarget::Native => unsafe { self.native_value() },
            HintTarget::Guest => {
                assert_eq!(self.size(), 8 * F::TOTAL_LIMBS, "the operand is an element");
                self.guest_element(0)
            }
        }
    }

    /// A secp256k1 element operand (see `Secp256k1Element`)
    ///
    /// # Safety
    ///
    /// For a querier in this process, as for [`native_value`](Self::native_value): the operand
    /// must be a `T`. A guest operand is read through the guest's memory only.
    pub unsafe fn secp256k1_element<T: Secp256k1Element + 'static>(&self) -> T {
        match self.querier {
            // SAFETY: the caller's obligation
            HintTarget::Native => unsafe { self.native_value() },
            HintTarget::Guest => {
                assert_eq!(self.size(), 32, "the operand is an element");
                let words: [u32; 8] = self.guest_words(0, 8).try_into().expect("8 words");
                T::from_guest_operand_words(&words)
            }
        }
    }

    /// `N` adjacent secp256k1 elements (see `Secp256k1Element`)
    ///
    /// # Safety
    ///
    /// For a querier in this process, as for [`native_value`](Self::native_value): the operand
    /// must be a `[T; N]`. A guest operand is read through the guest's memory only.
    pub unsafe fn secp256k1_elements<T: Secp256k1Element + 'static, const N: usize>(
        &self,
    ) -> [T; N] {
        match self.querier {
            // SAFETY: the caller's obligation
            HintTarget::Native => unsafe { self.native_value() },
            HintTarget::Guest => {
                assert_eq!(self.size(), 32 * N, "the operand is {N} elements");
                core::array::from_fn(|i| {
                    let words: [u32; 8] = self.guest_words(32 * i, 8).try_into().expect("8 words");
                    T::from_guest_operand_words(&words)
                })
            }
        }
    }
}

/// The base field element of its guest Montgomery limbs (`R = 2^(32 words)`), any representative
pub fn from_guest_montgomery<F: PrimeField>(words: &[u32]) -> F {
    let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let r = F::from(2u64).pow([32 * words.len() as u64]);
    F::from_le_bytes_mod_order(&bytes) * r.inverse().expect("a power of two is invertible")
}
