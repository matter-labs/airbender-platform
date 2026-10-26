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
- Hints ("advice"): field inverses, square roots, quotients and pairing witnesses that a prover supplies through an oracle and the guest checks (`hints`), the crypto halves of the zksync-os precompiles that ask for them (`precompiles`), and ModExp (`modexp`). See [Hints, precompiles and ModExp](#hints-precompiles-and-modexp).

## Features

- `proving`: enables delegation-oriented features used on `riscv32` Airbender guests. It implies `bigint_ops`, which the hints of the bn254 and bls12-381 fields need on `riscv32`.
- `alloc`: the endpoints and the ModExp that allocate (the pairing checks, the MSMs, `modexp`), and the oracle side of the hints on 64-bit hosts (native recording).
- Default features keep host usage available and enable the secp256k1 static context.

## Usage

```toml
[dependencies]
airbender-crypto = { git = "https://github.com/matter-labs/airbender-platform", branch = "main" }
```

Use `features = ["proving"]` for guest builds, or enable the `crypto` feature on `airbender-sdk` if you want the same API through the SDK re-export.

## Hints, precompiles and ModExp

### The hint protocol

A hint ("advice") is a value that the prover supplies through an oracle and that the guest checks
before it uses it: an inverse with one multiplication, a square root with one squaring, a pairing's
residue witness by its equation. A wrong or missing answer either fails the computation or is
ignored, and no answer changes a result. The ignored words are bn254 G2 slopes: the slope answer
is fixed-size, and the slopes that a chain does not use, or all of them when the prover marks the
chain as exceptional, are skipped (`HintedChain`; the projective computation runs in place of an
exceptional chain). A noncanonical answer fails the computation too, except a secp256k1 quotient
on RV32, which is taken as it is and checked modulo `p` by one multiplication: a noncanonical
representative of the right quotient is the same element. The protocol is that of zksync-os,
unchanged (`hints`):

- A query is **2 oracle words**: on RV32, two writes to the non-determinism CSR, the query ID and
  then the address of a request struct in guest memory.
- The query IDs are `0x4005_0011` (field hints, the 15 `FieldHintOp`s: square roots, inverses and
  quotients of the secp256k1, bn254 and bls12-381 fields, pairing residue witnesses, G2 slopes),
  `0x4005_0010` (ModExp reductions of a multi-digit modulus) and `0x4005_0031` (ModExp wide
  quotients of a single-digit modulus).
- The answer is **raw `u32` words**, read with one CSR read per word, and the protocol adds no
  framing. The field hints (`0x4005_0011`) and the ModExp wide quotient (`0x4005_0031`) have **no
  length prefix**, and their field elements are the raw Montgomery limbs of the delegated fields
  of the guest. The ModExp reduction answer (`0x4005_0010`) **starts with two length words**, the
  lengths of the quotient and of the remainder in `u32` words, which are not Montgomery limbs;
  their words follow.
- `HintOracle` is the querier's side of it. `CsrHintOracle` (RV32 only) is the CSR,
  `RecordingHintOracle` (below) answers a querier on a native host, and `NoHints` is the type of
  no oracle.

### Which path runs

Only the presence of an oracle selects the path; no Cargo feature does. Every endpoint (the
`precompiles` functions, `modexp::modexp`, `verify_kzg_proof_with_oracle`) takes an
`Option<&mut O>` with `O: HintOracle`. With `Some(oracle)` it runs the hinted path, which asks the
hints of zksync-os `e855bca2` (ModExp: `185a0568`) in the same order and checks them. With `None`
(write `None::<&mut NoHints>`) it runs the path without advice. The results are the same, but for
the raw output of `modexp::modexp`: the two paths can return a different number of leading zero
bytes (a zero result is `[0]` without advice, and no bytes with it), and agree once
`modexp::write_modexp_output` pads (or truncates) the output to the length of the modulus.

The hint functions below the endpoints (`hints::bn254_fq_inverse`, `hints::bls12_381_fq_sqrt`, the
dividers, `Secp256k1HooksWithOracle::new`, and so on) are not endpoints: they take the oracle
itself (`&mut O`) and always use the oracle they are given. They do not choose a path: the
endpoint, or its caller, decides whether to call them.

On RV32 the hints answer straight into the delegated fields, so they need `bigint_ops` (which
`proving` turns on). Without it, the hint items of the bn254 and bls12-381 fields are not there,
and the endpoints run with `None` and panic with `Some`.

### Native recording

The hint tape of a guest is recorded natively, before the RV32 run, as zksync-os's native run
does. This needs `alloc` and a 64-bit host, and does not need `proving`:

```text
use airbender_crypto::hints::{record_hints, NoHints};
use airbender_crypto::precompiles::bn254_g1_add;

// the hinted path, and the words that the guest reads when it makes the same call
let (sum, tape) = record_hints(|oracle| bn254_g1_add(&points, Some(oracle)));
// the path without hints gives the same result
assert_eq!(sum, bn254_g1_add(&points, None::<&mut NoHints>));
```

The tape is the words of the answers in query order; the host feeds it to the guest as input,
where the guest makes its calls. The recorder needs `alloc` only, not `std`. Callers that cannot
pass an oracle through their calls keep the recorder themselves: `airbender-crypto-c` keeps one
per thread for its C recording API (`airbender_crypto_record_begin` and
`airbender_crypto_record_end`).

A recorder keeps its state in itself, so recordings on several threads are independent, **but
only if nothing in the build turns `proving` on for a native target**. With `proving`, the native
fields are the emulated delegated ones, which share one scratch space (a `static mut`) between all
threads: recordings, and every other use of those fields, must then be serialized by the caller.
A crate elsewhere in the dependency graph can turn `proving` on (Cargo unifies features), so a
threaded recorder should not rely on this crate's own manifest.

### Entry points

`precompiles` holds the crypto halves of the zksync-os precompiles: the EVM halves (gas, the padding
and size checks of the input, the encoding of the output, the cycle markers) stay in zksync-os.
Each endpoint takes an optional oracle; those that allocate also take an allocator (and need
`alloc`, or `cfg(test)`).

| Precompile | Endpoints |
| --- | --- |
| ecrecover (needs `secp256k1-static-context`) | `secp256k1_ecrecover` |
| bn254 (EIP-196, EIP-197) | `bn254_g1_add`, `bn254_g1_mul`, `bn254_pairing_check` (allocates) |
| bls12-381 (EIP-2537) | `bls12_381_g1_add`, `bls12_381_g2_add`, `bls12_381_map_fp_to_g1`, `bls12_381_map_fp2_to_g2`, `bls12_381_g1_msm` and `bls12_381_g2_msm` (both allocate), `bls12_381_pairing_check` (allocates) |
| point evaluation (EIP-4844) | `kzg_verify_proof`, `verify_kzg_proof_with_oracle` (with the parsers `parse_g1_compressed_with_oracle` and `parse_scalar`) |

`modexp::modexp(base, exp, modulus, allocator, oracle)` is ModExp, with `modexp::write_modexp_output`
for the padding of the result (needs `alloc`). **The advice ModExp only builds on `riscv32` or on
64-bit little-endian targets** (a `compile_error!` ported from zksync-os, which applies to every
build of this crate with `alloc`). See [`src/modexp/README.md`](https://github.com/matter-labs/airbender-platform/blob/main/crates/airbender-crypto/src/modexp/README.md).

Below them are the hinted operations that the endpoints and the C bindings compose (they take an
oracle, not an `Option`: see "Which path runs"):
`hints::{bn254,bls12_381}_fq_inverse`, `hints::{bn254,bls12_381}_fq12_inverse`,
`hints::bls12_381_fq2_{inverse,sqrt}`, the dividers (`hints::Bn254FqDivider`,
`hints::Bls12381FqDivider`, `hints::Bls12381Fq2Divider`), `hints::Secp256k1HooksWithOracle`,
`hinted_ops::to_affine_with_inverse`, and the `*_with_divider` and `into_affine_with_inverse`
functions of `bls12_381::{g1, g2}`.

Rust and C use the same operations; the C bindings are in
[`airbender-crypto-c`](https://github.com/matter-labs/airbender-platform/tree/main/crates/airbender-crypto-c).

Every ported file starts with a `// Ported from` line that names the zksync-os revision and path;
[`src/hints/README.md`](https://github.com/matter-labs/airbender-platform/blob/main/crates/airbender-crypto/src/hints/README.md)
maps each file to its sources.

### Invariants

Typed curve inputs of the functions that take a divider or an inverse (`*_with_divider`,
`into_affine_with_inverse`, `hinted_ops::to_affine_with_inverse`) must be on the curve, and those
functions do not validate it: callers using `new_unchecked` or writing public coordinates are
responsible for this invariant. The byte-level endpoints of `precompiles` establish it when they
decode their input, and keep the subgroup requirements of zksync-os: the MSMs and the pairing
checks check every point, including one with a zero scalar or an infinity partner.

### Testing

The unit tests of the crate run on the delegated fields of the test build, whose representation is
the guest's. A build without `cfg(test)` has the arkworks fields instead (a bls12-381 element is 6
limbs there, not 8, and a secp256k1 answer is a canonical integer, not Montgomery words), so
separate integration tests cover it, and **they only compile with `--features alloc`**:
`tests/hints_native_fields.rs`, `tests/hints_bls_pairing_layout.rs`, `tests/precompiles_native.rs`
and `tests/modexp_native.rs`. `cargo test -p airbender-crypto` alone skips them silently. A
workspace run (`cargo nextest run --workspace`, as CI does) gets `alloc` through
`airbender-crypto-c`.

```sh
cargo test -p airbender-crypto --features alloc
cargo test -p airbender-crypto --features alloc --release
```

A debug run uses reduced sets for the hinted property tests, because an answer of the recording
oracle takes milliseconds on the emulated delegated fields: 16 cases instead of 64 for the MSM,
pairing and KZG tests (`HINT_HEAVY_CASES`), and fewer random scalars and points in the MSM
tests. **The release run is authoritative for the full test sets of zksync-os.** The 2 blake2s
tests that need `blake2s/test_program/dump_bin.sh` fail without it.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.

