//! The secp256k1 hints from the host over a word channel: on the RISC-V guest the
//! non-determinism CSR, as ZKsync OS uses it.
//!
//! [`OracleHooks`] implements [`Secp256k1Hooks`] by hint and verify: a square root, an inverse, a
//! quotient or the witness of a recovery comes from the host and is checked on the guest (an
//! element with one multiplication, the witness by `recover_witnessed`). The channel follows the
//! memory-based oracle protocol of ZKsync OS (`zk_ee::oracle::memory_io`, and the field hints of
//! `basic_system::system_functions::field_ops`), so that a host which serves ZKsync OS serves these
//! hooks, and these hooks can stand in for the hooks of ZKsync OS:
//!
//! - a query is two words: the query id [`FIELD_OPS_ADVISE_QUERY_ID`], then the address of a
//!   request [`FieldOpsHint`] in the memory of the querier, which holds the op ([`FieldHintOp`])
//!   and the address and the length in words of the operand. The host reads the request and the
//!   operand from the memory of the querier while it receives the second word;
//! - the response is exactly the words the querier reads, without a length: each one a channel
//!   read, stored by the querier where the value goes.
//!
//! The elements cross the channel in the representation of the build ([`MONTGOMERY_WORDS`]): with
//! the delegated field of the RISC-V guest they are their Montgomery words in memory, so an
//! operand is sent where it is and an answer is written straight into its element, then checked to
//! be below the modulus (a quotient excepted: the check against the fraction holds for any
//! representative). Otherwise the canonical integers cross as little-endian words, converted at
//! both ends. The host side of the protocol is [`host`].
//!
//! A wrong hint fails an assertion: hints come from the prover's own host, never from external
//! input, so a bad one is a broken prover rather than a case to recover from.

use super::field::FieldElement;
use super::hooks::{RecoveryWitness, Secp256k1Hooks};
use super::points::Affine;
use super::scalars::Scalar;
use core::mem::MaybeUninit;

#[cfg(all(not(target_arch = "riscv32"), feature = "secp256k1-static-context"))]
pub mod host;

/// The query id of the field hints of ZKsync OS (`ADVICE_SUBSPACE_MASK | 0x11`)
pub const FIELD_OPS_ADVISE_QUERY_ID: u32 = 0x4005_0011;

/// The secp256k1 ops of the field hints of ZKsync OS (`FieldHintOp` there, whose other numbers
/// are the hints of the other curves), with the words of their answers
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldHintOp {
    /// Base field square root: an element in; a candidate root (8 words) and a "quadratic
    /// non-residue" flag (one word) out. The candidate is the root of the input, or of its
    /// negation when the input is not a square (`p = 3 mod 4`).
    Secp256k1BaseFieldSqrt = 0,
    /// Base field inverse: an element in, its inverse out
    Secp256k1BaseFieldInverse = 1,
    /// Scalar field inverse: a scalar in, its inverse out
    Secp256k1ScalarFieldInverse = 2,
    /// Base field division: the numerator and the denominator (two adjacent elements) in, the
    /// quotient out
    Secp256k1BaseFieldDivision = 11,
    /// The witness of a key recovery (`recover_witnessed`): a [`RecoveryRequest`] in; a flag out
    /// (0: no witness, nothing more), then the coordinates of the key as canonical little-endian
    /// words (8 each) and the four coefficients (`i128`, 4 little-endian words each)
    Secp256k1RecoveryWitness = 14,
}

impl FieldHintOp {
    pub const ALL: [Self; 5] = [
        Self::Secp256k1BaseFieldSqrt,
        Self::Secp256k1BaseFieldInverse,
        Self::Secp256k1ScalarFieldInverse,
        Self::Secp256k1BaseFieldDivision,
        Self::Secp256k1RecoveryWitness,
    ];

    pub fn parse_u32(value: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|op| *op as u32 == value)
    }
}

