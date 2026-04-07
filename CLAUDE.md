# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

Airbender Platform is a Rust SDK for building zk-provable RISC-V guest programs with host-side execution, proving, and verification. It wraps zksync-airbender v2 (the low-level prover/transpiler) into ergonomic host and guest APIs.

## Toolchain

Rust nightly-2026-02-10 (pinned in `rust-toolchain.toml`). Guest programs target `riscv32i-unknown-none-elf`.

## Build and Test

```bash
# Workspace build (excludes guest crates which need RISC-V target)
cargo build

# Run tests
cargo test

# Clippy and format
cargo clippy --all -- -D warnings
cargo fmt

# Build a guest example (requires cargo-airbender installed)
cargo airbender build --manifest-path examples/fibonacci/guest/Cargo.toml

# Run a host example (after building its guest)
cargo run --manifest-path examples/fibonacci/host/Cargo.toml

# Install the CLI tool
cargo install --path crates/cargo-airbender
```

## Architecture

### Two-program model
- **Guest**: RISC-V binary (`riscv32i-unknown-none-elf`, no_std) — the program being proved
- **Host**: Native binary — loads guest artifacts, provides inputs, runs/proves/verifies

### Crate layout

**Guest-side:**
- `airbender-sdk` (`airbender`) — public guest SDK, re-exports everything below
- `airbender-guest` — `read::<T>()` for inputs, `commit()` for outputs
- `airbender-rt` — boot sequence, allocator, CSR syscalls, panic handler
- `airbender-macros` — `#[airbender::main]` proc macro

**Host-side:**
- `airbender-host` — `Program`, `TranspilerRunner`, `CpuProver`, `GpuProver`, `Verifier`, `Receipt`

**Shared:**
- `airbender-core` — `Commit` trait, wire framing protocol, manifest schema
- `airbender-codec` — versioned serialization (bincode v2 based)
- `airbender-crypto` — shared crypto primitives (SHA256, Keccak, secp256k1, BN254, etc.)

**Tooling:**
- `cargo-airbender` — CLI: `new`, `build`, `run`, `prove`, `verify`, `flamegraph`
- `airbender-build` — guest build orchestration (`build_dist()`)

### Communication protocol
1. Host serializes inputs via `AirbenderCodecV0` → wire-frames as `[u32]`
2. Guest reads via CSR non-determinism source (`QuasiUARTSource`)
3. Guest commits output to registers x10..x17 (8 words, or x10..x25 for extended)
4. Host extracts `Receipt` from final register state

### Upstream dependencies (from zksync-airbender dev branch)
- `riscv_transpiler` — VM execution (JIT on x86_64, interpreter fallback)
- `execution_utils` — CPU/GPU proving wrappers
- `riscv_common` — RISC-V CSR operations, boot primitives
- `common_constants` — ROM specs, timestamps
- `gpu_prover` — GPU proving (optional, requires CUDA)

## Key APIs

**Host execution:**
```rust
let program = Program::load("dist/app")?;
let runner = program.transpiler_runner().with_cycles(1_000_000).build()?;
let result = runner.run(inputs.words())?;
// result.receipt.output == [u32; 8]
```

**Host proving:**
```rust
let prover = program.cpu_prover().build()?;
let prove_result = prover.prove(inputs.words())?;
```

**Guest program:**
```rust
#[airbender::main]
fn main() -> u32 {
    let n: u32 = airbender::guest::read().unwrap();
    fibonacci(n)
}
```

## Build artifacts

`cargo airbender build` produces in `dist/app/`:
- `app.bin` — full memory image (ROM content)
- `app.text` — instruction section only (for transpiler tape)
- `app.elf` — ELF with symbols (for flamegraph/debugging)
- `manifest.toml` — SHA256 hashes, codec version, build metadata

## Features

- `airbender-host`: `gpu-prover` (default) — enables GPU proving backend
- `airbender-sdk`: `std`, `crypto`, `allocator-talc` (default) / `allocator-bump` / `allocator-custom`
- `airbender-crypto`: `forward` (default), `proving`, `secp256k1-static-context` (default)
