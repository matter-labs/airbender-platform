# Airbender Platform

Airbender Platform is a Rust toolkit for writing RISC-V zk-provable programs.

You write a **guest** program (the code you want to prove) and a **host** program (the native Rust code that feeds inputs, runs the guest, and generates/verifies proofs). The platform handles compilation, execution, proving, and verification.

## What's Inside

**`cargo airbender`** - CLI for the full development lifecycle:
- `new` - scaffold a host+guest project
- `build` - compile guest artifacts (`--reproducible` for deterministic Docker builds)
- `run` - execute a guest binary
- `flamegraph` - profile guest execution
- `prove` / `verify-proof` - generate and verify proofs from the command line

**Guest SDK** (`airbender-sdk`) - everything your guest program needs:
- Entry point macro, `std` support, allocator selection
- Typed input from host, output commitment
- Cycle markers for profiling
- Prover-accelerated crypto primitives

**Host SDK** (`airbender-host`) - drive guest programs from native Rust:
- Load and run RISC-V binaries
- Generate verification keys, prove execution, verify proofs
- Collect cycle-marker snapshots from transpiler runs

## Documentation

Read the **[Airbender Platform Book](https://matter-labs.github.io/airbender-platform/latest)** for the full user guide.
Browse the **[workspace API docs](https://matter-labs.github.io/airbender-platform/api/)** for the current `main` branch rustdocs.

The book source lives in [`docs/`](./docs/). To build locally:

```sh
cargo install mdbook
mdbook serve docs
```

## Examples

Complete guest + host examples are in [`examples/`](./examples/).

## Status

This repository is under active development.

## Policies

- [Security policy](SECURITY.md)
- [Contribution policy](CONTRIBUTING.md)

## License

Airbender Platform is distributed under the terms of either

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