/// The request of a hint query, at the address the second word of the query gives: the op, and
/// the address and the length in `u32` words of the operand, which the host reads where it is.
/// `W` is the word of the querier: `u32` on the RISC-V guest, `usize` for a querier in the
/// process of the host (`GenericFieldOpsHint<W>` of ZKsync OS).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FieldOpsHint<W> {
    pub op: u32,
    pub src_ptr: W,
    pub src_len_u32_words: u32,
}

/// The operand of [`FieldHintOp::Secp256k1RecoveryWitness`]: the signature `(r, s)` of `message`
/// (the digest reduced modulo the order) and the point `(x, y)` of the signature with `x = r` and
/// the `y` of the recovery id, each in the representation of the querier (see
/// [`MONTGOMERY_WORDS`])
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RecoveryRequest {
    pub r: Scalar,
    pub s: Scalar,
    pub message: Scalar,
    pub x: FieldElement,
    pub y: FieldElement,
}

impl RecoveryRequest {
    /// The request of the recovery of the key of the signature `(r, s)` of `message` with the
    /// point `point` of the signature
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

/// The words of the operand of [`FieldHintOp::Secp256k1RecoveryWitness`]
pub const RECOVERY_REQUEST_WORDS: usize = 5 * 8;

/// Whether the elements of this build are their Montgomery words (`x 2^256 mod m`, 8 little-endian
/// `u32` words) in memory: the delegated field of the RISC-V guest. The host reads an operand as
/// those words, any 256-bit representative, and answers with words below the modulus, which the
/// querier stores straight into the element. Otherwise the canonical integers cross the channel.
pub const MONTGOMERY_WORDS: bool = cfg!(all(target_arch = "riscv32", feature = "bigint_ops"));

/// The base field modulus `p` and the order `n`, little-endian words
pub const P_WORDS: [u32; 8] = [
    0xfffffc2f, 0xfffffffe, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
];
pub const N_WORDS: [u32; 8] = [
    0xd0364141, 0xbfd25e8c, 0xaf48a03b, 0xbaaedce6, 0xfffffffe, 0xffffffff, 0xffffffff, 0xffffffff,
];
/// `2^256 mod p` and `2^256 mod n`, the Montgomery form of one, little-endian words
pub const R_MOD_P_WORDS: [u32; 8] = [0x3d1, 0x1, 0, 0, 0, 0, 0, 0];
pub const R_MOD_N_WORDS: [u32; 8] = [0x2fc9bebf, 0x402da173, 0x50b75fc4, 0x45512319, 0x1, 0, 0, 0];

#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
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
#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
const unsafe fn leading_words<T>(value: &T) -> [u32; 8] {
    // SAFETY: guaranteed by the caller
    unsafe { *core::ptr::from_ref(value).cast::<[u32; 8]>() }
}

// A change of the delegated representation must be noticed here: the elements are 8 words, aligned
// for words, and the words are the Montgomery form with `R = 2^256` (`ONE` is `R mod m`), which the
// host relies on.
#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
const _: () = {
    assert!(size_of::<FieldElement>() == 32 && align_of::<FieldElement>() >= 4);
    assert!(size_of::<Scalar>() == 32 && align_of::<Scalar>() >= 4);
    assert!(size_of::<RecoveryRequest>() == 4 * RECOVERY_REQUEST_WORDS);
    // SAFETY: the elements are 32 bytes, aligned for `u32`: the 4 limbs of the delegated
    // representation, without padding (reading padding would fail the compile-time evaluation,
    // not go unnoticed)
    assert!(words_eq(
        &unsafe { leading_words(&FieldElement::ONE) },
        &R_MOD_P_WORDS
    ));
    assert!(words_eq(
        &unsafe { leading_words(&FieldElement::ZERO) },
        &[0; 8]
    ));
    assert!(words_eq(
        &unsafe { leading_words(&Scalar::ONE) },
        &R_MOD_N_WORDS
    ));
    assert!(words_eq(&unsafe { leading_words(&Scalar::ZERO) }, &[0; 8]));
};

/// The big-endian bytes of little-endian words (word 0 is the least significant)
pub fn be_bytes_of_words(words: &[u32; 8]) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, word) in words.iter().enumerate() {
        bytes[28 - 4 * i..32 - 4 * i].copy_from_slice(&word.to_be_bytes());
    }
    bytes
}

