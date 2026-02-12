# Airbender Platform

Airbender Platform is a workspace for building zk-provable programs with guest and host tooling.

This project provides:

- `cargo airbender`: an utility to manage airbender projects and interact with the built RISC-V programs:
    - Create projects with `cargo airbender new`
    - Build projects with `cargo airbender build`
    - Run RISC-V programs with `cargo airbender run` and `cargo airbender run-transpiler`
    - Benchmark programs with `cargo airbender flamegraph`
    - Prove and verify proofs from CLI via `cargo airbender prove` & `cargo airbender verify-proof`.
- Guest SDK: a set of utilities to make building guest programs convenient:
    - Project scaffolding: entrypoint, `std` bindings, allocator.
    - Reading input from host.
    - Committing values.
    - Passing debug logs.
    - Accessing prover-accelerated crypto primitives.
- Host SDK: a set of utilities to interact with your program:
    - Load and run RISC-V projects from Rust.
    - Generate verification keys, prove execution, verify proofs.

## Documentation

The user manual lives in [`docs/`](./docs/).

Start here:

- [`docs/README.md`](./docs/README.md) (table of contents)
- [`docs/01-installation-and-hello-world.md`](./docs/01-installation-and-hello-world.md)
- [`docs/02-host-program-api.md`](./docs/02-host-program-api.md)
- [`docs/03-guest-program-api.md`](./docs/03-guest-program-api.md)
- [`docs/04-crypto-on-guest-and-host.md`](./docs/04-crypto-on-guest-and-host.md)
- [`docs/05-cli-reference.md`](./docs/05-cli-reference.md)

## Examples

Complete guest + host examples are in [`examples/`](./examples/).

## Status

This repository is under active development.
