# airbender-guest [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![no_std supported](https://img.shields.io/badge/no__std-supported-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_guest/) | [Guest program guide](https://matter-labs.github.io/airbender-platform/latest/03-guest-program-api.html)

`airbender-guest` contains the lower-level guest-side APIs that power `airbender-sdk`. Most applications should depend on the SDK and use `airbender::guest`, but this crate is available directly when you want a thinner guest dependency surface.

## What It Provides

- Typed input reads via `read()` and `read_with(...)`.
- Output helpers via `commit(...)`, `exit_error()`, and the `Commit` trait.
- Development-only profiling hooks via `cycle_marker()` and `record_cycles(...)`.
- `Transport` abstractions for tests and custom integrations.

## Features

- `std`: enables guest-side `std` support where applicable.
- `allocator-talc` (default), `allocator-bump`, `allocator-custom`: forwarded runtime allocator selection.

## Usage

```toml
[dependencies]
airbender-guest = "0.1.0"
```

If you are building a normal guest program, prefer `airbender-sdk` and import these APIs from `airbender::guest`.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