/// The little-endian words of big-endian bytes
pub fn words_of_be_bytes(bytes: &[u8; 32]) -> [u32; 8] {
    core::array::from_fn(|i| {
        u32::from_be_bytes(bytes[28 - 4 * i..32 - 4 * i].try_into().expect("4 bytes"))
    })
}

/// Whether the little-endian `value` is below the little-endian `modulus`
pub fn is_below(value: &[u32; 8], modulus: &[u32; 8]) -> bool {
    for (value, modulus) in value.iter().zip(modulus).rev() {
        if value != modulus {
            return value < modulus;
        }
    }
    false
}

/// An element on the channel: a base field element or a scalar
pub trait ChannelElement: Copy {
    /// The modulus, little-endian words
    const MODULUS: [u32; 8];

    /// The little-endian words of the canonical integer of the element
    fn to_canonical_words(&self) -> [u32; 8];

    /// The element of the words of a canonical integer, `None` for the modulus or above
    fn from_canonical_words(words: &[u32; 8]) -> Option<Self>;
}

impl ChannelElement for FieldElement {
    const MODULUS: [u32; 8] = P_WORDS;

    fn to_canonical_words(&self) -> [u32; 8] {
        words_of_be_bytes(&self.to_bytes().into())
    }

    fn from_canonical_words(words: &[u32; 8]) -> Option<Self> {
        Self::from_bytes(&be_bytes_of_words(words))
    }
}

impl ChannelElement for Scalar {
    const MODULUS: [u32; 8] = N_WORDS;

    fn to_canonical_words(&self) -> [u32; 8] {
        words_of_be_bytes(&self.to_repr().into())
    }

    fn from_canonical_words(words: &[u32; 8]) -> Option<Self> {
        Self::from_be_bytes_checked(&be_bytes_of_words(words))
    }
}

/// A word channel to the host: on the RISC-V guest the non-determinism CSR ([`CsrChannel`]). The
/// words are `usize`, as the addresses of the queries are; the response words are `u32`.
pub trait OracleChannel {
    fn write_word(&mut self, word: usize);
    fn read_word(&mut self) -> usize;
}

/// The non-determinism CSR as the channel
#[cfg(target_arch = "riscv32")]
#[derive(Clone, Copy, Debug, Default)]
pub struct CsrChannel;

#[cfg(target_arch = "riscv32")]
impl OracleChannel for CsrChannel {
    #[inline(always)]
    fn write_word(&mut self, word: usize) {
        riscv_common::csr_write_word(word)
    }

    #[inline(always)]
    fn read_word(&mut self) -> usize {
        riscv_common::csr_read_word() as usize
    }
}

/// The hooks over the non-determinism CSR: `CsrHooks::new(CsrChannel)`
#[cfg(target_arch = "riscv32")]
pub type CsrHooks = OracleHooks<CsrChannel>;

/// Secp256k1 hooks that take the expensive operations as hints from the host over a channel and
/// check them (see the [module documentation](self))
pub struct OracleHooks<C: OracleChannel> {
    channel: C,
}

impl<C: OracleChannel> OracleHooks<C> {
    pub const fn new(channel: C) -> Self {
        Self { channel }
    }

    pub fn channel(&mut self) -> &mut C {
        &mut self.channel
    }

    /// Sends the query for the hint `op` on the operand at `operand`, `words` long, which the host
    /// reads where it is while it receives the query
    #[inline(always)]
    fn send(&mut self, op: FieldHintOp, operand: *const u8, words: usize) {
        let request = FieldOpsHint::<usize> {
            op: op as u32,
            src_ptr: operand.expose_provenance(),
            src_len_u32_words: words as u32,
        };
        // The address is exposed before the barrier below, so that for the compiler the request
        // has escaped by then: otherwise its stores are dead (the channel writes read no memory)
        // and get removed.
        let address = core::ptr::from_ref(&request).expose_provenance();
        self.channel.write_word(FIELD_OPS_ADVISE_QUERY_ID as usize);
        // The host reads the request and the operand as soon as it receives the address, while
        // for the compiler the channel write (a `nomem` CSR asm) reads no memory: without a
        // barrier, the stores that fill the request or the operand are dead and get removed.
        // `black_box(())` is an empty asm that may read any exposed memory, so these stores happen
        // before it, and the channel write, an asm with side effects too, stays after it. Being
        // zero-sized, the argument is not spilled: no instruction is emitted.
        core::hint::black_box(());
        self.channel.write_word(address);
    }

