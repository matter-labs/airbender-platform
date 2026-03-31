# airbender-macros [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![proc macro crate](https://img.shields.io/badge/crate-proc--macro-orange.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_macros/) | [Guest program guide](https://matter-labs.github.io/airbender-platform/latest/03-guest-program-api.html)

> [!WARNING]
> airbender-platform project is under active development and is in alpha state. Use at your own risk.

> [!NOTE]
> **Support crate.** Most guest applications should depend on [`airbender-sdk`](https://matter-labs.github.io/airbender-platform/api/airbender_sdk/) instead, which re-exports `#[airbender::main]` and all guest APIs through a single dependency.

`airbender-macros` implements the procedural macros used by Airbender guest programs. In practice, most projects reach this crate through `airbender-sdk`, which re-exports `#[airbender::main]`.

## What It Provides

- `#[airbender::main]`, the guest entrypoint attribute.
- Compile-time validation that the annotated function is synchronous, takes no arguments, and returns a committable value.
- Generated `_start_rust` entrypoint wiring, including optional custom allocator initialization.

## Usage

Most guest applications should depend on the SDK:

```toml
[dependencies]
airbender = { package = "airbender-sdk", git = "https://github.com/matter-labs/airbender-platform", branch = "main" }
```

Direct use of `airbender-macros` is uncommon because the generated code targets the `airbender` SDK re-export surface.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
