# Airbender C crypto accelerators

A C ABI over `airbender-crypto`. It implements all 19 functions of the eth-act
[`zkvm_accelerators.h`](https://github.com/eth-act/zkevm-standards/blob/d5f18133720b7cfc1547a91f78f8e152ec80b2bb/standards/c-interface-accelerators/zkvm_accelerators.h),
vendored unchanged in `include/`. `include/airbender_crypto.h` adds Airbender's field
inversion, division and square roots, Jacobian normalization, BLS12-381 subgroup checks and
cofactor clearing, BLAKE2s, and the hint-recording API. Each export is a thin wrapper over
the `precompiles` endpoints, `modexp`, `blake2f` or a checked hinted operation.

## C contract

- Include `airbender_crypto.h` (C11 or C++11). Don't compile with `-fshort-enums` or
  struct packing options. All byte structs have alignment 8, also on RV32.
- Pointers must be non-null, aligned and valid for the declared sizes, also for empty inputs.
  The one exception is the size query `airbender_crypto_record_end(NULL, 0)`. Violations
  abort (native) or fail execution (RV32). Inputs may overlap outputs.
- Malformed inputs return `ZKVM_EFAIL` and leave every output byte unchanged. A well-formed
  signature or pairing that does not verify returns `ZKVM_EOK` with `false`.
- Integers and coordinates are big-endian; infinity is all-zero x/y. BN254 Fp2 is
  imaginary-first (EIP-197), BLS12-381 Fp2 real-first (EIP-2537). BLS12-381 coordinates take
  48 bytes, not EIP-2537's 64. The Airbender field functions put c0 first, BN254 Fq2 included.
- Scalars are arbitrary 256-bit integers, reduced modulo the group order. Recovery IDs are
  0 to 3, not Ethereum's `v`, and recovery returns x || y without the `0x04` prefix. ECDSA
  verification accepts both s forms.
- An empty BN254 pairing is true; an empty BLS12-381 MSM or pairing fails (EIP-2537).
- `zkvm_modexp` writes exactly `mod_len` bytes, left-padded; there is no size limit.
- `zkvm_blake2f` takes `rounds` as a number (decode EVM's big-endian bytes first) and `f` as
  0 or 1.
- The pairing checks, MSMs and ModExp allocate; the other exports don't.

## Hints

A hint is checked by the operation that asks for it: a wrong or missing hint fails execution
or is ignored, and never changes a result.

- On RV32 with `csr-oracle`, the exports read hints through the non-determinism CSR from the
  guest's input. Without it they take the path without hints.
- A host records the tape natively, with the library built without features: it makes the
  guest's calls, with the same inputs and in the same order, between
  `airbender_crypto_record_begin()` and `airbender_crypto_record_end(words, capacity)`, and
  places the words in the guest's input where the guest makes the calls. Rust hosts use
  `airbender_crypto::hints::record_hints`.
- Recording is per thread. Recordings on several threads are independent only if nothing in
  the build enables `airbender-crypto/proving`, whose emulated delegated fields share one
  scratch space.
- `airbender-crypto/secp256k1-shamir-msm` does not change the tape.

## Features

All RV32 only; `delegation` is a compile error natively.

- `delegation`: delegated big-integer arithmetic (`airbender-crypto/proving`).
- `csr-oracle` (requires `delegation`): hints through the CSR.
- `c-runtime`: airbender-rt's allocator and panic handler for C guests. The guest calls
  `airbender_crypto_init_heap(start, end)` once before anything allocates. Leave it off in
  Rust guests that start through airbender-rt.

## Building

```sh
# Native static library; `-- --print native-static-libs` lists the platform libraries
cargo rustc -p airbender-crypto-c --release --crate-type staticlib
# RV32, with c-runtime. FEATURES: 'delegation' (default), 'csr-oracle delegation' or ''
sh crates/airbender-crypto-c/build-rv32.sh
```

The RV32 archive defines no `_start` and no strong libc symbols: the C application supplies
its startup, memory layout, stack and heap.

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
be read exactly, and a tampered tape must trap or change nothing. Both need clang; the RV32
test also needs the pinned toolchain's `llvm-tools-preview`.
