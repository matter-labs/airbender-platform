# airbender-rt [![Build status](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/matter-labs/airbender-platform/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/matter-labs/airbender-platform#license) ![rust nightly required](https://img.shields.io/badge/rust-nightly-blue.svg?label=Required%20Rust) ![no_std supported](https://img.shields.io/badge/no__std-supported-green.svg)

**Documentation:** [API docs](https://matter-labs.github.io/airbender-platform/api/airbender_rt/) | [Guest program guide](https://matter-labs.github.io/airbender-platform/latest/03-guest-program-api.html)

> [!WARNING]
> airbender-platform project is under active development and is in alpha state. Use at your own risk.

> [!NOTE]
> **Support crate.** Most guest applications should depend on [`airbender-sdk`](https://matter-labs.github.io/airbender-platform/api/airbender_sdk/) instead, which re-exports the runtime, guest API, and codec helpers behind a single dependency.

`airbender-rt` is the low-level runtime support crate for Airbender guest programs. It handles bootstrapping, allocator setup, syscalls, and UART helpers used by the guest-facing APIs.

## What It Provides

- `start(...)` and `start_with_allocator_init(...)` for guest bootstrapping.
- Built-in allocator backends for `talc` and bump allocation, plus hooks for custom allocators.
- Runtime modules for syscalls, UART output, and `getrandom` integration.
- Panic and allocation-error handlers for `no_std` `riscv32` guest builds.

## Features

- `allocator-talc` (default), `allocator-bump`, `allocator-custom`: select the runtime allocator model.
- `std`: enables the small amount of standard-library glue used by `std` guest builds.

## Usage

Most guest applications should depend on `airbender-sdk` or `airbender-guest`. Depend on `airbender-rt` directly only when you need runtime-level control.

## License

Licensed under either [Apache License, Version 2.0](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/matter-labs/airbender-platform/blob/main/LICENSE-MIT) at your option.
