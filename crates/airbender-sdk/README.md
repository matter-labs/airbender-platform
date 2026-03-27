# airbender-sdk [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![no_std supported](https://img.shields.io/badge/no__std-supported-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_sdk/) | [Guest program guide](https://matter-labs.github.io/airbender-platform/latest/03-guest-program-api.html)

`airbender-sdk` is the main entry point for writing Airbender guest programs. It re-exports the guest API, runtime, codec helpers, and the `#[airbender::main]` attribute behind a single dependency, so guest code can import everything from `airbender`.

## What It Provides

- `#[airbender::main]` for defining the guest entrypoint.
- `airbender::guest` for input reads, output commits, and cycle markers.
- `airbender::rt` and `airbender::codec` for lower-level runtime and serialization access.
- Optional `airbender::crypto` re-exports for shared host/guest crypto code.

## Features

- `std`: enables guest-side standard-library support.
- `crypto`: re-exports `airbender-crypto` with proving-oriented guest backends.
- `allocator-talc` (default), `allocator-bump`, `allocator-custom`: select the guest allocator model.

## Usage

```toml
[dependencies]
airbender = { package = "airbender-sdk", version = "0.1.0" }
```

Complete guest + host examples live in [`examples/`](https://github.com/matter-labs/airbender-platform/tree/main/examples).

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
