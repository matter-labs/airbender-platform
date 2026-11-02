//! The secp256k1 hints: the elements in the answers, and the hooks of the recovery that use them.

#[cfg(any(feature = "alloc", test))]
use super::answer::HintTarget;
use super::answer::{read_hint_answer, HintAnswer};
use super::request::{send_field_hint_query, FieldHintOp};
use crate::secp256k1::field::FieldElement;
use crate::secp256k1::scalars::Scalar;
use crate::secp256k1::{Affine, RecoveryWitness};
use crate::{bigint_op_delegation_raw, BigIntOps};
use airbender_oracle::{MemoryOracle, OracleError};
#[cfg(any(feature = "alloc", test))]
use alloc::vec::Vec;
use core::mem::MaybeUninit;

/// `2^256 mod p` and `2^256 mod n`, the Montgomery form of 1, in little-endian words
const R_MOD_P_WORDS: [u32; 8] = [0x3d1, 0x1, 0, 0, 0, 0, 0, 0];
const R_MOD_N_WORDS: [u32; 8] = [0x2fc9bebf, 0x402da173, 0x50b75fc4, 0x45512319, 0x1, 0, 0, 0];
/// `2^-256 mod p` and `2^-256 mod n`, big-endian
const R_INVERSE_MOD_P: [u8; 32] = [
    0xc9, 0xbd, 0x19, 0x05, 0x15, 0x53, 0x83, 0x99, 0x9c, 0x46, 0xc2, 0xc2, 0x95, 0xf2, 0xb7, 0x61,
    0xbc, 0xb2, 0x23, 0xfe, 0xdc, 0x24, 0xa0, 0x59, 0xd8, 0x38, 0x09, 0x1d, 0x08, 0x68, 0x19, 0x2a,
];
const R_INVERSE_MOD_N: [u8; 32] = [
    0xd9, 0xe8, 0x89, 0x0d, 0x64, 0x94, 0xef, 0x93, 0x89, 0x7f, 0x30, 0xc1, 0x27, 0xcf, 0xab, 0x5d,
    0x3b, 0xbb, 0xd4, 0x56, 0x7f, 0xa5, 0x0c, 0x3c, 0x80, 0xfd, 0x22, 0x93, 0x80, 0x97, 0xc0, 0x16,
];

