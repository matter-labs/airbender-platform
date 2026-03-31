# airbender-crypto [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![host and guest](https://img.shields.io/badge/runtime-host%20%2B%20guest-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_crypto/) | [Crypto guide](https://matter-labs.github.io/airbender-platform/latest/04-crypto-on-guest-and-host.html)

> [!WARNING]
> airbender-platform project is under active development and is in alpha state. Use at your own risk.

> [!NOTE]
> **Support crate.** For guest programs, enable the `crypto` feature on [`airbender-sdk`](https://matter-labs.github.io/airbender-platform/api/airbender_sdk/) to access crypto through `airbender::crypto`. For host programs, this crate can be used directly.

`airbender-crypto` provides shared crypto primitives for Airbender host and guest programs. The same Rust API can be used on both sides, while guest builds can opt into delegated backends for proving-oriented workloads.

## What It Provides

- Hashing modules such as `sha256`, `sha3`, `ripemd160`, and `blake2s`.
- Curve and field modules including `secp256k1`, `secp256r1`, `bn254`, and `bls12_381`.
- Re-exports of supporting crates such as `k256`, `p256`, and arkworks types used by the Airbender integrations.

## Features

- `proving`: enables delegation-oriented features used on `riscv32` Airbender guests.
- Default features keep host usage available and enable the secp256k1 static context.

## Usage

```toml
[dependencies]
airbender-crypto = { git = "https://github.com/matter-labs/airbender-platform", branch = "main" }
```

Use `features = ["proving"]` for guest builds, or enable the `crypto` feature on `airbender-sdk` if you want the same API through the SDK re-export.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
