//! Boot helpers for Airbender guest programs.

/// Initialize the Airbender runtime and then execute the entrypoint.
///
/// This helper is available only for built-in allocator backends. When
/// `allocator-custom` is enabled, use `start_with_allocator_init` so the
/// runtime receives an explicit allocator init hook.
#[cfg(not(feature = "allocator-custom"))]
pub fn start<F>(entry: F) -> !
where
    F: FnOnce() -> core::convert::Infallible,
{
    start_with_allocator_init(crate::allocator::init, entry)
}

/// Initialize the Airbender runtime with a custom allocator init hook.
pub fn start_with_allocator_init<F>(
    allocator_init: unsafe fn(*mut usize, *mut usize),
    entry: F,
) -> !
where
    F: FnOnce() -> core::convert::Infallible,
{
    #[cfg(not(target_arch = "riscv32"))]
    let _ = allocator_init;

    #[cfg(target_arch = "riscv32")]
    {
        riscv_common::boot_sequence::init();
        // SAFETY: The boot sequence guarantees that heap_start and heap_end
        // point to a valid, exclusively-owned memory region.
        unsafe {
            allocator_init(
                riscv_common::boot_sequence::heap_start(),
                riscv_common::boot_sequence::heap_end(),
            );
        }
    }

    match entry() {}
}
