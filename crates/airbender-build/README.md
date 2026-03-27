# airbender-build [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![host tooling](https://img.shields.io/badge/runtime-host%20tooling-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_build/) | [CLI build reference](https://matter-labs.github.io/airbender-platform/latest/05-cli-reference.html)

`airbender-build` compiles guest crates and packages the resulting `dist/` bundle used by the rest of the platform. It is the library behind `cargo airbender build`, and is useful when you want to embed the same build and packaging flow in Rust tooling.

## What It Provides

- `BuildConfig` for project path, binary selection, profile, target, dist output, forwarded Cargo args, and reproducible-build settings.
- `build_dist(...)` to compile a guest binary, extract `app.bin`, `app.elf`, and `app.text`, and write `manifest.toml`.
- Re-exports of the manifest schema from `airbender-core`, so host tooling can share one artifact format.

Reproducible builds run inside a pinned Docker container and record provenance metadata in the generated manifest.

## Usage

```toml
[dependencies]
airbender-build = "0.1.0"
```

Use `cargo-airbender` for normal CLI workflows. Reach for this crate when you need the same packaging behavior inside a custom build pipeline or host-side tool.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
