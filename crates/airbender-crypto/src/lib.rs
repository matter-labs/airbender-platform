#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]
#![allow(clippy::uninit_assumed_init)]
#![allow(clippy::new_without_default)]
// `ECMultContext::new_in` and the allocating precompile endpoints need it, behind `alloc` (or
// `cfg(test)`). Declaring it unconditionally trips `unused_features` on builds without them.
#![cfg_attr(any(feature = "alloc", test), feature(allocator_api))]
// The crate is `no_std`, so `context.rs`'s `use alloc::boxed::Box` needs the
// crate linked explicitly. Without this, `--features alloc` does not build at all.
// Tests use `alloc` too (the oracle side of `hints`).
// The emulation of the bigint delegation on a host keeps its scratch space per thread
// (`std::thread_local!`): a host process computes on several threads, the guest on one.
#[cfg(any(feature = "alloc", test))]
extern crate alloc;
#[cfg(all(emulated_delegation, not(test)))]
extern crate std;

// Compile-time checks of the memory layout of the extension field towers and of the affine points,
// which are not `repr(C)`: oracles read these values where they are in the memory of the RISC-V
// guest. The components of an extension field element are back to back, `c0` first, without
// padding, recursively, and an affine point is `x`, then `y`, then the flag of the point at
// infinity.
macro_rules! assert_tower_layout {
    ($fq:ty, $fq2:ty, $fq6:ty, $fq12:ty) => {
        const _: () = {
            use core::mem::{offset_of, size_of};
            assert!(offset_of!($fq2, c0) == 0 && offset_of!($fq2, c1) == size_of::<$fq>());
            assert!(size_of::<$fq2>() == 2 * size_of::<$fq>());
            assert!(offset_of!($fq6, c0) == 0 && offset_of!($fq6, c1) == size_of::<$fq2>());
            assert!(offset_of!($fq6, c2) == 2 * size_of::<$fq2>());
            assert!(size_of::<$fq6>() == 3 * size_of::<$fq2>());
            assert!(offset_of!($fq12, c0) == 0 && offset_of!($fq12, c1) == size_of::<$fq6>());
            assert!(size_of::<$fq12>() == 2 * size_of::<$fq6>());
        };
    };
}

macro_rules! assert_affine_layout {
    ($point:ty, $coordinate:ty) => {
        const _: () = {
            use core::mem::{offset_of, size_of};
            assert!(offset_of!($point, x) == 0);
            assert!(offset_of!($point, y) == size_of::<$coordinate>());
            assert!(offset_of!($point, infinity) == 2 * size_of::<$coordinate>());
        };
    };
}
pub mod affine_glv;
// The bigint delegation, its emulation and what is built on them (the delegated fields here;
// elsewhere the secp256k1 and secp256r1 fields and scalars of the delegation, the delegated
// digits and the hinted path of ModExp, the delegated hashes) do not exist on `riscv64`, whose
// guest has no delegation and must not carry its emulation either: nothing there can select
// them (see `build.rs`), and a cfg that tried to would not compile.
#[allow(clippy::all)]
#[allow(unused_imports)]
#[cfg(all(
    not(target_arch = "riscv64"),
    any(bigint_ops, feature = "testing", test)
))]
pub mod ark_ff_delegation;
#[allow(clippy::all)]
#[cfg(not(target_arch = "riscv64"))]
#[allow(unused_imports, dead_code)]
mod bigint_delegation;
pub mod blake2f;
#[allow(unexpected_cfgs)]
pub mod blake2s;
// Used by the precompiles and ModExp on RV32 only
#[cfg(any(target_arch = "riscv32", test))]
mod byte_order;
pub mod bytes32;
#[cfg(test)]
mod delegation_counts;
pub(crate) mod extension_tower;
#[cfg(any(target_pointer_width = "64", test, proving))]
pub mod field_inverse;
pub mod hinted_ops;
pub mod hints;
pub(crate) mod jacobian;
#[cfg(any(fp_lazy_reduction, test))]
mod lazy_fp;
pub mod modexp;
pub mod oracle_common;
pub mod precompiles;
pub mod scratch;
pub use self::bytes32::Bytes32;
#[allow(clippy::all)]
pub mod bls12_381;
#[allow(clippy::all)]
pub mod bn254;
mod glv_decomposition;
pub mod k256;
pub mod p256;
pub mod ripemd160;
pub mod secp256k1;
pub mod secp256r1;
pub mod sha256;
pub mod sha3;

pub use k256 as rust_k256;

#[cfg(bigint_ops)]
pub use self::ark_ff_delegation::{BigInt, BigInteger};

#[cfg(not(bigint_ops))]
pub use self::ark_ff::{BigInt, BigInteger};

#[cfg(bigint_ops)]
pub use crate::ark_ff_delegation::Fp;

#[cfg(not(bigint_ops))]
pub use ark_ff::Fp;

#[cfg(not(target_arch = "riscv64"))]
mod raw_delegation_interface;

pub use blake2 as blake2_ext;

pub use ark_ec;
pub use ark_ff;
pub use ark_serialize;

#[cfg(not(target_arch = "riscv64"))]
pub use self::raw_delegation_interface::{
    bigint_op_delegation_raw, bigint_op_delegation_with_carry_bit_raw,
};

// TODO: Keep this compatibility shim while external call sites migrate away from
// explicit initialization requirements.
pub fn init_lib() {}

/// The operations of the bigint delegation
#[cfg(not(target_arch = "riscv64"))]
#[derive(Clone, Copy)]
pub enum BigIntOps {
    Add = 0,
    Sub = 1,
    SubAndNegate = 2,
    MulLow = 3,
    MulHigh = 4,
    Eq = 5,
    MemCpy = 7,
}

pub trait MiniDigest: Sized {
    type HashOutput;

    fn new() -> Self;
    fn digest(input: impl AsRef<[u8]>) -> Self::HashOutput;
    fn update(&mut self, input: impl AsRef<[u8]>);
    fn finalize(self) -> Self::HashOutput;
    fn finalize_reset(&mut self) -> Self::HashOutput;

    // Allow to reorder and avoid copying in finalize - output - reset operations sequence if hasher supports
    // such case due to it's internal structure
    #[inline(always)]
    fn finalize_reset_with_closure<FN: FnOnce(&Self::HashOutput)>(&mut self, closure: FN) {
        let t = self.finalize_reset();
        (closure)(&t);
    }
}

///
/// Parse the byte array as a BE 32-byte BigInt.
/// If length is less than 32 bytes, it will be left-padded (most significant bytes) with zeroes.
///
pub fn parse_u256_be<const N: usize>(input: &[u8; N]) -> BigInt<4> {
    assert!(N <= 32);
    // Arkworks has strange format for integer serialization, so we do manually
    let mut repr = [0u64; 4];
    let mut repr_index = 0usize;
    let mut offset = input.len();
    while offset >= 8 {
        offset -= 8;
        repr[repr_index] = u64::from_be_bytes(input[offset..offset + 8].try_into().unwrap());
        repr_index += 1;
    }
    if offset != 0 {
        let mut buff = [0u8; 8];
        buff[8 - offset..].copy_from_slice(&input[..offset]);
        repr[repr_index] = u64::from_be_bytes(buff);
    }
    BigInt::new(repr)
}

#[cfg(test)]
mod signature_tests;
