# Airbender Examples

Each example contains two crates:

- `guest/`: guest program built with the Airbender toolchain.
- `host/`: native runner that executes, proves, and verifies the guest program.

## Toolchain

Each guest directory contains a `rust-toolchain.toml` that pins the nightly toolchain to the same version used inside the reproducible build container (`DEFAULT_GUEST_TOOLCHAIN` in `crates/airbender-build/src/constants.rs`). `rustup` picks this up automatically — no `+toolchain` override is needed when running `cargo` commands from a guest directory.

## Build and Run

From a guest directory:

```sh
cargo airbender build
```

From the corresponding host directory:

```sh
cargo run --release
```

To generate and verify a proof:

```sh
cargo run --release -- --prove
```

## Examples

- `cycle-markers` (transpiler cycle profiling + delegation snapshots)
- `fibonacci`
- `u256-add` (no_std + `ruint`)
- `std-btreemap` (std + `BTreeMap`)
- `revm-witness-replay` (`revm` EVM execution inside Airbender with calldata, memory ops, and ZK-proven output)
- `reth-block-replay` (fetch a block from reth, re-execute with `paradigmxyz/stateless`, and prove in ZK)
