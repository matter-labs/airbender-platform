# cargo-airbender [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![cargo subcommand](https://img.shields.io/badge/cargo-subcommand-orange.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/cargo_airbender/) | [CLI reference](https://matter-labs.github.io/airbender-platform/latest/05-cli-reference.html)

`cargo-airbender` is the main CLI for Airbender project scaffolding, guest builds, execution, proving, and verification.

## Commands

- `new`: create a host + guest template project.
- `build`: compile a guest and package a `dist/` bundle.
- `run` and `flamegraph`: execute guest binaries through the transpiler.
- `prove`, `generate-vk`, and `verify-proof`: work with dev, CPU, or GPU proof flows.
- `clean`: remove Docker resources created by reproducible builds.

## Installation

From a local checkout:

```sh
cargo install --path crates/cargo-airbender --force
```

From GitHub:

```sh
cargo install --git https://github.com/matter-labs/airbender-platform --branch main cargo-airbender --force
```

By default GPU support is enabled. Install with `--no-default-features` if you want a CLI binary without GPU proving support.

## Requirements

- Rust nightly pinned by [`rust-toolchain.toml`](https://github.com/matter-labs/airbender-platform/blob/main/rust-toolchain.toml).
- `cargo-binutils` for guest build flows that invoke `cargo objcopy`.
- Docker only when using reproducible builds.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
