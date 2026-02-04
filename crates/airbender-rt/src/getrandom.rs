//! Custom `getrandom` backend for the Airbender guest environment.

/// Stub implementation to avoid linker errors when dependencies pull in `getrandom`.
///
/// Airbender guests do not have access to system randomness, so calls are
/// rejected with `UNSUPPORTED`.
#[cfg(target_arch = "riscv32")]
#[no_mangle]
pub unsafe extern "Rust" fn __getrandom_v02_custom(
    _dest: *mut u8,
    _len: usize,
) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}

/// Stub implementation for `getrandom` v0.3 symbols.
#[cfg(target_arch = "riscv32")]
#[no_mangle]
pub unsafe extern "Rust" fn __getrandom_v03_custom(
    _dest: *mut u8,
    _len: usize,
) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}
