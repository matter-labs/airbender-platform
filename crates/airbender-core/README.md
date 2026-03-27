# airbender-core [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![host and guest](https://img.shields.io/badge/runtime-host%20%2B%20guest-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_core/) | [Platform guide](https://matter-labs.github.io/airbender-platform/latest/)

`airbender-core` contains the shared data model that other Airbender crates build on. It keeps the host and guest boundary explicit: commit layouts, framed input words, and the manifest schema for packaged guest artifacts.

## What It Provides

- `guest::Commit` for mapping values into the public output registers (`x10..x17`).
- `wire` helpers for the canonical host-to-guest framed input format.
- `manifest` types and parsers for the packaged guest artifact schema used by host tooling.

## Features

- `host` (default): enables manifest parsing and serialization support via `serde`, `toml`, and `thiserror`.

Disable default features for `no_std` guest-side use:

```toml
[dependencies]
airbender-core = { version = "0.1.0", default-features = false }
```

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
