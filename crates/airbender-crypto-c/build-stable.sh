#!/bin/sh
# Builds the C library with a stable toolchain (TOOLCHAIN, default 1.99.1) instead of the pinned
# nightly: RUSTC_BOOTSTRAP=1 lets a stable compiler accept the crates' `#![feature]` attributes.
# The toolchain must be installed; the RV32 target (prebuilt core) is added if missing.
#
#   sh build-stable.sh            # native and RV32
#   sh build-stable.sh native     # the native archive only
#   sh build-stable.sh rv32       # the RV32 archive only; FEATURES as for build-rv32.sh
set -eu
cd "$(dirname "$0")"
TOOLCHAIN="${TOOLCHAIN:-1.99.1}"
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
export RUSTC_BOOTSTRAP=1
rustc -V
case "${1-all}" in
    native|rv32|all) ;;
    *) echo "usage: $0 [native|rv32|all]" >&2; exit 2 ;;
esac
if test "${1-all}" != rv32; then
    (cd ../.. && cargo rustc -p airbender-crypto-c --release --crate-type staticlib)
fi
if test "${1-all}" != native; then
    if ! rustup target list --toolchain "$TOOLCHAIN" --installed | grep -q '^riscv32i-unknown-none-elf$'; then
        rustup target add riscv32i-unknown-none-elf --toolchain "$TOOLCHAIN"
    fi
    sh build-rv32.sh
fi
