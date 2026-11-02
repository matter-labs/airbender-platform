# airbender-crypto [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![host and guest](https://img.shields.io/badge/runtime-host%20%2B%20guest-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_crypto/) | [Crypto guide](https://matter-labs.github.io/airbender-platform/latest/04-crypto-on-guest-and-host.html)

> [!WARNING]
> airbender-platform project is under active development and is in alpha state. Use at your own risk.

> [!NOTE]
> **Support crate.** For guest programs, enable the `crypto` feature on [`airbender-sdk`](https://matter-labs.github.io/airbender-platform/api/airbender_sdk/) to access crypto through `airbender::crypto`. For host programs, this crate can be used directly.

`airbender-crypto` provides shared crypto primitives for Airbender host and guest programs. The same Rust API can be used on both sides, while guest builds can opt into delegated backends for proving-oriented workloads.

## What It Provides

- Hashing modules such as `sha256`, `sha3`, `ripemd160`, and `blake2s`.
- Curve and field modules including `secp256k1`, `secp256r1`, `bn254`, and `bls12_381`.
- Re-exports of supporting crates such as `k256`, `p256`, and arkworks types used by the Airbender integrations.
- Hints: values a prover supplies through an oracle and the guest checks (`hints`), the crypto halves of the EVM precompiles that use them (`precompiles`), and ModExp (`modexp`). See [Hints and precompiles](#hints-and-precompiles).

## Features

- `proving`: enables delegation-oriented features used on `riscv32` Airbender guests. It implies `bigint_ops` (the bigint delegation of the fields) and `hinted_operations`.
- `hinted_operations`: the hinted paths of the primitives, which take oracle witnesses (`hints`). Independent of `bigint_ops`: the answers are read in the representation of the fields of the build.
- `oracle_implementations`: the oracle side on a native host, which computes the witnesses (`*/oracle_implementations`, `oracle_common`) and records the hint tape; it allocates.
- `alloc`: the allocating endpoints (pairing checks, MSMs).
- Default features keep host usage available and enable the secp256k1 static context and the hinted operations.

## Usage

```toml
[dependencies]
airbender-crypto = { git = "https://github.com/matter-labs/airbender-platform", branch = "main" }
```

Use `features = ["proving"]` for guest builds, or enable the `crypto` feature on `airbender-sdk` if you want the same API through the SDK re-export.

## Hints and precompiles

A hint is a value the prover supplies through an oracle and the guest checks before use: an
inverse by one multiplication, a square root by one squaring, a pairing residue witness by its
equation. A wrong answer fails the computation or is ignored (unused bn254 G2 slope words); it
never changes a result.

Every endpoint (the `precompiles` functions and `modexp::modexp`) takes `Option<&mut O>` with
`O: MemoryOracle` (from `airbender-oracle`, whose `Error` type is the caller's). `Some(oracle)` runs
the hinted path, which asks the same queries as zksync-os (in the same order, except that the bn254
pairing check asks its residue witness before the hints of the pairs, which it then takes a chunk
of pairs at a time); `None::<&mut NoHints>` runs the plain path. Both give the same result (for
`modexp`, after `modexp::write_modexp_output` pads it). The hinted paths need `hinted_operations`;
without it the endpoints panic when given an oracle. `precompiles` covers ecrecover, bn254
(EIP-196/197), BLS12-381 (EIP-2537) and KZG point evaluation (EIP-4844); `airbender-crypto-c`
exposes them to C. ModExp allocates nothing: its operands are bounded by EIP-7823.

The oracle side of every hint lives in the `oracle_implementations` module of its primitive
(`secp256k1`, `bn254`, `bls12_381`, `modexp`), behind the `oracle_implementations` feature; what
they share (the residue witnesses, the operands read from the querier's memory, the recorder) is
in `oracle_common`. The answers are in the representation of the querier's target
(`hints::HintTarget`): a querier in this process, or the RISC-V guest with `bigint_ops`.

A query is two words written to the non-determinism CSR: the query ID, then the address of a
request in guest memory. The answer is raw `u32` words, one CSR read each, with no framing. The
query IDs are allocated in `airbender_oracle::query_ids`. Field hints use `0x4005_0011`, and their
elements are the raw Montgomery limbs of the guest's delegated fields; the ModExp queries are
described in `src/modexp/README.md`.

ecrecover takes the recovered key from the oracle with a short 4-dimensional decomposition of the
signature (`FieldHintOp::Secp256k1RecoveryWitness`), and checks it with about a fifth fewer group
operations than computing it. A witness that does not check out panics, as any wrong hint does; the
oracle answers "no witness" for a key at infinity, and the key is then computed.

Functions that take a divider or a hinted inverse (`*_with_divider`, `into_affine_with_inverse`,
`hinted_ops::to_affine_with_inverse`) assume their points are on the curve. The byte-level
endpoints check this when decoding, and the MSMs and pairing checks also check the subgroup.

### Recording hints

With `oracle_implementations` on a 64-bit host, `record_hints` returns the tape the guest will read:

```text
let (sum, tape) = record_hints(|oracle| bn254_g1_add(&points, Some(oracle)));
assert_eq!(sum, bn254_g1_add(&points, None::<&mut NoHints>));
```

Recorders on different threads are independent, unless `proving` is enabled on a native target
(possibly by another crate, through feature unification): its emulated delegated fields share one
`static mut` scratch space, so the caller must serialize every use of them.

### Testing

Unit tests run on the delegated fields of the test build, which have the guest's representation.
The integration tests in `tests/` cover the arkworks fields of a normal build, and the ones for
hints, precompiles and ModExp only compile with the oracle side (and `alloc` for the allocating
endpoints):

```sh
cargo test -p airbender-crypto --features alloc,oracle_implementations
```

Debug builds run fewer property-test cases for the hint-heavy endpoints; `--release` runs them all.
The two blake2s tests need the RISC-V program built by `blake2s/test_program/dump_bin.sh`.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.

