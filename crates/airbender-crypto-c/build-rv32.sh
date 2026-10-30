#!/bin/sh
# Builds the RV32 static library, with `c-runtime`. FEATURES defaults to `delegation`;
# 'csr-oracle delegation' adds hints, '' selects software arithmetic.
# RUSTFLAGS must reach every crate: lower-atomic only on this one leaves atomic libcalls.
set -eu
cd "$(dirname "$0")/../.."
export RUSTFLAGS="${RUSTFLAGS:-} -C panic=abort -C target-feature=+m,-unaligned-scalar-mem,+relax -C passes=lower-atomic"
cargo rustc -p airbender-crypto-c --release --target riscv32im-risc0-zkvm-elf --crate-type staticlib \
    -Z build-std=core,alloc,panic_abort,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --features "c-runtime ${FEATURES-delegation}" "$@"
