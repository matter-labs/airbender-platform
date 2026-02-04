//! Boot helpers for Airbender guest programs.

/// Initialize the Airbender runtime and then execute the entrypoint.
pub fn start<F>(entry: F) -> !
where
    F: FnOnce() -> core::convert::Infallible,
{
    #[cfg(target_arch = "riscv32")]
    {
        riscv_common::boot_sequence::init();
        unsafe {
            crate::allocator::init(
                riscv_common::boot_sequence::heap_start(),
                riscv_common::boot_sequence::heap_end(),
            );
        }
    }

    match entry() {}
}
