# Airbender Platform

Airbender Platform is a workspace for building zk-provable programs with guest and host tooling.

This project provides:

- `cargo airbender`: an utility to manage airbender projects and interact with the built RISC-V programs:
    - Create host+guest projects with `cargo airbender new`
    - Build projects with `cargo airbender build`
    - Run RISC-V programs with `cargo airbender run`
    - Benchmark programs with `cargo airbender flamegraph`
    - Prove and verify proofs from CLI via `cargo airbender prove` & `cargo airbender verify-proof`.
- Guest SDK: a set of utilities to make building guest programs convenient:
    - Project scaffolding: entrypoint, `std` bindings, allocator.
    - Reading input from host.
    - Committing values.
    - Emitting cycle markers for transpiler profiling.
    - Passing debug logs.
    - Accessing prover-accelerated crypto primitives.
- Host SDK: a set of utilities to interact with your program:
    - Load and run RISC-V projects from Rust.
    - Collect cycle-marker snapshots from transpiler runs.
    - Generate verification keys, prove execution, verify proofs.

## Documentation

Read the **[Airbender Platform Book](https://matter-labs.github.io/airbender-platform/latest)** for the full user guide.

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