const fn words_eq(a: &[u32; 8], b: &[u32; 8]) -> bool {
    let mut i = 0;
    while i < 8 {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The first 8 words of the representation of `value`
///
/// # Safety
///
/// `T` must be at least 32 bytes, aligned for `u32`, without padding in its first 32 bytes.
const unsafe fn leading_words<T>(value: &T) -> [u32; 8] {
    // SAFETY: guaranteed by the caller
    unsafe { *core::ptr::from_ref(value).cast::<[u32; 8]>() }
}

/// Whether the little-endian `value` is below the little-endian `modulus`
fn is_below(value: &[u32; 8], modulus: &[u32; 8]) -> bool {
    for (value, modulus) in value.iter().zip(modulus).rev() {
        if value != modulus {
            return value < modulus;
        }
    }
    false
}

fn be_bytes(words: &[u32; 8]) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (word, out) in words.iter().rev().zip(bytes.as_chunks_mut::<4>().0) {
        *out = word.to_be_bytes();
    }
    bytes
}

fn words_of_be_bytes(bytes: &[u8; 32]) -> [u32; 8] {
    let mut words = [0u32; 8];
    for (word, chunk) in words.iter_mut().rev().zip(bytes.as_chunks::<4>().0) {
        *word = u32::from_be_bytes(*chunk);
    }
    words
}

/// A secp256k1 `FieldElement` (modulo `p`) or `Scalar` (modulo `n`) in the hints. Operands are
/// read in place; answers are 8 little-endian `u32` words in the querier's representation:
/// - on the guest, the Montgomery form `x 2^256 mod m`, which is both types' memory layout there
///   (checked at compile time). Operands may be any 256-bit representative; answers are written
///   straight into the destination and checked to be below the modulus.
/// - natively, the canonical integer, through the types' API.
pub trait Secp256k1Element: Copy {
    /// The modulus, in little-endian words
    const MODULUS: [u32; 8];
    /// `2^256 mod m`, the Montgomery form of 1, in little-endian words
    const MONTGOMERY_ONE: [u32; 8];
    /// `2^-256 mod m`, big-endian
    const R_INVERSE: [u8; 32];
    /// Whether this build's representation is the Montgomery words, as on the guest
    const IS_MONTGOMERY_WORDS: bool;

    /// The canonical big-endian bytes of the element
    fn to_be_bytes(self) -> [u8; 32];

    /// The element of canonical big-endian bytes, `None` for the modulus or above
    fn from_be_bytes(bytes: &[u8; 32]) -> Option<Self>;

    /// The product of the elements
    fn multiply(self, rhs: Self) -> Self;

    /// The words of the canonical integer of the element
    fn to_canonical_words(self) -> [u32; 8] {
        words_of_be_bytes(&self.to_be_bytes())
    }

    /// The element of the words of a canonical integer, `None` for the modulus or above
    fn from_canonical_words(words: &[u32; 8]) -> Option<Self> {
        Self::from_be_bytes(&be_bytes(words))
    }

    /// The Montgomery words of the element, below the modulus
    fn to_montgomery_words(self) -> [u32; 8] {
        let r = Self::from_canonical_words(&Self::MONTGOMERY_ONE).expect("below the modulus");
        self.multiply(r).to_canonical_words()
    }

    /// The element of Montgomery words below the modulus
    fn from_montgomery_words(words: &[u32; 8]) -> Self {
        let r_inverse = Self::from_be_bytes(&Self::R_INVERSE).expect("below the modulus");
        Self::from_canonical_words(words)
            .expect("below the modulus")
            .multiply(r_inverse)
    }

    /// The element of a guest operand's Montgomery words, any 256-bit representative (`2^256 < 2 m`)
    fn from_guest_operand_words(words: &[u32; 8]) -> Self {
        let mut reduced = *words;
        if !is_below(&reduced, &Self::MODULUS) {
            let mut borrow = false;
            for (word, modulus) in reduced.iter_mut().zip(&Self::MODULUS) {
                let (difference, borrow_1) = word.overflowing_sub(*modulus);
                let (difference, borrow_2) = difference.overflowing_sub(u32::from(borrow));
                *word = difference;
                borrow = borrow_1 | borrow_2;
            }
        }
        Self::from_montgomery_words(&reduced)
    }

    /// Checks a guest answer's Montgomery words at `words` are below the modulus, with the bigint
    /// delegation
    ///
    /// # Safety
    ///
    /// `words` must be valid for reads and writes, and 32-byte aligned: a delegation operand.
    unsafe fn validate<E: OracleError>(words: *mut [u32; 8]) -> Result<(), E>;
}

/// A 32-byte aligned operand of the bigint delegation. As an immutable static it lands in
/// `.rodata`, which the linker places above the ROM bound.
#[repr(C, align(32))]
struct DelegationOperand([u32; 8]);

static FIELD_MODULUS: DelegationOperand =
    DelegationOperand(<FieldElement as Secp256k1Element>::MODULUS);
static SCALAR_MODULUS: DelegationOperand = DelegationOperand(<Scalar as Secp256k1Element>::MODULUS);

/// Checks the value at `words` is below `modulus`: subtracting the modulus borrows exactly then,
/// and adding it back restores the value.
///
/// # Safety
///
/// `words` must be valid for reads and writes, and 32-byte aligned.
#[inline(always)]
unsafe fn check_below_with_delegation<E: OracleError>(
    words: *mut [u32; 8],
    modulus: &'static DelegationOperand,
) -> Result<(), E> {
    let modulus = core::ptr::from_ref(modulus).cast::<()>();
    // SAFETY: both operands are distinct 32-byte aligned 32-byte values in RAM, and the delegation
    // only writes the first one
    let borrow = unsafe { bigint_op_delegation_raw(words.cast(), modulus, BigIntOps::Sub) } != 0;
    // SAFETY: as above
    unsafe { bigint_op_delegation_raw(words.cast(), modulus, BigIntOps::Add) };
    if borrow {
        Ok(())
    } else {
        Err(E::new("the secp256k1 hint is not canonical"))
    }
}

impl Secp256k1Element for FieldElement {
    const MODULUS: [u32; 8] = [
        0xfffffc2f, 0xfffffffe, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
        0xffffffff,
    ];
    const MONTGOMERY_ONE: [u32; 8] = R_MOD_P_WORDS;
    const R_INVERSE: [u8; 32] = R_INVERSE_MOD_P;
    const IS_MONTGOMERY_WORDS: bool = core::mem::size_of::<Self>() == 32
        && core::mem::align_of::<Self>() >= core::mem::align_of::<u32>()
        // SAFETY: the element is 32 bytes aligned for `u32` (reading padding would fail const
        // evaluation); `ONE` and `ZERO` tell whether they are the Montgomery words
        && words_eq(unsafe { &leading_words(&Self::ONE) }, &Self::MONTGOMERY_ONE)
        && words_eq(unsafe { &leading_words(&Self::ZERO) }, &[0; 8]);

    fn to_be_bytes(self) -> [u8; 32] {
        self.to_bytes().into()
    }

    fn from_be_bytes(bytes: &[u8; 32]) -> Option<Self> {
        Self::from_bytes(bytes)
    }

    fn multiply(mut self, rhs: Self) -> Self {
        self.mul_in_place(&rhs);
        self
    }

    unsafe fn validate<E: OracleError>(words: *mut [u32; 8]) -> Result<(), E> {
        // SAFETY: guaranteed by the caller
        unsafe { check_below_with_delegation(words, &FIELD_MODULUS) }
    }
}

impl Secp256k1Element for Scalar {
    const MODULUS: [u32; 8] = [
        0xd0364141, 0xbfd25e8c, 0xaf48a03b, 0xbaaedce6, 0xfffffffe, 0xffffffff, 0xffffffff,
        0xffffffff,
    ];
    const MONTGOMERY_ONE: [u32; 8] = R_MOD_N_WORDS;
    const R_INVERSE: [u8; 32] = R_INVERSE_MOD_N;
    const IS_MONTGOMERY_WORDS: bool = core::mem::size_of::<Self>() == 32
        && core::mem::align_of::<Self>() >= core::mem::align_of::<u32>()
        // SAFETY: as for `FieldElement`
        && words_eq(unsafe { &leading_words(&Self::ONE) }, &Self::MONTGOMERY_ONE)
        && words_eq(unsafe { &leading_words(&Self::ZERO) }, &[0; 8]);

    fn to_be_bytes(self) -> [u8; 32] {
        self.to_repr().into()
    }

    fn from_be_bytes(bytes: &[u8; 32]) -> Option<Self> {
        is_below(&words_of_be_bytes(bytes), &Self::MODULUS).then(|| scalar_of_be_bytes(bytes))
    }

    fn multiply(mut self, rhs: Self) -> Self {
        self *= rhs;
        self
    }

    unsafe fn validate<E: OracleError>(words: *mut [u32; 8]) -> Result<(), E> {
        // SAFETY: guaranteed by the caller
        unsafe { check_below_with_delegation(words, &SCALAR_MODULUS) }
    }
}

/// The scalar of a big-endian integer below the order
fn scalar_of_be_bytes(bytes: &[u8; 32]) -> Scalar {
    use crate::k256::elliptic_curve::scalar::FromUintUnchecked;
    Scalar::from_k256_scalar(crate::k256::Scalar::from_uint_unchecked(
        crate::k256::U256::from_be_slice(bytes),
    ))
}

/// Receives an element in this target's representation (see [`Secp256k1Element`]), validated.
#[inline(always)]
fn read_secp256k1_element<'a, T: Secp256k1Element, O: MemoryOracle>(
    oracle: &mut O,
    dst: &'a mut MaybeUninit<T>,
) -> Result<&'a mut T, O::Error> {
    #[cfg(target_arch = "riscv32")]
    {
        // `validate` relies on the element being its Montgomery words, 32-byte aligned
        const { assert!(T::IS_MONTGOMERY_WORDS && core::mem::align_of::<T>() >= 32) };
        let words = dst.as_mut_ptr().cast::<[u32; 8]>();
        // SAFETY: the element is 8 words, 32-byte aligned (checked above), which initialize it
        unsafe {
            oracle.write_words(words.cast::<u32>(), 8)?;
            T::validate(words)?;
            Ok(dst.assume_init_mut())
        }
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        let mut words = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { oracle.write_words(words.as_mut_ptr(), 8)? };
        let element = T::from_canonical_words(&words)
            .ok_or(O::Error::new("the secp256k1 hint is not canonical"))?;
        Ok(dst.write(element))
    }
}