    /// A response word
    #[inline(always)]
    fn read_u32(&mut self) -> u32 {
        u32::try_from(self.channel.read_word()).expect("a response word is a u32")
    }

    /// The next `num_words` response words into `dst`
    ///
    /// # Safety
    ///
    /// `dst` must be aligned for `u32` and valid for writes of `num_words` `u32`s.
    #[inline(always)]
    unsafe fn read_words(&mut self, dst: *mut u32, num_words: usize) {
        for i in 0..num_words {
            // SAFETY: within the words of `dst`
            unsafe { dst.add(i).write(self.read_u32()) };
        }
    }

    /// Asks for the hint `op` on the element `x`, and receives the answer, an element, into `x`:
    /// with [`MONTGOMERY_WORDS`] straight, as the host read the operand before it answered
    #[inline(always)]
    fn query_element<T: ChannelElement>(&mut self, op: FieldHintOp, x: &mut T) {
        if MONTGOMERY_WORDS {
            let words = core::ptr::from_mut(x).cast::<[u32; 8]>();
            self.send(op, words.cast_const().cast::<u8>(), 8);
            // SAFETY: the element is 8 words aligned for them (asserted at compile time), and
            // any words are a value of it (plain integers)
            unsafe {
                self.read_words(words.cast::<u32>(), 8);
                assert!(is_below(&*words, &T::MODULUS), "the hint is not canonical");
            }
        } else {
            let operand = x.to_canonical_words();
            self.send(op, operand.as_ptr().cast(), 8);
            *x = self.receive_canonical();
        }
    }

    /// An element answered as canonical words
    #[inline(always)]
    fn receive_canonical<T: ChannelElement>(&mut self) -> T {
        let mut words = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { self.read_words(words.as_mut_ptr(), 8) };
        T::from_canonical_words(&words).expect("the hint is not canonical")
    }

    /// The coordinate of a key of a witness: canonical words, as big-endian bytes
    #[inline(always)]
    fn receive_coordinate(&mut self) -> [u8; 32] {
        let mut words = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { self.read_words(words.as_mut_ptr(), 8) };
        be_bytes_of_words(&words)
    }

    /// A coefficient of a witness: an `i128` as 4 little-endian words
    #[inline(always)]
    fn receive_coefficient(&mut self) -> i128 {
        let mut value = 0u128;
        for i in 0..4 {
            value |= u128::from(self.read_u32()) << (32 * i);
        }
        value as i128
    }
}

impl<C: OracleChannel> Secp256k1Hooks for OracleHooks<C> {
    /// An inversion is a hint checked with one multiplication
    const FE_INVERT_IS_CHEAP: bool = true;

    /// A division is a hint checked with one multiplication
    const FE_DIVIDE_IS_CHEAP: bool = true;

    fn recovery_witness(
        &mut self,
        point: &Affine,
        r: &Scalar,
        s: &Scalar,
        message: &Scalar,
    ) -> Option<RecoveryWitness> {
        let op = FieldHintOp::Secp256k1RecoveryWitness;
        if MONTGOMERY_WORDS {
            let request = RecoveryRequest::new(point, r, s, message);
            self.send(
                op,
                core::ptr::from_ref(&request).cast(),
                RECOVERY_REQUEST_WORDS,
            );
        } else {
            let request = [
                r.to_canonical_words(),
                s.to_canonical_words(),
                message.to_canonical_words(),
                point.x.to_canonical_words(),
                point.y.to_canonical_words(),
            ];
            self.send(op, request.as_ptr().cast(), RECOVERY_REQUEST_WORDS);
        }
        if self.read_u32() == 0 {
            return None;
        }
        let key_x = self.receive_coordinate();
        let key_y = self.receive_coordinate();
        let coefficients = core::array::from_fn(|_| self.receive_coefficient());
        // checked by the recovery (`recover_witnessed`), which fails for a wrong witness
        Some(RecoveryWitness {
            key_x,
            key_y,
            coefficients,
        })
    }

