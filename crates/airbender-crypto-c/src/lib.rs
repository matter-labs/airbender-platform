//! C ABI for Airbender's crypto accelerators (see `include/airbender_crypto.h`).
//!
//! # Safety
//! Every export requires non-null pointers, aligned for their type and valid for the declared
//! sizes, even for empty inputs; the one exception is `airbender_crypto_record_end(NULL, 0)`.
//! Inputs may overlap outputs: an output is written only after every input was read.
//! Violations abort execution. Malformed cryptographic inputs return -1.
//!
//! # Hints
//! An export takes airbender-crypto's hinted path when there is an oracle (`with_active_oracle`)
//! and the plain path otherwise. On RV32 with `csr-oracle` the oracle is the non-determinism CSR;
//! natively it is the recorder of an active recording on the calling thread. A wrong or missing
//! hint aborts execution or is ignored; it never changes a result.
#![cfg_attr(target_arch = "riscv32", no_std)]
#![feature(allocator_api)]

// Native builds record the hints of the guest's delegated arithmetic without delegating.
#[cfg(all(feature = "delegation", not(target_arch = "riscv32")))]
compile_error!("`delegation` is RV32 only: native builds record hints without it");

// The hint answers are written in the layout of the delegated fields.
#[cfg(all(
    feature = "csr-oracle",
    target_arch = "riscv32",
    not(feature = "delegation")
))]
compile_error!("`csr-oracle` requires `delegation`");

extern crate alloc;
use airbender_crypto as crypto;
use core::{mem, ptr, slice};
use crypto::ark_ec::{
    short_weierstrass::{Affine, SWCurveConfig},
    AffineRepr,
};
use crypto::ark_ff::{AdditiveGroup, Field, PrimeField};
use crypto::hints::{HintError, MemoryOracle};

/// `$hinted` with the oracle of the call (`$oracle: &mut ActiveOracle`), else `$plain`. The hinted
/// operations exist natively and on RV32 with `delegation`. At most once per export.
macro_rules! hinted {
    (|$oracle:ident| $hinted:expr, $plain:expr $(,)?) => {
        $crate::with_active_oracle(|oracle| match oracle {
            #[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
            Some($oracle) => $hinted,
            #[cfg(all(target_arch = "riscv32", not(feature = "delegation")))]
            Some(_) => unreachable!("no oracle without delegation"),
            None => $plain,
        })
    };
}

mod curves;
mod fields;
#[cfg(not(target_arch = "riscv32"))]
mod recording;
mod signatures;

/// The oracle of the exports: the non-determinism CSR on RV32, the active recorder natively.
pub(crate) enum ActiveOracle<'a> {
    #[cfg(target_arch = "riscv32")]
    Csr(
        airbender_oracle::CsrOracle<HintError>,
        core::marker::PhantomData<&'a mut ()>,
    ),
    #[cfg(not(target_arch = "riscv32"))]
    Recording(&'a mut crypto::hints::RecordingHintOracle),
}

// Forwarding only: queries come from airbender-crypto's hinted operations.
impl MemoryOracle for ActiveOracle<'_> {
    type Error = HintError;

    #[inline(always)]
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        match self {
            // SAFETY: the caller's contract, unchanged
            #[cfg(target_arch = "riscv32")]
            Self::Csr(oracle, _) => unsafe { oracle.send_query(query_id, input_word) },
            // SAFETY: the caller's contract, unchanged
            #[cfg(not(target_arch = "riscv32"))]
            Self::Recording(oracle) => unsafe { oracle.send_query(query_id, input_word) },
        }
    }

    #[inline(always)]
    fn read_word(&mut self) -> Result<u32, HintError> {
        match self {
            #[cfg(target_arch = "riscv32")]
            Self::Csr(oracle, _) => oracle.read_word(),
            #[cfg(not(target_arch = "riscv32"))]
            Self::Recording(oracle) => oracle.read_word(),
        }
    }

    #[inline(always)]
    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
        match self {
            // SAFETY: the caller's contract, unchanged
            #[cfg(target_arch = "riscv32")]
            Self::Csr(oracle, _) => unsafe { oracle.write_words(dst, num_words) },
            // SAFETY: the caller's contract, unchanged
            #[cfg(not(target_arch = "riscv32"))]
            Self::Recording(oracle) => unsafe { oracle.write_words(dst, num_words) },
        }
    }

    #[inline(always)]
    fn finish_query(&mut self) -> Result<(), HintError> {
        match self {
            #[cfg(target_arch = "riscv32")]
            Self::Csr(oracle, _) => oracle.finish_query(),
            #[cfg(not(target_arch = "riscv32"))]
            Self::Recording(oracle) => oracle.finish_query(),
        }
    }
}