/// Receives a base field quotient, which the caller checks against the fraction. On the guest the
/// words are taken unchecked: the delegated representation accepts any 256-bit representative and
/// the caller's check is modulo `p`, so a representative above `p` is the same element.
#[inline(always)]
fn read_secp256k1_quotient<'a, O: MemoryOracle>(
    oracle: &mut O,
    dst: &'a mut MaybeUninit<FieldElement>,
) -> Result<&'a mut FieldElement, O::Error> {
    #[cfg(target_arch = "riscv32")]
    {
        const {
            assert!(
                <FieldElement as Secp256k1Element>::IS_MONTGOMERY_WORDS
                    && core::mem::align_of::<FieldElement>() >= core::mem::align_of::<u32>()
            )
        };
        // SAFETY: the element is 8 aligned words (checked above), and any value of them is valid
        unsafe {
            oracle.write_words(dst.as_mut_ptr().cast::<u32>(), 8)?;
            Ok(dst.assume_init_mut())
        }
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        read_secp256k1_element(oracle, dst)
    }
}

macro_rules! impl_hint_answer_for_secp256k1_element {
    ($($t:ty),+) => {$(
        impl HintAnswer for $t {
            #[inline(always)]
            fn read<'a, O: MemoryOracle>(
                oracle: &mut O,
                dst: &'a mut MaybeUninit<Self>,
            ) -> Result<&'a mut Self, O::Error> {
                read_secp256k1_element(oracle, dst)
            }

            #[cfg(any(feature = "alloc", test))]
            fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
                response.extend_from_slice(&match target {
                    HintTarget::Native => self.to_canonical_words(),
                    HintTarget::Guest => self.to_montgomery_words(),
                });
            }
        }
    )+};
}

impl_hint_answer_for_secp256k1_element!(FieldElement, Scalar);

/// The operand of [`FieldHintOp::Secp256k1RecoveryWitness`]: the signature `(r, s)` of `message`
/// (the digest reduced modulo the order) and the point `(x, y)` of the signature, with `x = r` and
/// the `y` of the recovery id, each in the querier's representation (see [`Secp256k1Element`]).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RecoveryRequest {
    pub r: Scalar,
    pub s: Scalar,
    pub message: Scalar,
    pub x: FieldElement,
    pub y: FieldElement,
}

