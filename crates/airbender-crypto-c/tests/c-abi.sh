#!/bin/sh
set -eu
cd "$(dirname "$0")/../../.."
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
cargo rustc -p airbender-crypto-c --release --crate-type staticlib -- --print native-static-libs 2>"$scratch/link.log" || { cat "$scratch/link.log"; exit 1; }
native_libs=$(sed -n 's/.*native-static-libs: //p' "$scratch/link.log" | tail -n 1)
test -n "$native_libs" || { cat "$scratch/link.log"; echo 'No native link flags returned'; exit 1; }
archive="${CARGO_TARGET_DIR:-target}/release/libairbender_crypto_c.a"
for language in c c++; do
    if test "$language" = c; then compiler=${CC:-cc}; standard=c11; else compiler=${CXX:-c++}; standard=c++11; fi
    # $native_libs is split on purpose
    $compiler -x "$language" -std="$standard" -Wall -Wextra -Werror -Wno-missing-field-initializers -Wno-missing-braces \
        -Icrates/airbender-crypto-c/include -c crates/airbender-crypto-c/tests/abi.c -o "$scratch/abi.o"
    $compiler "$scratch/abi.o" "$archive" $native_libs -o "$scratch/abi"
    "$scratch/abi"
    if (ulimit -c 0; "$scratch/abi" --null) >"$scratch/panic.log" 2>&1; then
        echo 'Null pointer contract did not abort'; exit 1
    fi
    echo "$standard ABI and null-pointer abort passed"
done
${RV32_CC:-clang} --target=riscv32-none-elf -ffreestanding -std=c11 -DLAYOUT_ONLY \
    -Icrates/airbender-crypto-c/include -fsyntax-only crates/airbender-crypto-c/tests/abi.c
echo 'RV32 C struct layouts passed'