/// `f` with the oracle of the call: the CSR with `csr-oracle`, none without.
#[cfg(target_arch = "riscv32")]
#[inline(always)]
pub(crate) fn with_active_oracle<R>(f: impl FnOnce(Option<&mut ActiveOracle<'_>>) -> R) -> R {
    let mut oracle = ActiveOracle::Csr(
        airbender_oracle::CsrOracle::new(airbender_oracle::CsrChannel),
        core::marker::PhantomData,
    );
    f(cfg!(feature = "csr-oracle").then_some(&mut oracle))
}

/// `f` with the recorder of this thread's recording, if any. The recorder is borrowed for the whole
/// of `f`, so an export calls this once.
#[cfg(not(target_arch = "riscv32"))]
pub(crate) fn with_active_oracle<R>(f: impl FnOnce(Option<&mut ActiveOracle<'_>>) -> R) -> R {
    recording::with_active_recorder(|recorder| match recorder {
        Some(recorder) => f(Some(&mut ActiveOracle::Recording(recorder))),
        None => f(None),
    })
}

#[cfg(all(target_arch = "riscv32", feature = "c-runtime"))]
#[no_mangle]
/// Initializes airbender-rt's global allocator for a standalone C guest.
/// # Safety
/// Call once, before anything allocates. [start, end) must be writable, aligned for usize, owned
/// by the allocator only, and stay live.
pub unsafe extern "C" fn airbender_crypto_init_heap(start: *mut u8, end: *mut u8) {
    check_ptr(start.cast::<usize>());
    check_ptr(end.cast::<usize>());
    assert!(start.addr() < end.addr(), "empty or reversed heap range");
    static INITIALIZED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
    assert!(
        !INITIALIZED.swap(true, core::sync::atomic::Ordering::Relaxed),
        "heap already initialized"
    );
    airbender_rt::allocator::init(start.cast(), end.cast());
}

#[cfg(not(target_arch = "riscv32"))]
std::thread_local! {
    /// The words of an ended recording that did not fit the caller's buffer.
    static RECORDED: core::cell::RefCell<Option<alloc::vec::Vec<u32>>> =
        const { core::cell::RefCell::new(None) };
}

#[cfg(not(target_arch = "riscv32"))]
#[no_mangle]
/// Native builds only: starts recording the hints of this thread's calls. A recording in progress
/// starts over, and words of an ended one that were not copied are dropped.
pub extern "C" fn airbender_crypto_record_begin() {
    RECORDED.take();
    recording::begin_recording();
}

#[cfg(not(target_arch = "riscv32"))]
#[no_mangle]
/// Native builds only: ends this thread's recording and returns its number of words, 0 without
/// one. The words are copied when they fit in `capacity`; otherwise they are kept for the next
/// call. `(NULL, 0)` is a size query.
/// # Safety
/// `words` must be valid for writes of `capacity` words, or null with a zero `capacity`.
pub unsafe extern "C" fn airbender_crypto_record_end(words: *mut u32, capacity: usize) -> usize {
    if !(words.is_null() && capacity == 0) {
        check_ptr(words);
    }
    RECORDED.with_borrow_mut(|recorded| {
        let tape = recorded.get_or_insert_with(recording::end_recording);
        let len = tape.len();
        if len <= capacity {
            if len != 0 {
                ptr::copy_nonoverlapping(tape.as_ptr(), words, len);
            }
            *recorded = None;
        }
        len
    })
}

/// Layout of every upstream byte-array struct, including 8-byte alignment on RV32.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bytes<const N: usize> {
    pub data: [u8; N],
}

/// Layout of the upstream pair structs: two byte arrays, unpadded.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pair<const P: usize, const Q: usize> {
    pub first: Bytes<P>,
    pub second: Bytes<Q>,
}