    fn fe_sqrt_and_assign(&mut self, x: &mut FieldElement) -> bool {
        // as the default hooks: the root of zero is zero
        if x.is_zero() {
            return true;
        }
        let operand = *x;
        self.query_element(FieldHintOp::Secp256k1BaseFieldSqrt, x);
        let is_quadratic_non_residue = self.read_u32() != 0;

        // the candidate is `x^((p + 1) / 4)`: for a square its root, otherwise a root of `-x`
        // (`p = 3 mod 4`, so `-1` is not a square, and this proves that `x` is none)
        let mut squared = *x;
        squared.square_in_place();
        if is_quadratic_non_residue {
            squared.add_in_place(&operand);
        } else {
            squared.sub_in_place(&operand);
        }
        assert!(squared.is_zero(), "the square root hint is wrong");
        !is_quadratic_non_residue
    }

    fn fe_invert_and_assign(&mut self, x: &mut FieldElement) {
        // as the default hooks: the inverse of zero is zero
        if x.is_zero() {
            return;
        }
        let mut product = *x;
        self.query_element(FieldHintOp::Secp256k1BaseFieldInverse, x);
        product.mul_in_place(x);
        assert!(product.is_one(), "the inverse hint is wrong");
    }

    fn scalar_invert_and_assign(&mut self, x: &mut Scalar) {
        // as the default hooks: the inverse of zero is zero
        if x.is_zero() {
            return;
        }
        let mut product = *x;
        self.query_element(FieldHintOp::Secp256k1ScalarFieldInverse, x);
        product *= &*x;
        assert!(product.is_one(), "the scalar inverse hint is wrong");
    }

    fn fe_divide<'a>(
        &mut self,
        fraction: &mut [FieldElement; 2],
        quotient: &'a mut MaybeUninit<FieldElement>,
    ) -> &'a mut FieldElement {
        let op = FieldHintOp::Secp256k1BaseFieldDivision;
        let quotient = if MONTGOMERY_WORDS {
            self.send(
                op,
                core::ptr::from_mut(fraction).cast_const().cast::<u8>(),
                16,
            );
            // SAFETY: the element is 8 words aligned for them (asserted at compile time), and any
            // words are a value of it: the delegated arithmetic takes any representative, and the
            // check below is modulo `p`, so the quotient is not checked to be canonical
            unsafe {
                self.read_words(quotient.as_mut_ptr().cast::<u32>(), 8);
                quotient.assume_init_mut()
            }
        } else {
            let operand = [
                fraction[0].to_canonical_words(),
                fraction[1].to_canonical_words(),
            ];
            self.send(op, operand.as_ptr().cast(), 16);
            quotient.write(self.receive_canonical())
        };

        // quotient * denominator == numerator: the denominator is not zero (the caller's
        // obligation), so the quotient is the only element that passes. The fraction is ours to
        // overwrite.
        let [numerator, denominator] = fraction;
        denominator.mul_in_place(quotient);
        denominator.sub_in_place(numerator);
        assert!(denominator.is_zero(), "the quotient hint is wrong");
        quotient
    }
}

#[cfg(all(test, feature = "secp256k1-static-context"))]
mod tests {
    use super::host::{answer, read_request, QuerierMemory, Representation};
    use super::*;
    use crate::secp256k1::hooks::DefaultSecp256k1Hooks;
    use crate::secp256k1::recover_from_bytes_with_hooks;
    use proptest::{prop_assert, prop_assert_eq, proptest};
    use std::collections::VecDeque;

    type SignatureInput = ([u8; 32], [u8; 32], [u8; 32], bool);

    /// The memory of a querier in this process: the addresses are exposed pointers
    struct NativeMemory;

