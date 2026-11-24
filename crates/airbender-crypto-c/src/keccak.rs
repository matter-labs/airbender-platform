//! C bindings of the streaming Keccak-256 and SHA3-256 hashers (`airbender_crypto`'s
//! `Keccak256Core`), over an opaque state in the layout of the keccak delegation.
use crate::{check_ptr, input, Bytes};
use airbender_crypto::sha3::delegated::Keccak256Core;
use airbender_crypto::MiniDigest;
use core::mem::{align_of, size_of};

/// The opaque state: holds a `Keccak256Core` of either padding
#[repr(C, align(256))]
#[derive(Clone, Copy)]
pub struct KeccakState {
    pub opaque: [u8; 512],
}

const _: () = {
    assert!(size_of::<Keccak256Core<false>>() <= size_of::<KeccakState>());
    assert!(align_of::<Keccak256Core<false>>() <= align_of::<KeccakState>());
    assert!(size_of::<Keccak256Core<true>>() == size_of::<Keccak256Core<false>>());
};

/// The hasher of a C state; the state must be valid for reads and writes
unsafe fn core<const SHA3: bool>(state: *mut KeccakState) -> *mut Keccak256Core<SHA3> {
    check_ptr(state);
    state.cast()
}

/// The four bindings of one padding
macro_rules! hasher_bindings {
    ($sha3:literal, $init:ident, $update:ident, $finalize:ident, $finalize_reset:ident) => {
        #[no_mangle]
        /// Initializes the state.
        /// # Safety
        /// See the crate-level contract.
        pub unsafe extern "C" fn $init(state: *mut KeccakState) {
            core::<$sha3>(state).write(Keccak256Core::new());
        }

        #[no_mangle]
        /// Absorbs `len` bytes of `data` into an initialized state.
        /// # Safety
        /// See the crate-level contract.
        pub unsafe extern "C" fn $update(state: *mut KeccakState, data: *const u8, len: usize) {
            (*core::<$sha3>(state)).update(input(data, len));
        }

        #[no_mangle]
        /// Writes the digest of an initialized state, which is unspecified afterwards.
        /// # Safety
        /// See the crate-level contract.
        pub unsafe extern "C" fn $finalize(state: *mut KeccakState, digest: *mut Bytes<32>) {
            check_ptr(digest);
            let output = (*core::<$sha3>(state)).finalize_in_place();
            digest.write(Bytes { data: output.0 });
        }

        #[no_mangle]
        /// Writes the digest of an initialized state, and initializes it again.
        /// # Safety
        /// See the crate-level contract.
        pub unsafe extern "C" fn $finalize_reset(state: *mut KeccakState, digest: *mut Bytes<32>) {
            check_ptr(digest);
            let output = (*core::<$sha3>(state)).finalize_reset();
            digest.write(Bytes { data: output.0 });
        }
    };
}

hasher_bindings!(
    false,
    airbender_keccak256_init,
    airbender_keccak256_update,
    airbender_keccak256_finalize,
    airbender_keccak256_finalize_reset
);
hasher_bindings!(
    true,
    airbender_sha3_256_init,
    airbender_sha3_256_update,
    airbender_sha3_256_finalize,
    airbender_sha3_256_finalize_reset
);
