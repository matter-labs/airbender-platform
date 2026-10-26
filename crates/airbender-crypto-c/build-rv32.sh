#!/bin/sh
# Run from any directory. FEATURES selects delegated arithmetic (default) and/or
# `csr-oracle` (which requires `delegation`): checked hints read from the input,
# from a tape that a native build without features records (see the README).
# FEATURES='' selects software arithmetic without hints. The standalone C runtime
# (`c-runtime`) uses airbender-rt and is always enabled here. The C guest
# initializes its heap with airbender_crypto_init_heap before allocating.
set -eu
cd "$(dirname "$0")/../.."
export RUSTFLAGS="${RUSTFLAGS:-} -C panic=abort -C target-feature=+m,-unaligned-scalar-mem,+relax -C passes=lower-atomic"
cargo rustc -p airbender-crypto-c --release --target riscv32im-risc0-zkvm-elf --crate-type staticlib \
    -Z build-std=core,alloc,panic_abort,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --features "c-runtime ${FEATURES-delegation}" "$@"