pub const EOK: i32 = 0;
pub const EFAIL: i32 = -1;

fn check_ptr<T>(p: *const T) {
    assert!(!p.is_null(), "null C pointer");
    assert_eq!(p as usize % mem::align_of::<T>(), 0, "unaligned C pointer");
}

// The C encoding: canonical big-endian prime-field components, c0 first recursively for
// extension fields.
pub(crate) fn decode<F: Field>(bytes: &[u8]) -> Option<F> {
    let width = (F::BasePrimeField::MODULUS_BIT_SIZE as usize).div_ceil(8);
    let count = F::extension_degree() as usize;
    if bytes.len() != width * count {
        return None;
    }
    let mut elements = [F::BasePrimeField::ZERO; 12];
    for (element, input) in elements[..count].iter_mut().zip(bytes.chunks_exact(width)) {
        let mut repr = <F::BasePrimeField as PrimeField>::BigInt::default();
        for (limb, chunk) in repr
            .as_mut()
            .iter_mut()
            .zip(input.as_chunks::<8>().0.iter().rev())
        {
            *limb = u64::from_be_bytes(*chunk);
        }
        *element = F::BasePrimeField::from_bigint(repr)?;
    }
    F::from_base_prime_field_elems(elements[..count].iter().copied())
}

pub(crate) fn encode_into<F: Field>(value: F, bytes: &mut [u8]) {
    let width = (F::BasePrimeField::MODULUS_BIT_SIZE as usize).div_ceil(8);
    for (element, dst) in value
        .to_base_prime_field_elements()
        .zip(bytes.chunks_exact_mut(width))
    {
        for (limb, chunk) in element
            .into_bigint()
            .as_ref()
            .iter()
            .zip(dst.as_chunks_mut::<8>().0.iter_mut().rev())
        {
            chunk.copy_from_slice(&limb.to_be_bytes());
        }
    }
}

// Compact affine coordinates (not EIP-2537's 64-byte padding); infinity is all-zero x/y.
pub(crate) fn point<C: SWCurveConfig>(bytes: &[u8]) -> Option<Affine<C>> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Some(Affine::identity());
    }
    let (x, y) = bytes.split_at(bytes.len() / 2);
    let point = Affine::new_unchecked(decode(x)?, decode(y)?);
    point.is_on_curve().then_some(point)
}

pub(crate) fn point_bytes<C: SWCurveConfig, const N: usize>(point: Affine<C>) -> Bytes<N> {
    let mut output = Bytes { data: [0; N] };
    if let Some((x, y)) = point.xy() {
        encode_into(x, &mut output.data[..N / 2]);
        encode_into(y, &mut output.data[N / 2..]);
    }
    output
}

unsafe fn read<T: Copy>(p: *const T) -> T {
    check_ptr(p);
    p.read()
}

unsafe fn input<'a, T>(p: *const T, n: usize) -> &'a [T] {
    check_ptr(p);
    assert!(
        n <= isize::MAX as usize / mem::size_of::<T>(),
        "C length overflow"
    );
    slice::from_raw_parts(p, n)
}