// The guest sends the five elements as their Montgomery words, back to back.
#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
const _: () = assert!(core::mem::size_of::<RecoveryRequest>() == 5 * 32);

impl RecoveryRequest {
    /// The request of the witness of the recovery of the key of the signature `(r, s)` of
    /// `message` with the point `point` of the signature
    pub fn new(point: &Affine, r: &Scalar, s: &Scalar, message: &Scalar) -> Self {
        Self {
            r: *r,
            s: *s,
            message: *message,
            x: point.x,
            y: point.y,
        }
    }

    /// The point of the signature
    pub fn point(&self) -> Affine {
        Affine {
            x: self.x,
            y: self.y,
            infinity: false,
        }
    }
}

/// The words of a witness, the same on every target: the coordinates of the key as canonical
/// little-endian words, then each coefficient as the 4 little-endian words of its two's
/// complement. Read unchecked: the recovery checks the witness, and panics if it is wrong.
impl HintAnswer for RecoveryWitness {
    #[inline(always)]
    fn read<'a, O: MemoryOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, O::Error> {
        let mut words = [0u32; 32];
        // SAFETY: the buffer holds 32 words
        unsafe { oracle.write_words(words.as_mut_ptr(), 32)? };
        let (key, coefficients) = words.as_chunks::<8>().0.split_at(2);
        Ok(dst.write(RecoveryWitness {
            key_x: be_bytes(&key[0]),
            key_y: be_bytes(&key[1]),
            coefficients: core::array::from_fn(|i| {
                let words = &coefficients[i / 2][4 * (i % 2)..][..4];
                let mut value = 0u128;
                for (j, word) in words.iter().enumerate() {
                    value |= u128::from(*word) << (32 * j);
                }
                value as i128
            }),
        }))
    }

    #[cfg(any(feature = "alloc", test))]
    fn write(&self, _target: HintTarget, response: &mut Vec<u32>) {
        response.extend_from_slice(&words_of_be_bytes(&self.key_x));
        response.extend_from_slice(&words_of_be_bytes(&self.key_y));
        for coefficient in self.coefficients {
            for word in coefficient.to_le_bytes().as_chunks::<4>().0 {
                response.push(u32::from_le_bytes(*word));
            }
        }
    }
}

/// Secp256k1 hooks implementation that uses an oracle for field operations.
pub struct Secp256k1HooksWithOracle<'a, O: MemoryOracle> {
    oracle: &'a mut O,
}

impl<'a, O: MemoryOracle> Secp256k1HooksWithOracle<'a, O> {
    pub fn new(oracle: &'a mut O) -> Self {
        Self { oracle }
    }
}

impl<'a, O: MemoryOracle> crate::secp256k1::hooks::Secp256k1Hooks
    for Secp256k1HooksWithOracle<'a, O>
{
    /// A hinted inversion costs one multiplication, so the scalar multiplication makes its table
    /// affine
    const FE_INVERT_IS_CHEAP: bool = true;

    /// A hinted division costs one multiplication, so the scalar multiplication works in affine
    /// coordinates
    const FE_DIVIDE_IS_CHEAP: bool = true;

    /// The key and its 4-dimensional decomposition from the oracle, which the recovery checks
    /// with about a fifth fewer group operations than it computes the key with
    fn recovery_witness(
        &mut self,
        point: &Affine,
        r: &Scalar,
        s: &Scalar,
        message: &Scalar,
    ) -> Option<RecoveryWitness> {
        let request = RecoveryRequest::new(point, r, s, message);
        // SAFETY: `RecoveryRequest` is the operand type of the recovery witness
        unsafe {
            send_field_hint_query(self.oracle, FieldHintOp::Secp256k1RecoveryWitness, &request)
        }
        .expect("must send the field hint query");
        let present: bool = read_hint_answer(self.oracle).expect("the hint answer is well-formed");
        let witness =
            present.then(|| read_hint_answer(self.oracle).expect("the hint answer is well-formed"));
        self.oracle
            .finish_query()
            .expect("the hint answer has no excess data");
        witness
    }

    fn fe_divide<'q>(
        &mut self,
        fraction: &mut [FieldElement; 2],
        quotient: &'q mut MaybeUninit<FieldElement>,
    ) -> &'q mut FieldElement {
        // SAFETY: `[FieldElement; 2]` is the operand type of the division
        unsafe {
            send_field_hint_query(
                self.oracle,
                FieldHintOp::Secp256k1BaseFieldDivision,
                fraction,
            )
        }
        .expect("must send the field hint query");
        let quotient =
            read_secp256k1_quotient(self.oracle, quotient).expect("the hint answer is well-formed");
        self.oracle
            .finish_query()
            .expect("the hint answer has no excess data");

        // quotient * denominator == numerator: the denominator is non-zero (the caller's
        // obligation), so only the quotient passes. The fraction is ours to overwrite.
        let [numerator, denominator] = fraction;
        denominator.mul_in_place(quotient);
        denominator.sub_in_place(numerator);
        assert!(denominator.is_zero());

        quotient
    }

    #[allow(clippy::bool_comparison)]
    fn fe_sqrt_and_assign(&mut self, x: &mut FieldElement) -> bool {
        // as the default hooks: sqrt(0) = 0
        if x.is_zero() {
            return true;
        }

        let operand = *x;
        // SAFETY: `FieldElement` is the operand type of the base field sqrt
        let is_quadratic_non_residue: bool = unsafe {
            self.query_into(FieldHintOp::Secp256k1BaseFieldSqrt, x, |oracle| {
                read_hint_answer(oracle)
            })
        };

        // candidate² == x for a square, candidate² == -x otherwise: as p = 3 mod 4, -1 is a
        // non-residue, so a valid answer of either kind proves the flag
        let mut squared = *x;
        squared.square_in_place();
        if is_quadratic_non_residue == false {
            squared.sub_in_place(&operand);
            assert!(squared.is_zero());
        } else {
            squared.add_in_place(&operand);
            assert!(squared.is_zero());
        }

        !is_quadratic_non_residue
    }

    fn fe_invert_and_assign(&mut self, x: &mut FieldElement) {
        // as the default hooks: invert(0) = 0
        if x.is_zero() {
            return;
        }

        let mut product = *x;
        // SAFETY: `FieldElement` is the operand type of the base field inverse
        unsafe { self.query_into(FieldHintOp::Secp256k1BaseFieldInverse, x, |_| Ok(())) };

        product.mul_in_place(x);
        assert!(product.is_one());
    }

    fn scalar_invert_and_assign(&mut self, x: &mut Scalar) {
        // as the default hooks: invert(0) = 0
        if x.is_zero() {
            return;
        }

        let mut product = *x;
        // SAFETY: `Scalar` is the operand type of the scalar field inverse
        unsafe { self.query_into(FieldHintOp::Secp256k1ScalarFieldInverse, x, |_| Ok(())) };

        product *= &*x;
        assert!(product.is_one());
    }
}

