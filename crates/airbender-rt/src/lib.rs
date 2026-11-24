#![doc = include_str!("../README.md")]
#![no_std]
#![cfg_attr(
    all(target_arch = "riscv32", not(feature = "std")),
    feature(alloc_error_handler)
)]
#![cfg_attr(
    all(target_arch = "riscv32", feature = "std"),
    feature(allocator_api, str_from_raw_parts)
)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(test, not(feature = "std")))]
extern crate std;

pub mod allocator;
#[cfg(feature = "boot")]
pub mod boot;
pub mod getrandom;
pub mod sys;
pub mod uart;

#[cfg(all(feature = "std", target_arch = "riscv32"))]
mod glue;

#[cfg(all(feature = "boot", not(feature = "allocator-custom")))]
pub use boot::start;
#[cfg(feature = "boot")]
pub use boot::start_with_allocator_init;

#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write as _;
    let mut uart = uart::QuasiUart::new();
    let _ = uart.write_str("PANIC: ");
    let _ = write!(uart, "{info}");
    sys::exit_error();
}

#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    use core::fmt::Write as _;
    let mut uart = uart::QuasiUart::new();
    let _ = write!(
        uart,
        "ALLOC_ERROR: size={} align={}",
        layout.size(),
        layout.align()
    );
    sys::exit_error();
}
