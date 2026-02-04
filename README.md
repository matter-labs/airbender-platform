# Airbender Platform

Airbender Platform is a development workspace for building zk-provable applications with Airbender.

It includes:
- `airbender-sdk` for guest programs.
- `airbender-host` for execute/prove/verify host flows.
- `cargo-airbender` (`cargo airbender`) for project scaffolding, guest builds, and runtime proving commands.
- guest/host example applications under `examples/`.

## Status

This repository is under active development.

## Prerequisites

- Rust toolchain from `rust-toolchain.toml`.
- `clang` available in `PATH` (in case of C++ dependencies being used).
- `cargo-binutils` for `cargo objcopy`:

```sh
cargo install cargo-binutils --locked
```

## Install `cargo airbender`

```sh
cargo install --path crates/cargo-airbender --force
```

## Quick Start

Build a guest:

```sh
cd examples/fibonacci/guest
cargo airbender build
```

Build outputs are now namespaced by app name (`app` by default), so artifacts are written to
`dist/<app-name>/` (for example `dist/app/app.bin`). You can create multiple build variants:

```sh
cargo airbender build --app-name with_extra_feature -- --features my_extra_feature
```

Use `--` to forward additional `cargo build` flags.

Run the corresponding host (execute only):

```sh
cd ../host
cargo run
```

Run proof generation + verification:

```sh
cargo run -- --prove
```

## Create a New Guest Project

Default local SDK detection (current default):

```sh
cargo airbender new ./my-guest
```

Explicit local SDK path:

```sh
cargo airbender new ./my-guest --sdk-path /path/to/airbender-platform/crates/airbender-sdk
```

Published SDK version (future-default workflow once published):

```sh
cargo airbender new ./my-guest --sdk-version 0.1.0
```

## Tuning

- Default simulator cycle budget: `100_000_000`.
- Override per run with `AIRBENDER_MAX_CYCLES`:

```sh
AIRBENDER_MAX_CYCLES=500000000 cargo run
```

## Runtime Commands

`cargo airbender` also exposes runtime flows:

```sh
# Run with simulator
cargo airbender run ./dist/app/app.bin --input ./input.hex

# Run via transpiler JIT
cargo airbender run-transpiler ./dist/app/app.bin --input ./input.hex

# Emit flamegraph SVG
cargo airbender flamegraph ./dist/app/app.bin --input ./input.hex --output flamegraph.svg

# Generate proof
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output proof.bin

# Generate verification keys
cargo airbender generate-vk ./dist/app/app.bin --output vk.bin

# Verify proof
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

By default, command logs use `info` level. Override with `RUST_LOG`, for example:

```sh
RUST_LOG=debug cargo airbender prove ./dist/app/app.bin --input ./input.hex --output proof.bin
```