impl<'a, O: MemoryOracle> Secp256k1HooksWithOracle<'a, O> {
    /// Asks for the hint `op` on `x` and receives the answer element into `x`, then the rest of
    /// the answer with `rest`. The oracle reads the operand before the answer overwrites it.
    ///
    /// # Safety
    ///
    /// `T` must be the operand type of `op` (see `send_field_hint_query`).
    #[inline(always)]
    unsafe fn query_into<T: Secp256k1Element + HintAnswer, R>(
        &mut self,
        op: FieldHintOp,
        x: &mut T,
        rest: impl FnOnce(&mut O) -> Result<R, O::Error>,
    ) -> R {
        // SAFETY: `x` is a value of the operand type of `op` (the caller's obligation)
        unsafe { send_field_hint_query(self.oracle, op, &*x) }
            .expect("must send the field hint query");
        // SAFETY: `MaybeUninit<T>` has the layout of `T`, and `read` writes a valid `T` (on the
        // guest any words are one), so `x` stays valid even if the answer is rejected
        let dst = unsafe { &mut *core::ptr::from_mut(x).cast::<MaybeUninit<T>>() };
        T::read(self.oracle, dst).expect("the hint answer is well-formed");
        let rest = rest(self.oracle).expect("the hint answer is well-formed");
        self.oracle
            .finish_query()
            .expect("the hint answer has no excess data");
        rest
    }
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    use crate::hints::host::RecordingHintOracle;
    use crate::hints::test_support::{lying_oracle, lying_oracle_last_word, TapeOracle};
    use crate::hints::{HintError, QuerierMemory};
    use crate::secp256k1::hooks::{DefaultSecp256k1Hooks, Secp256k1Hooks};
    use proptest::{prop_assert, prop_assert_eq, proptest};

    /// The integer of little-endian words plus the modulus, if it stays below `2^256`
    fn plus_modulus(words: &[u32; 8], modulus: &[u32; 8]) -> Option<[u32; 8]> {
        let mut sum = [0u32; 8];
        let mut carry = 0u64;
        for ((out, word), modulus) in sum.iter_mut().zip(words).zip(modulus) {
            let t = u64::from(*word) + u64::from(*modulus) + carry;
            *out = t as u32;
            carry = t >> 32;
        }
        (carry == 0).then_some(sum)
    }

    /// The conversions of `x` to and from Montgomery and canonical words
    fn check_round_trip<T: Secp256k1Element>(x: T) {
        let words = x.to_montgomery_words();
        assert!(is_below(&words, &T::MODULUS));
        assert_eq!(
            T::from_montgomery_words(&words).to_be_bytes(),
            x.to_be_bytes()
        );
        // a guest operand may be any representative below 2^256
        if let Some(unreduced) = plus_modulus(&words, &T::MODULUS) {
            assert_eq!(
                T::from_guest_operand_words(&unreduced).to_be_bytes(),
                x.to_be_bytes()
            );
        }
        let words = x.to_canonical_words();
        assert_eq!(be_bytes(&words), x.to_be_bytes());
        assert_eq!(
            T::from_canonical_words(&words).map(T::to_be_bytes),
            Some(x.to_be_bytes())
        );
    }

