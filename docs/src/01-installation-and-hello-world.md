# Installation & Hello World

This chapter gets you from a fresh machine to your first Airbender guest build, run, and proof.

## Prerequisites

- Rust nightly toolchain from [`rust-toolchain.toml`](https://github.com/matter-labs/airbender-platform/blob/main/rust-toolchain.toml)
- `clang` available in `PATH`
- `cargo-binutils` for `cargo objcopy`

Install `cargo-binutils`:

```sh
cargo install cargo-binutils --locked
```

### Proving Hardware (optional)

The hardware requirements for  `airbender` depend on the proving backend. No specialized hardware
is required for any other `airbender` commands except `airbender prove`.

The following proving backends are supported via `airbender prove --backend [BACKEND]`:

#### `dev`
This backend is used for development purposes to test programs. Useful in scenarios where no cryptographic 
proving is required, e.g. local program testing. Requires no specialized hardware.

#### `cpu`
This backend is mostly used for debugging circuits, as it can only prove the base layer and is usually slow. 
Useful in special scenarios, e.g. identifying constraint failures without running the full end-to-end proving cycle. 
Requires hardware with a powerful CPU and significant RAM (at least 64GB).

### `gpu`
This backend is used for full end-to-end GPU proving. Requires hardware with at least **32GB of VRAM** (e.g. 
NVIDIA RTX 5090 or better) and atleast **64GB of RAM** per GPU with a high-end CPU to avoid bottlenecks.


## Install `cargo airbender`

From a local clone:

```sh
cargo install --path crates/cargo-airbender --force
```

You can also install from the public repository:

```sh
cargo install --git https://github.com/matter-labs/airbender-platform --branch main cargo-airbender --force
```

By default, GPU support is enabled in `cargo-airbender`, so `prove --backend gpu` and `generate-vk`
work out of the box.

If you want to disable GPU support in the CLI binary, install with:

```sh
cargo install --path crates/cargo-airbender --no-default-features --force
```

You can still build (but not run) the project without having Nvidia GPU or CUDA installed by setting
`ZKSYNC_USE_CUDA_STUBS=true` in your environment.

## Hello World (Template Project)

Create a new host+guest template project.

When called without a path, the project is initialized in the current directory:

```sh
cargo airbender new
```

The destination directory must be empty.

When called with a path, the project is initialized there:

```sh
cargo airbender new ./hello-airbender
```

`cargo airbender new` asks interactive questions for:

- project name
- `std` enablement
- allocator (`talc`, `bump`, `custom`)

Flags like `--name`, `--enable-std`, and `--allocator` prefill prompt defaults. For CI/non-interactive usage, add `--yes`:

```sh
cargo airbender new ./hello-airbender --yes --name hello-airbender --enable-std --allocator talc
```

By default, generated projects depend on `airbender-sdk` from this repository (`main` branch). You can override this with:

- `--sdk-path <path>` for a local checkout (workspace root, `crates/`, or crate path)
- `--sdk-version <version>` once published versions are available

The template includes:

- `.gitignore` at project root (ignores `target/`)
- `guest/`: guest program (default: `no_std`; or `std` with `--enable-std`)
- `host/`: host app that runs and optionally proves guest execution
- `guest/.cargo/config.toml` with guest target/build flags used by both `cargo airbender build` and regular `cargo` tooling
- `rust-toolchain.toml` in both crates to pin the compiler channel

The generated guest reads a `u32` input and returns `value + 1`.

Build the guest:

```sh
cd hello-airbender/guest
cargo airbender build
```

The generated `guest/.cargo/config.toml` also makes plain `cargo build` and `cargo check` use the same guest target and flags.

This produces artifacts in `dist/app/` by default:

- `dist/app/app.bin`
- `dist/app/app.elf`
- `dist/app/app.text`
- `dist/app/manifest.toml`

Create an input file (`u32` words encoded as hex, 8 hex chars per word):

> Note: this is a manual codec-v0 payload for `u32 = 41` (used by the template's `read::<u32>()`).

```sh
printf '00000001\n29000000\n' > input.hex
```

Run the program:

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

This input represents a codec-v0 encoded `u32 = 41`, so the template guest should produce `42` in output register `x10`.

For non-trivial input files, generate words from host-side values via `Inputs` methods (see [`docs/02-host-program-api.md`](./02-host-program-api.md)).

Generate and verify a proof:

via the `dev` prover:
```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output ./proof.bin --backend dev
```

Or via the `gpu` prover, if you have the [required](#proving-hardware-optional) hardware:

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output ./proof.bin --backend gpu --level base
cargo airbender generate-vk ./dist/app/app.bin --output ./vk.bin --level base
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

You can also run the generated host flow:

```sh
cd ../host
cargo run
cargo run -- --prove
```

By default, proving uses the dev backend and does not require CUDA.
`airbender-host` exposes GPU proving by default; if you disabled default features in your host
dependency, re-enable `gpu-prover` to call GPU APIs.

## Prefer Full End-to-End Examples

For complete guest + host applications, start with:

- [`examples/fibonacci`](https://github.com/matter-labs/airbender-platform/tree/main/examples/fibonacci)
- [`examples/u256-add`](https://github.com/matter-labs/airbender-platform/tree/main/examples/u256-add)
- [`examples/std-btreemap`](https://github.com/matter-labs/airbender-platform/tree/main/examples/std-btreemap)
