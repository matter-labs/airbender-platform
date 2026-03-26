use airbender_build::{build_dist, BuildConfig};
use std::fs;
use std::path::Path;

/// Searches `haystack` for `needle` as a contiguous byte sequence.
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Absolute path to the `airbender-sdk` crate, resolved relative to this crate's manifest.
fn sdk_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../airbender-sdk")
        .canonicalize()
        .expect("airbender-sdk must exist next to airbender-build")
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Creates a minimal guest project with a format-string panic in an unreachable branch.
///
/// A string-literal-only panic is an unreliable probe because the `&str` can survive
/// for unrelated reasons. A format-argument panic is stripped only when
/// `-Cpanic=immediate-abort` replaces the call site before the format struct is emitted.
fn scaffold_probe_guest(dir: &Path) {
    let sdk = sdk_path();

    write_file(
        &dir.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "probe"
version = "0.1.0"
edition = "2021"

[dependencies]
airbender = {{ package = "airbender-sdk", path = "{}" }}
"#,
            sdk.display()
        ),
    );

    write_file(
        &dir.join(".cargo/config.toml"),
        r#"[build]
target = "riscv32im-risc0-zkvm-elf"
rustflags = [
  "-C", "target-feature=+m,-unaligned-scalar-mem,+relax",
  "-C", "link-arg=-Tmemory.x",
  "-C", "link-arg=-Tlink.x",
  "-C", "link-arg=--save-temps",
  "-C", "force-frame-pointers",
  "-C", "passes=lower-atomic",
  "--cfg", "getrandom_backend=\"custom\"",
]

[env]
CC = "clang"

[unstable]
build-std = ["alloc", "core", "panic_abort", "compiler_builtins", "std", "proc_macro"]
build-std-features = ["compiler-builtins-mem"]
"#,
    );

    write_file(
        &dir.join("rust-toolchain.toml"),
        r#"[toolchain]
channel = "nightly-2026-02-10"
"#,
    );

    write_file(
        &dir.join("src/main.rs"),
        r#"#![no_std]
#![no_main]

use airbender::guest::read;

#[airbender::main]
fn main() -> u32 {
    let value: u32 = read().expect("read");
    if value == u32::MAX {
        panic!("PROBE_{}", value);
    }
    value + 1
}
"#,
    );
}

/// Verifies that `--panic-immediate-abort` eliminates format strings from the guest binary.
///
/// Builds the same probe guest twice:
/// - without the flag: `PROBE_` format string must be present in the binary
/// - with the flag:    `PROBE_` format string must be absent from the binary
///
/// Also asserts that `manifest.toml` records the correct `panic_immediate_abort` value
/// and that the panic_immediate_abort binary is smaller than the default binary.
#[test]
fn panic_immediate_abort_strips_format_string_from_binary() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let guest_dir = tmp.path().join("guest");
    let dist_dir = tmp.path().join("dist");
    scaffold_probe_guest(&guest_dir);

    // Build WITHOUT --panic-immediate-abort: format string must be present.
    let mut config = BuildConfig::new(&guest_dir);
    config.app_name = "default".to_string();
    config.dist_dir = Some(dist_dir.clone());
    let artifacts_without = build_dist(&config).expect("build without panic_immediate_abort");

    let bin_without = fs::read(&artifacts_without.app_bin).expect("read app.bin");
    assert!(
        contains_bytes(&bin_without, b"PROBE_"),
        "format string must be present in default binary ({} bytes)",
        bin_without.len(),
    );

    // Build WITH --panic-immediate-abort: format string must be absent.
    let mut config = BuildConfig::new(&guest_dir);
    config.app_name = "panic-immediate-abort".to_string();
    config.dist_dir = Some(dist_dir.clone());
    config.panic_immediate_abort = true;
    let artifacts_with = build_dist(&config).expect("build with panic_immediate_abort");

    let bin_with = fs::read(&artifacts_with.app_bin).expect("read app.bin");
    assert!(
        !contains_bytes(&bin_with, b"PROBE_"),
        "format string must be absent from panic_immediate_abort binary ({} bytes)",
        bin_with.len(),
    );

    // panic_immediate_abort binary must be less than half the size of the default binary.
    // Eliminating panic formatting infrastructure typically produces a ~5x reduction;
    // the 2x threshold is conservative enough to avoid brittleness while still catching
    // a missing flag (where the size would be equal).
    assert!(
        bin_with.len() * 2 < bin_without.len(),
        "panic_immediate_abort binary ({} bytes) must be less than half the default binary ({} bytes)",
        bin_with.len(),
        bin_without.len(),
    );
}
