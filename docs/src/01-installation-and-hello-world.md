# Installation & Hello World

Before diving into the APIs, let's make sure everything works. We'll install the toolchain, scaffold a project, and prove a simple program end to end.

## Prerequisites

- Rust nightly toolchain from [`rust-toolchain.toml`](https://github.com/matter-labs/airbender-platform/blob/main/rust-toolchain.toml)
- `clang` available in `PATH`
- `cargo-binutils` for `cargo objcopy`
- Docker (only needed for `cargo airbender build --reproducible`)

Install `cargo-binutils`:

```sh
cargo install cargo-binutils --locked
```

## Install `cargo airbender`

From a local clone:

```sh
cargo install --path crates/cargo-airbender --force
```

Or directly from the repository:

```sh
cargo install --git https://github.com/matter-labs/airbender-platform --branch main cargo-airbender --force
```

GPU support is enabled by default, so `prove --backend gpu` and `generate-vk` work out of the box. To install without GPU support:

```sh
cargo install --path crates/cargo-airbender --no-default-features --force
```

You can compile (but not run) GPU-dependent code without NVIDIA hardware by setting `ZKSYNC_USE_CUDA_STUBS=true`.

## Create Your First Project

Scaffold a new host+guest project:

```sh
cargo airbender new ./hello-airbender
```

The command asks for a project name, whether to enable `std`, which allocator to use, and which prover backend (dev or gpu). For now, accept the defaults.

> For CI or scripted usage, pass `--yes` to skip prompts:
> `cargo airbender new ./hello-airbender --yes --name hello-airbender`

The generated project has two crates:

- `guest/` - a RISC-V program that reads a `u32` input and returns `value + 1`
- `host/` - a native Rust program that feeds inputs and runs the guest

Build the guest:

```sh
cd hello-airbender/guest
cargo airbender build
```

This produces artifacts in `dist/app/`:

```text
dist/app/app.bin
dist/app/app.elf
dist/app/app.text
dist/app/manifest.toml
```

Now run it from the host:

```sh
cd ../host
cargo run --release
```

You should see the execution output. The host feeds `41` as input, the guest returns `42`.

## Prove Your First Program

Generate and verify a dev proof:

```sh
cargo run --release -- --prove
```

That's it. The host runs the guest, generates a proof, and verifies it. The dev backend doesn't require a GPU and is meant for local development.

For real GPU proving, see [Proving Hardware](#proving-hardware) below and the [CLI Reference](./05-cli-reference.md).

## What Just Happened?

The generated `guest/.cargo/config.toml` configures the RISC-V target and build flags. This means plain `cargo build` and `cargo check` also work for the guest. `cargo airbender build` adds artifact packaging on top (binary, ELF, text sections, manifest with SHA-256 hashes).

The host uses `airbender-host` to load the guest binary, serialize inputs with `Inputs::push(...)`, and call the runner/prover APIs. See [Host Program API](./02-host-program-api.md) for the full API.

## CLI-Only Workflow

You can also run and prove guest programs directly from the CLI without writing a host program.

Create an input file (hex-encoded `u32` words, 8 hex chars per word):

> This is a codec-v0 payload for `u32 = 41`.

```sh
printf '00000001\n29000000\n' > input.hex
```

Run:

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

Prove with the dev backend:

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output ./proof.bin --backend dev
```

Or with the GPU backend (requires [compatible hardware](#proving-hardware)):

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output ./proof.bin --backend gpu --level base
cargo airbender generate-vk ./dist/app/app.bin --output ./vk.bin --level base
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

For non-trivial inputs, use the host-side `Inputs::push(...)` API and `write_hex_file(...)` to generate input files. See [Host Program API](./02-host-program-api.md).

## Proving Hardware

No specialized hardware is needed for development. The proving backends have different requirements:

| Backend | Use case | Hardware |
|---------|----------|----------|
| `dev` | Local testing, no real proving | Any machine |
| `cpu` | Debugging circuits (base layer only, slow) | Powerful CPU, 64GB+ RAM |
| `gpu` | Full end-to-end proving | NVIDIA GPU with 32GB+ VRAM, 64GB+ RAM |

## Next Steps

- Read the [Guest Program API](./03-guest-program-api.md) to learn how to write real guest programs.
- Read the [Host Program API](./02-host-program-api.md) to learn how to feed inputs and verify proofs.
- Browse the [examples](https://github.com/matter-labs/airbender-platform/tree/main/examples) for complete working projects.
