# airbender-host [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![host tooling](https://img.shields.io/badge/runtime-host%20tooling-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_host/) | [Host program guide](https://matter-labs.github.io/airbender-platform/latest/02-host-program-api.html)

`airbender-host` is the host-side API for executing, proving, and verifying Airbender guest programs from native Rust applications.

## What It Provides

- `Program` for loading a packaged `dist/` directory and validating manifest hashes.
- `Inputs` for serializing typed values or raw bytes into the canonical guest input word stream.
- Runner, prover, and verifier builders covering transpiler execution plus dev, CPU, and GPU proving flows.
- Cycle-marker utilities for profiling transpiler runs.

## Features

- `gpu-prover` (default): exposes GPU proving and verification-key generation support.

Disable default features if you want a dev-only host binary without GPU support:

```toml
[dependencies]
airbender-host = { version = "0.1.0", default-features = false }
```

Complete host-side examples live in [`examples/`](https://github.com/matter-labs/airbender-platform/tree/main/examples).

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
