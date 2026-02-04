//! Build-time defaults shared across the crate.

pub const DEFAULT_APP_NAME: &str = "app";

pub const DEFAULT_GUEST_TARGET: &str = "riscv32im-risc0-zkvm-elf";

pub const DEFAULT_GUEST_BUILD_STD: &str = "alloc,core,panic_abort,compiler_builtins,std,proc_macro";

pub const DEFAULT_GUEST_BUILD_STD_FEATURES: &str = "compiler-builtins-mem";

pub const DEFAULT_GUEST_CC: &str = "clang";

pub const DEFAULT_GUEST_TOOLCHAIN: &str = "nightly-2026-01-19";

pub const DEFAULT_GUEST_RUSTFLAGS: &[&str] = &[
    "-C",
    "target-feature=+m,-unaligned-scalar-mem,+relax",
    "-C",
    "link-arg=-Tmemory.x",
    "-C",
    "link-arg=-Tlink.x",
    "-C",
    "link-arg=--save-temps",
    "-C",
    "force-frame-pointers",
    "-C",
    "passes=lower-atomic",
    "--cfg",
    r#"getrandom_backend="custom""#,
];