/// Check output before any hint queries, but do not create a mutable reference.
unsafe fn commit<T>(output: *mut T, compute: impl FnOnce() -> Option<T>) -> i32 {
    check_ptr(output);
    match compute() {
        Some(result) => {
            ptr::write(output, result);
            EOK
        }
        None => EFAIL,
    }
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_keccak256(
    data: *const u8,
    len: usize,
    output: *mut Bytes<32>,
) -> i32 {
    commit(output, || {
        Some(Bytes {
            data: <crypto::sha3::Keccak256 as crypto::MiniDigest>::digest(input(data, len)).0,
        })
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_sha256(data: *const u8, len: usize, output: *mut Bytes<32>) -> i32 {
    use crypto::sha256::{Digest, Sha256};
    commit(output, || {
        Some(Bytes {
            data: Sha256::digest(input(data, len)).into(),
        })
    })
}

#[no_mangle]
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_ripemd160(
    data: *const u8,
    len: usize,
    output: *mut Bytes<32>,
) -> i32 {
    use crypto::ripemd160::{Digest, Ripemd160};
    commit(output, || {
        let mut result = Bytes { data: [0; 32] };
        result.data[12..].copy_from_slice(&Ripemd160::digest(input(data, len)));
        Some(result)
    })
}

/// The output of ModExp: `mod_len` bytes written in order
struct ModExpOutput<'a> {
    bytes: &'a mut [u8],
    written: usize,
}

impl crypto::modexp::TryExtend<u8> for ModExpOutput<'_> {
    type Error = ();

    fn try_extend<I: IntoIterator<Item = u8>>(&mut self, iter: I) -> Result<(), ()> {
        for byte in iter {
            *self.bytes.get_mut(self.written).ok_or(())? = byte;
            self.written += 1;
        }
        Ok(())
    }
}

#[no_mangle]
/// Arbitrary-length big-endian ModExp; the output is exactly `mod_len` bytes.
/// # Safety
/// See the crate-level contract. Output may overlap any input.
pub unsafe extern "C" fn zkvm_modexp(
    base: *const u8,
    base_len: usize,
    exp: *const u8,
    exp_len: usize,
    modulus: *const u8,
    mod_len: usize,
    output: *mut u8,
) -> i32 {
    check_ptr(output);
    assert!(mod_len <= isize::MAX as usize, "C length overflow");
    let base = input(base, base_len);
    let exp = input(exp, exp_len);
    let modulus = input(modulus, mod_len);
    let result = with_active_oracle(|oracle| {
        crypto::modexp::modexp(base, exp, modulus, alloc::alloc::Global, oracle)
    });
    // The result is owned, so the output may overlap the operands.
    let mut output = ModExpOutput {
        bytes: slice::from_raw_parts_mut(output, mod_len),
        written: 0,
    };
    crypto::modexp::write_modexp_output(result, mod_len, &mut output)
        .expect("the output holds mod_len bytes");
    debug_assert_eq!(output.written, mod_len);
    EOK
}

#[no_mangle]
/// Existing Airbender BLAKE2s hash: delegated on RV32 with `delegation`, software otherwise.
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn airbender_blake2s256(
    data: *const u8,
    len: usize,
    output: *mut Bytes<32>,
) -> i32 {
    commit(output, || {
        Some(Bytes {
            data: <crypto::blake2s::Blake2s256 as crypto::MiniDigest>::digest(input(data, len)),
        })
    })
}

#[no_mangle]
/// Software BLAKE2b compression (EIP-152). No hints or delegation acceleration.
/// # Safety
/// See the crate-level contract. `h` is both input and output.
pub unsafe extern "C" fn zkvm_blake2f(
    rounds: u32,
    h: *mut Bytes<64>,
    m: *const Bytes<128>,
    t: *const Bytes<16>,
    f: u8,
) -> i32 {
    commit(h, || {
        let mut state = read(h);
        let message = read(m);
        let offset = read(t);
        if f > 1 {
            return None;
        }
        crypto::blake2f::compress(rounds, &mut state.data, &message.data, &offset.data, f == 1);
        Some(state)
    })
}

pub use curves::*;
pub use fields::*;
pub use signatures::*;