    impl QuerierMemory for NativeMemory {
        fn word_size(&self) -> usize {
            size_of::<usize>()
        }

        fn read_u32(&self, address: usize) -> u32 {
            assert!(address != 0 && address.is_multiple_of(4), "a word address");
            // SAFETY: the querier exposes its request and its operand while it sends the query
            unsafe { core::ptr::with_exposed_provenance::<u32>(address).read() }
        }
    }

    /// A channel answered in this process, in the canonical representation of a native build
    #[derive(Default)]
    struct NativeChannel {
        query_id: Option<u32>,
        response: VecDeque<u32>,
        served: Vec<FieldHintOp>,
    }

    impl OracleChannel for NativeChannel {
        fn write_word(&mut self, word: usize) {
            match self.query_id.take() {
                None => {
                    assert!(
                        self.response.is_empty(),
                        "the previous answer is read in full"
                    );
                    self.query_id = Some(u32::try_from(word).expect("a query id"));
                }
                Some(id) => {
                    assert_eq!(id, FIELD_OPS_ADVISE_QUERY_ID);
                    const { assert!(!MONTGOMERY_WORDS) };
                    let (op, operand) = read_request(&NativeMemory, word);
                    self.served.push(op);
                    self.response
                        .extend(answer(op, &operand, Representation::Canonical));
                }
            }
        }

        fn read_word(&mut self) -> usize {
            self.response
                .pop_front()
                .expect("the querier reads only what the host answered") as usize
        }
    }

    /// A signature of `digest` by `key`, as `(digest, r, s, y_is_odd)`
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

    #[test]
    fn the_protocol_is_the_one_of_zksync_os() {
        assert_eq!(FIELD_OPS_ADVISE_QUERY_ID, 0x4005_0000 | 0x11);
        assert_eq!(FieldHintOp::Secp256k1BaseFieldSqrt as u32, 0);
        assert_eq!(FieldHintOp::Secp256k1BaseFieldInverse as u32, 1);
        assert_eq!(FieldHintOp::Secp256k1ScalarFieldInverse as u32, 2);
        assert_eq!(FieldHintOp::Secp256k1BaseFieldDivision as u32, 11);
        assert_eq!(FieldHintOp::Secp256k1RecoveryWitness as u32, 14);
        for op in FieldHintOp::ALL {
            assert_eq!(FieldHintOp::parse_u32(op as u32), Some(op));
        }
        assert_eq!(FieldHintOp::parse_u32(3), None);
        // the request as the guest sends it
        assert_eq!(size_of::<FieldOpsHint<u32>>(), 12);
        assert_eq!(core::mem::offset_of!(FieldOpsHint<u32>, op), 0);
        assert_eq!(core::mem::offset_of!(FieldOpsHint<u32>, src_ptr), 4);
        assert_eq!(
            core::mem::offset_of!(FieldOpsHint<u32>, src_len_u32_words),
            8
        );
    }

