#!/bin/sh
# Builds the RV32 static library, with `c-runtime`. FEATURES defaults to `delegation`;
# 'csr-oracle delegation' adds hints, 'csr-oracle' hints on software arithmetic, '' selects
# software arithmetic.
# The target is the standard riscv32i-unknown-none-elf, whose prebuilt `core` needs no
# build-std; the ISA is RV32I with Zmmul (multiplication only, no M): divisions and remainders
# become compiler-builtins calls, which the archive carries. The prebuilt `core` itself is
# compiled for plain RV32I, so its own multiplications are calls too.
# RUSTFLAGS must reach every crate: lower-atomic only on this one leaves atomic libcalls.
# Fat LTO over one codegen unit (override with CARGO_PROFILE_RELEASE_LTO and
# CARGO_PROFILE_RELEASE_CODEGEN_UNITS) halves the archive, which still carries every Rust crate
# whole, and takes a quarter off the linked guest.
# The archive is copied from the target directory to this crate's root, as
# crates/airbender-crypto-c/libairbender_crypto_c.a.
set -eu
crate=$(cd "$(dirname "$0")" && pwd)
cd "$crate/../.."
export RUSTFLAGS="${RUSTFLAGS:-} -C panic=abort -C target-feature=+zmmul,-unaligned-scalar-mem,+relax -C passes=lower-atomic"
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-fat}"
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-1}"
cargo rustc -p airbender-crypto-c --release --target riscv32i-unknown-none-elf --crate-type staticlib \
    --features "c-runtime ${FEATURES-delegation}" "$@"
cp "${CARGO_TARGET_DIR:-target}/riscv32i-unknown-none-elf/release/libairbender_crypto_c.a" "$crate/libairbender_crypto_c.a"
echo "copied to $crate/libairbender_crypto_c.a"