    #[test]
    fn secp256k1_montgomery_words_round_trip() {
        // the Montgomery form of 1 is `2^256` reduced; converting back checks `R_INVERSE`
        assert_eq!(FieldElement::ONE.to_montgomery_words(), R_MOD_P_WORDS);
        assert_eq!(Scalar::ONE.to_montgomery_words(), R_MOD_N_WORDS);
        check_round_trip(FieldElement::ONE);
        check_round_trip(Scalar::ONE);
        proptest!(|(bytes: [u8; 32])| {
            if let Some(x) = FieldElement::from_be_bytes(&bytes) {
                check_round_trip(x);
            }
            if let Some(x) = Scalar::from_be_bytes(&bytes) {
                check_round_trip(x);
            }
        });
    }

    /// The delegated check of a guest answer, which must leave the words as they were
    fn below<T: Secp256k1Element>(words: [u32; 8]) -> bool {
        let mut operand = DelegationOperand(words);
        // SAFETY: the words are a 32-byte aligned local
        let valid = unsafe { T::validate::<HintError>(&raw mut operand.0) }.is_ok();
        assert_eq!(operand.0, words);
        valid
    }

    #[test]
    fn secp256k1_answers_must_be_below_the_modulus() {
        fn check<T: Secp256k1Element>() {
            let mut below_modulus = T::MODULUS;
            below_modulus[0] -= 1;
            assert!(!below::<T>(T::MODULUS));
            assert!(!below::<T>([u32::MAX; 8]));
            assert!(below::<T>(below_modulus));
            assert!(below::<T>([0; 8]));
            assert!(T::from_canonical_words(&T::MODULUS).is_none());
            assert!(T::from_canonical_words(&below_modulus).is_some());
        }
        check::<FieldElement>();
        check::<Scalar>();
        proptest!(|(words: [u32; 8])| {
            prop_assert_eq!(below::<FieldElement>(words), is_below(&words, &FieldElement::MODULUS));
            prop_assert_eq!(below::<Scalar>(words), is_below(&words, &Scalar::MODULUS));
        });
    }

    #[test]
    fn secp256k1_answers_follow_the_target() {
        fn check<T: Secp256k1Element + HintAnswer>(x: T) {
            for (target, words) in [
                (HintTarget::Native, x.to_canonical_words()),
                (HintTarget::Guest, x.to_montgomery_words()),
            ] {
                let mut response = Vec::new();
                x.write(target, &mut response);
                assert_eq!(response, words);
            }
        }
        check(FieldElement::from_be_bytes(&[7; 32]).unwrap());
        check(Scalar::from_be_bytes(&[7; 32]).unwrap());
    }

    #[test]
    fn hooks_with_oracle_match_the_default_hooks() {
        proptest!(|(a: [u8; 32], b: [u8; 32])| {
            let (Some(x), Some(y)) = (FieldElement::from_be_bytes(&a), FieldElement::from_be_bytes(&b))
            else {
                return Ok(());
            };
            let mut default = DefaultSecp256k1Hooks;
            let mut oracle = RecordingHintOracle::new();
            let mut hinted = Secp256k1HooksWithOracle::new(&mut oracle);

            let (mut d, mut h) = (x, x);
            prop_assert_eq!(default.fe_sqrt_and_assign(&mut d), hinted.fe_sqrt_and_assign(&mut h));
            prop_assert_eq!(d.to_bytes(), h.to_bytes());

            let (mut d, mut h) = (x, x);
            default.fe_invert_and_assign(&mut d);
            hinted.fe_invert_and_assign(&mut h);
            prop_assert_eq!(d.to_bytes(), h.to_bytes());

            if !y.normalizes_to_zero() {
                let (mut d, mut h) = (MaybeUninit::uninit(), MaybeUninit::uninit());
                let d = *default.fe_divide(&mut [x, y], &mut d);
                let h = *hinted.fe_divide(&mut [x, y], &mut h);
                prop_assert_eq!(d.to_bytes(), h.to_bytes());
            }

            if let Some(s) = Scalar::from_be_bytes(&a) {
                let (mut d, mut h) = (s, s);
                default.scalar_invert_and_assign(&mut d);
                hinted.scalar_invert_and_assign(&mut h);
                prop_assert_eq!(d.to_repr(), h.to_repr());
            }
        });
    }

    /// Zero inputs are answered as by the default hooks, without a query
    #[test]
    fn zero_inputs_send_no_query() {
        let mut oracle = TapeOracle::new([]);
        let mut hooks = Secp256k1HooksWithOracle::new(&mut oracle);
        let mut x = FieldElement::ZERO;
        assert!(hooks.fe_sqrt_and_assign(&mut x));
        hooks.fe_invert_and_assign(&mut x);
        assert!(x.normalizes_to_zero());
        let mut s = Scalar::ZERO;
        hooks.scalar_invert_and_assign(&mut s);
        assert!(s.is_zero());
        assert!(oracle.queries.is_empty());
    }