    #[test]
    fn words_and_bytes_round_trip() {
        proptest!(|(bytes: [u8; 32])| {
            prop_assert_eq!(be_bytes_of_words(&words_of_be_bytes(&bytes)), bytes);
        });
        assert_eq!(be_bytes_of_words(&P_WORDS)[31], 0x2f);
        assert_eq!(be_bytes_of_words(&N_WORDS)[31], 0x41);
        assert!(is_below(&[0; 8], &P_WORDS));
        assert!(!is_below(&P_WORDS, &P_WORDS));
        assert!(is_below(&N_WORDS, &P_WORDS));
        assert!(!is_below(&P_WORDS, &N_WORDS));
        // the canonical words of one
        assert_eq!(
            FieldElement::ONE.to_canonical_words(),
            [1, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(Scalar::ONE.to_canonical_words(), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert!(FieldElement::from_canonical_words(&P_WORDS).is_none());
        assert!(Scalar::from_canonical_words(&N_WORDS).is_none());
        let r = FieldElement::from_canonical_words(&R_MOD_P_WORDS).unwrap();
        assert_eq!(r.to_canonical_words(), R_MOD_P_WORDS);
    }

    #[test]
    fn the_hooks_recover_as_the_default_hooks_with_the_witness() {
        proptest!(|(key: [u8; 32], digest: [u8; 32])| {
            if let Some((digest, r, s, odd)) = sign(&key, &digest) {
                let expected =
                    recover_from_bytes_with_hooks(&digest, &r, &s, odd, &mut DefaultSecp256k1Hooks)
                        .unwrap();
                let mut hooks = OracleHooks::new(NativeChannel::default());
                let recovered =
                    recover_from_bytes_with_hooks(&digest, &r, &s, odd, &mut hooks).unwrap();
                prop_assert_eq!(recovered, expected);
                let channel = hooks.channel();
                prop_assert!(channel.response.is_empty());
                let served = |op: FieldHintOp| channel.served.iter().filter(|o| **o == op).count();
                prop_assert_eq!(served(FieldHintOp::Secp256k1BaseFieldSqrt), 1);
                prop_assert_eq!(served(FieldHintOp::Secp256k1ScalarFieldInverse), 1);
                prop_assert_eq!(served(FieldHintOp::Secp256k1RecoveryWitness), 1);
                prop_assert!(served(FieldHintOp::Secp256k1BaseFieldDivision) > 100);
            }
        })
    }

    #[test]
    fn the_hooks_answer_the_operations_as_the_default_hooks() {
        proptest!(|(bytes: [u8; 32], other: [u8; 32])| {
            let mut hooks = OracleHooks::new(NativeChannel::default());
            if let Some(x) = FieldElement::from_bytes(&bytes) {
                let (mut expected, mut hinted) = (x, x);
                let exists = DefaultSecp256k1Hooks.fe_sqrt_and_assign(&mut expected);
                prop_assert_eq!(hooks.fe_sqrt_and_assign(&mut hinted), exists);
                prop_assert_eq!(hinted.to_bytes(), expected.to_bytes());

                let (mut expected, mut hinted) = (x, x);
                DefaultSecp256k1Hooks.fe_invert_and_assign(&mut expected);
                hooks.fe_invert_and_assign(&mut hinted);
                prop_assert_eq!(hinted.to_bytes(), expected.to_bytes());

                if let Some(y) = FieldElement::from_bytes(&other) {
                    if !y.is_zero() {
                        let mut fraction = [x, y];
                        let mut quotient = MaybeUninit::uninit();
                        let quotient = *hooks.fe_divide(&mut fraction, &mut quotient);
                        let mut expected = y;
                        expected.invert_in_place();
                        expected.mul_in_place(&x);
                        prop_assert_eq!(quotient.to_bytes(), expected.to_bytes());
                    }
                }
            }
            if let Some(k) = Scalar::from_be_bytes_checked(&bytes) {
                let (mut expected, mut hinted) = (k, k);
                DefaultSecp256k1Hooks.scalar_invert_and_assign(&mut expected);
                hooks.scalar_invert_and_assign(&mut hinted);
                prop_assert_eq!(hinted.to_repr(), expected.to_repr());
            }
            prop_assert!(hooks.channel().response.is_empty());
        })
    }

    /// A channel whose answers are off by one word
    struct Corrupting(NativeChannel);

    impl OracleChannel for Corrupting {
        fn write_word(&mut self, word: usize) {
            self.0.write_word(word)
        }

        fn read_word(&mut self) -> usize {
            self.0.read_word() ^ 1
        }
    }

    #[test]
    fn wrong_hints_fail() {
        let x = FieldElement::from_bytes(&[7; 32]).unwrap();
        let failed = std::panic::catch_unwind(|| {
            let mut hooks = OracleHooks::new(Corrupting(NativeChannel::default()));
            let mut y = x;
            hooks.fe_invert_and_assign(&mut y);
        });
        assert!(failed.is_err());
        let failed = std::panic::catch_unwind(|| {
            let mut hooks = OracleHooks::new(Corrupting(NativeChannel::default()));
            let mut y = x;
            hooks.fe_sqrt_and_assign(&mut y);
        });
        assert!(failed.is_err());
    }
}
