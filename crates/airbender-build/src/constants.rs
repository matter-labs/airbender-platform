//! Build-time defaults shared across the crate.

pub const DEFAULT_APP_NAME: &str = "app";

pub const DEFAULT_GUEST_TOOLCHAIN: &str = "nightly-2026-08-09";

// TODO: We would love to use `riscv32im-unknown-openvm-elf` target
// as it's meant to be generic, but currently the corresponding PR
// is not merged in Rust upstream. `risc0` toolchain is compatible,
// so we use it for now.
// For the upstream PR, see: https://github.com/rust-lang/rust/pull/149797
pub const DEFAULT_GUEST_TARGET: &str = "riscv32im-risc0-zkvm-elf";