    fn seven() -> FieldElement {
        FieldElement::from_be_bytes(&be_bytes_of(7)).unwrap()
    }

    /// The big-endian bytes of `n`
    fn be_bytes_of(n: u8) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[31] = n;
        bytes
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn wrong_fe_inverse_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Secp256k1BaseFieldInverse]);
        Secp256k1HooksWithOracle::new(&mut oracle).fe_invert_and_assign(&mut seven());
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn wrong_scalar_inverse_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Secp256k1ScalarFieldInverse]);
        let mut s = Scalar::from_be_bytes(&be_bytes_of(7)).unwrap();
        Secp256k1HooksWithOracle::new(&mut oracle).scalar_invert_and_assign(&mut s);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn wrong_fe_sqrt_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Secp256k1BaseFieldSqrt]);
        Secp256k1HooksWithOracle::new(&mut oracle).fe_sqrt_and_assign(&mut seven());
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn false_non_residue_flag_is_rejected() {
        let mut oracle = lying_oracle_last_word(&[FieldHintOp::Secp256k1BaseFieldSqrt]);
        Secp256k1HooksWithOracle::new(&mut oracle).fe_sqrt_and_assign(&mut seven());
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn wrong_fe_quotient_is_rejected() {
        let mut oracle = lying_oracle(&[FieldHintOp::Secp256k1BaseFieldDivision]);
        let mut fraction = [seven(), FieldElement::ONE];
        Secp256k1HooksWithOracle::new(&mut oracle)
            .fe_divide(&mut fraction, &mut MaybeUninit::uninit());
    }

    #[test]
    #[should_panic(expected = "the hint answer is well-formed")]
    fn non_canonical_answers_are_rejected() {
        let mut oracle = TapeOracle::new(<FieldElement as Secp256k1Element>::MODULUS);
        Secp256k1HooksWithOracle::new(&mut oracle).fe_invert_and_assign(&mut seven());
    }

    /// `(digest, r, s, y_is_odd)`
    type SignatureInput = ([u8; 32], [u8; 32], [u8; 32], bool);

    /// A signature of `digest` by `key`, as `(digest, r, s, y_is_odd)`, if its `x` is the
    /// unreduced `r`
    fn sign(key: &[u8; 32], digest: &[u8; 32]) -> Option<SignatureInput> {
        let key = crate::k256::ecdsa::SigningKey::from_bytes(key.into()).ok()?;
        let (signature, recovery_id) = key.sign_prehash_recoverable(digest).ok()?;
        if recovery_id.is_x_reduced() {
            return None;
        }
        let bytes = signature.to_bytes();
        let (r, s) = bytes.split_at(32);
        Some((
            *digest,
            r.try_into().unwrap(),
            s.try_into().unwrap(),
            recovery_id.is_y_odd(),
        ))
    }

    /// An oracle that records the op of every field hint query, read from its request
    struct OpRecording<O> {
        inner: O,
        ops: Vec<FieldHintOp>,
    }

    impl<O: MemoryOracle> MemoryOracle for OpRecording<O> {
        type Error = O::Error;

        unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), O::Error> {
            assert_eq!(query_id, crate::hints::FIELD_OPS_ADVISE_QUERY_ID);
            // SAFETY: the request is live during the query (the caller's contract)
            let memory = unsafe { crate::hints::host::NativeQuerierMemory::new() };
            let (op, _, _) = crate::hints::read_field_hint_request(&memory, input_word).unwrap();
            self.ops.push(FieldHintOp::parse_u32(op).unwrap());
            // SAFETY: the caller's contract
            unsafe { self.inner.send_query(query_id, input_word) }
        }

        fn read_word(&mut self) -> Result<u32, O::Error> {
            self.inner.read_word()
        }

        unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), O::Error> {
            // SAFETY: the caller's contract
            unsafe { self.inner.write_words(dst, num_words) }
        }

        fn finish_query(&mut self) -> Result<(), O::Error> {
            self.inner.finish_query()
        }
    }

    #[test]
    fn recovery_with_oracle_is_witnessed() {
        use crate::secp256k1::recover_from_bytes_with_hooks;
        use proptest::test_runner::{Config, TestRunner};

        let mut runner = TestRunner::new(Config::with_cases(32));
        runner
            .run(
                &(
                    proptest::prelude::any::<[u8; 32]>(),
                    proptest::prelude::any::<[u8; 32]>(),
                ),
                |(key, digest)| {
                    let Some((digest, r, s, odd)) = sign(&key, &digest) else {
                        return Ok(());
                    };
                    let expected = recover_from_bytes_with_hooks(
                        &digest,
                        &r,
                        &s,
                        odd,
                        &mut DefaultSecp256k1Hooks,
                    )
                    .unwrap();
                    let mut oracle = OpRecording {
                        inner: RecordingHintOracle::new(),
                        ops: Vec::new(),
                    };
                    let recovered = recover_from_bytes_with_hooks(
                        &digest,
                        &r,
                        &s,
                        odd,
                        &mut Secp256k1HooksWithOracle::new(&mut oracle),
                    )
                    .unwrap();
                    prop_assert_eq!(recovered, expected);
                    let count = |op| oracle.ops.iter().filter(|o| **o == op).count();
                    prop_assert_eq!(count(FieldHintOp::Secp256k1RecoveryWitness), 1);
                    prop_assert_eq!(count(FieldHintOp::Secp256k1BaseFieldSqrt), 1);
                    prop_assert_eq!(count(FieldHintOp::Secp256k1ScalarFieldInverse), 1);
                    prop_assert!(count(FieldHintOp::Secp256k1BaseFieldDivision) > 100);
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn recovery_without_a_witness_computes_the_key() {
        use crate::hints::host::answer_query;
        use crate::hints::test_support::InProcessMemoryOracle;
        use crate::secp256k1::recover_from_bytes_with_hooks;

        let (digest, r, s, odd) = sign(&[5; 32], &[9; 32]).unwrap();
        let expected =
            recover_from_bytes_with_hooks(&digest, &r, &s, odd, &mut DefaultSecp256k1Hooks)
                .unwrap();
        // honest, except that it has no witness
        let mut oracle = OpRecording {
            inner: InProcessMemoryOracle::new(
                |query_id: u32, input_word: usize, memory: &dyn QuerierMemory| {
                    let (op, _, _) =
                        crate::hints::read_field_hint_request(memory, input_word).unwrap();
                    if op == FieldHintOp::Secp256k1RecoveryWitness as u32 {
                        return std::vec![0];
                    }
                    let mut words = Vec::new();
                    // SAFETY: the handler runs during the query, while its request is live
                    let answered = unsafe {
                        answer_query(query_id, input_word, memory, Some(&mut words), None)
                    };
                    assert_eq!(answered, Ok(true));
                    words
                },
            ),
            ops: Vec::new(),
        };
        let recovered = recover_from_bytes_with_hooks(
            &digest,
            &r,
            &s,
            odd,
            &mut Secp256k1HooksWithOracle::new(&mut oracle),
        )
        .unwrap();
        assert_eq!(recovered, expected);
        assert_eq!(
            oracle
                .ops
                .iter()
                .filter(|op| **op == FieldHintOp::Secp256k1RecoveryWitness)
                .count(),
            1
        );
    }

    /// A wrong witness stops the recovery, as a wrong hint does: a valid signature is never
    /// turned into a failed recovery
    #[test]
    fn wrong_witnesses_are_rejected() {
        use crate::secp256k1::recover_from_bytes_with_hooks;

        fn recovery_panics_with<O: MemoryOracle>(mut oracle: O) -> std::string::String {
            let (digest, r, s, odd) = sign(&[5; 32], &[9; 32]).unwrap();
            let failed = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
                let _ = recover_from_bytes_with_hooks(
                    &digest,
                    &r,
                    &s,
                    odd,
                    &mut Secp256k1HooksWithOracle::new(&mut oracle),
                );
            }));
            let payload = failed.unwrap_err();
            payload
                .downcast_ref::<std::string::String>()
                .cloned()
                .unwrap_or_default()
        }

        let ops = [FieldHintOp::Secp256k1RecoveryWitness];
        // a coordinate of the key, then the top word of the last coefficient
        for message in [
            recovery_panics_with(lying_oracle(&ops)),
            recovery_panics_with(lying_oracle_last_word(&ops)),
        ] {
            assert!(
                message.contains("the witness of the recovery is wrong"),
                "{message}"
            );
        }
    }

    /// A signature whose key is the point at infinity has no witness: the recovery is computed,
    /// and fails
    #[test]
    fn a_key_at_infinity_has_no_witness() {
        use crate::k256::elliptic_curve::sec1::ToEncodedPoint;
        use crate::secp256k1::recover_from_bytes_with_hooks;

        // R = 7 G, and z / s = 7, so s R - z G is the point at infinity
        let point = (crate::k256::ProjectivePoint::GENERATOR * crate::k256::Scalar::from(7u64))
            .to_affine()
            .to_encoded_point(false);
        let r: [u8; 32] = (*point.x().unwrap()).into();
        let odd = point.y().unwrap()[31] & 1 == 1;
        let s = be_bytes_of(3);
        let digest = be_bytes_of(21);
        assert!(
            recover_from_bytes_with_hooks(&digest, &r, &s, odd, &mut DefaultSecp256k1Hooks)
                .is_err()
        );
        let mut oracle = OpRecording {
            inner: RecordingHintOracle::new(),
            ops: Vec::new(),
        };
        let recovered = recover_from_bytes_with_hooks(
            &digest,
            &r,
            &s,
            odd,
            &mut Secp256k1HooksWithOracle::new(&mut oracle),
        );
        assert!(recovered.is_err());
        assert!(oracle.ops.contains(&FieldHintOp::Secp256k1RecoveryWitness));
    }
}
