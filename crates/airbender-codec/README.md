# airbender-codec [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![no_std supported](https://img.shields.io/badge/no__std-supported-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_codec/) | [Platform guide](https://matter-labs.github.io/airbender-platform/latest/)

> [!WARNING]
> airbender-platform project is under active development and is in alpha state. Use at your own risk.

> [!NOTE]
> **Support crate.** For guest programs, use [`airbender-sdk`](https://matter-labs.github.io/airbender-platform/api/airbender_sdk/), which re-exports this crate's codec helpers via `airbender::codec`. For host programs, [`airbender-host`](https://matter-labs.github.io/airbender-platform/api/airbender_host/) exposes the codec through its `Inputs` API. Depend on this crate directly only when you need explicit control over encoded payloads.

`airbender-codec` defines the stable, versioned serialization layer used between host inputs and guest reads. It is `no_std`-compatible and currently exposes `AirbenderCodecV0`, built on `bincode` v2 with a fixed configuration.

## What It Provides

- `AirbenderCodec`, the trait implemented by concrete codec versions.
- `AirbenderCodecV0`, the current codec used by the platform.
- `AIRBENDER_CODEC_V0` and `CodecError`, so manifests and callers can pin and validate the encoded payload format.

This crate sits underneath `airbender-host::Inputs` and `airbender::guest::read`, but it can also be used directly when you need explicit control over encoded payloads.

## Usage

```toml
[dependencies]
airbender-codec = { git = "https://github.com/matter-labs/airbender-platform", branch = "main" }
```

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
