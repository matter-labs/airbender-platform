# Airbender C crypto accelerators

A C ABI over `airbender-crypto`. It implements all 19 functions of the eth-act
[`zkvm_accelerators.h`](https://github.com/eth-act/zkevm-standards/blob/d5f18133720b7cfc1547a91f78f8e152ec80b2bb/standards/c-interface-accelerators/zkvm_accelerators.h),
vendored unchanged in `include/`. `include/airbender_crypto.h` adds Airbender's own
surface: an inline byte swap (one `rev8` on the guest), the inline operations of the bigint
delegation on 32-byte aligned little-endian integers (one delegation call each on the guest),
streaming Keccak-256 and SHA3-256 over an opaque state in the layout of the keccak
delegation, BLAKE2s, and the hint-recording API. Each export is a thin wrapper over the
`precompiles` endpoints, `modexp`, `blake2f` or `Keccak256Core`; the inline functions have
no Rust behind them.

## C contract

- Include `airbender_crypto.h` (C11 or C++11). Don't compile with `-fshort-enums` or
  struct packing options. All byte structs have alignment 8, also on RV32; `airbender_u256`
  has alignment 32 and `airbender_keccak_state` 256, as their delegations require.
- Pointers must be non-null, aligned and valid for the declared sizes, also for empty inputs.
  The one exception is the size query `airbender_crypto_record_end(NULL, 0)`. Violations
  abort (native) or fail execution (RV32). Inputs may overlap outputs.
- Malformed inputs return `ZKVM_EFAIL` and leave every output byte unchanged. A well-formed
  signature or pairing that does not verify returns `ZKVM_EOK` with `false`.
- Integers and coordinates are big-endian; infinity is all-zero x/y. BN254 Fp2 is
  imaginary-first (EIP-197), BLS12-381 Fp2 real-first (EIP-2537). BLS12-381 coordinates take
  48 bytes, not EIP-2537's 64. `airbender_u256` alone is little-endian: convert a big-endian
  integer word by word with `airbender_bswap32`.
- The bigint operations are `static inline` and work in place on their first operand. Both
  operands must be 32-byte aligned, in RAM on RV32, and must not overlap, so they can never
  be the same address (nothing checks this; a violation is undefined behaviour). They return
  their carry, borrow, comparison or, for `mul_low`, whether the product fits in 256 bits.
  On RV32 each is one delegation call (addresses in x10/x11, the mask in x12,
  `csrrw x0, 0x7ca, x0`, the flag back in x12); define `AIRBENDER_NO_DELEGATION` to get the
  portable C that other targets compile.
- A hasher state is reusable after `finalize_reset`, and needs `init` again after `finalize`.
  Keccak-256 and SHA3-256 share the state type; use one family of functions on a state.
- Scalars are arbitrary 256-bit integers, reduced modulo the group order. Recovery IDs are
  0 to 3, not Ethereum's `v`, and recovery returns x || y without the `0x04` prefix. ECDSA
  verification accepts both s forms.
- An empty BN254 pairing is true; an empty BLS12-381 MSM or pairing fails (EIP-2537).
- `zkvm_modexp` writes exactly `mod_len` bytes, left-padded. A declared length over 1024
  bytes, leading zeroes included, returns `ZKVM_EFAIL` on the lengths alone (EIP-7823), before
  the operands are read.
- `zkvm_blake2f` takes `rounds` as a number (decode EVM's big-endian bytes first) and `f` as
  0 or 1.
- No export allocates. The pairing checks and the MSMs keep their parsed pairs in stack
  buffers sized by the transaction gas cap (EIP-7825): at most 492 BN254 pairs, 513 BLS12-381
  pairs, 2693 G1 and 1423 G2 MSM pairs. More return `ZKVM_EFAIL` on `num_pairs` alone, before
  the input is read. The largest buffer, the G1 MSM's, takes about 600 KB of stack (points,
  scalars and buckets), so the calling thread needs that much.

## Hints

A hint is checked by the operation that asks for it: a wrong or missing hint fails execution
or is ignored, and never changes a result.

- On RV32 with `csr-oracle`, the exports read hints through the non-determinism CSR from the
  guest's input. Without it they take the path without hints.
- A host records the tape natively, with the library built without features: it makes the
  guest's calls, with the same inputs and in the same order, between
  `airbender_crypto_record_begin()` and `airbender_crypto_record_end(words, capacity)`, and
  places the words in the guest's input where the guest makes the calls. Rust hosts use
  `airbender_crypto::hints::record_hints`. The tape is in the representation of the delegated
  fields, for a guest built with `delegation`.
- Recording is per thread. Recordings on several threads are independent only if nothing in
  the build enables `airbender-crypto/proving`, whose emulated delegated fields share one
  scratch space.
- `airbender-crypto/secp256k1-shamir-msm` does not change the tape.

## Features

All RV32 only; `delegation` is a compile error natively. Native builds always have the hinted
operations and their oracle side (`airbender-crypto/oracle_implementations`), for recording.

- `delegation`: delegated big-integer arithmetic (`airbender-crypto/proving`).
- `csr-oracle`: hints through the CSR (`airbender-crypto/hinted_operations`), with or without
  `delegation`.
- `c-runtime`: airbender-rt's panic and allocation-error handlers for C guests, and a global
  allocator whose methods call an undefined function: the library allocates nothing, `alloc` is
  only linked because the dependencies name it, and a guest whose live code could allocate
  fails to link (with `--gc-sections`, which the guest's link must use). Leave the feature off
  in Rust guests that start through airbender-rt, which bring their own handlers and
  allocator.

## Building

```sh
# Native static library; `-- --print native-static-libs` lists the platform libraries
cargo rustc -p airbender-crypto-c --release --crate-type staticlib
# RV32, with c-runtime and fat LTO. FEATURES: 'delegation' (default), 'csr-oracle delegation',
# 'csr-oracle' or ''. Copies the archive to crates/airbender-crypto-c/libairbender_crypto_c.a
sh crates/airbender-crypto-c/build-rv32.sh
```

The RV32 archive targets the standard `riscv32i-unknown-none-elf`, whose prebuilt `core` needs
no build-std, with Zmmul added (multiplication, no division: divisions and remainders are
compiler-builtins calls, which the archive carries; the prebuilt `core`'s own multiplications
are calls too). It defines no `_start` and no strong libc symbols: the C application supplies
its startup, memory layout and stack; it needs no heap. The archive holds every Rust crate
whole, `core` and `compiler_builtins` included, so it is several times the size of a linked
guest, which `--gc-sections` reduces to what the exports reach (about 1 MB of code).

`build-stable.sh` builds the same archives with a stable toolchain (`TOOLCHAIN`, default
1.99.1) instead of the pinned nightly, with `RUSTC_BOOTSTRAP=1` for the crates' nightly
features; `native`, `rv32` or nothing for both.

## Tests

```sh
cargo test -p airbender-crypto-c
sh crates/airbender-crypto-c/tests/c-abi.sh
python3 crates/airbender-crypto-c/tests/rv32-runtime.py --all
```

`c-abi.sh` links `tests/abi.c` as C11 and C++11 against the native archive and checks the
RV32 struct layouts. `rv32-runtime.py` records the tape of `tests/runtime.c` natively, then
builds the fixture for each RV32 configuration and runs it on the FullUnsigned machine through
`airbender-crypto/tests/rv32_runtime.rs`: the results must match the native ones, the tape must
be read exactly, and a tampered tape must trap or change nothing. Linking the guest also checks
that none of its live code can allocate (see `c-runtime`). Both need clang; the RV32
test also needs the pinned toolchain's `llvm-tools-preview`.
