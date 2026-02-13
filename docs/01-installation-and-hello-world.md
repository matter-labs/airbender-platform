# Installation & Hello World

This chapter gets you from a fresh machine to your first Airbender guest build, run, and proof.

## Prerequisites

- Rust nightly toolchain from [`rust-toolchain.toml`](../rust-toolchain.toml)
- `clang` available in `PATH`
- `cargo-binutils` for `cargo objcopy`

Install `cargo-binutils`:

```sh
cargo install cargo-binutils --locked
```

## Install `cargo airbender`

From a local clone:

```sh
cargo install --path crates/cargo-airbender --force
```

You can also install from the public repository:

```sh
cargo install --git https://github.com/popzxc/airbender-platform --branch main cargo-airbender --force
```

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

Run in the simulator:

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

This input represents a codec-v0 encoded `u32 = 41`, so the template guest should produce `42` in output register `x10`.

For non-trivial input files, generate words from host-side values via `Inputs` methods (see [`docs/02-host-program-api.md`](./02-host-program-api.md)).

Generate and verify a proof:

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output ./proof.bin
cargo airbender generate-vk ./dist/app/app.bin --output ./vk.bin
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

You can also run the generated host flow:

```sh
cd ../host
cargo run
cargo run -- --prove
```

If CUDA is not configured on your machine, prefix host commands with `ZKSYNC_USE_CUDA_STUBS=true`.

## Prefer Full End-to-End Examples

For complete guest + host applications, start with:

- [`examples/fibonacci`](../examples/fibonacci/)
- [`examples/u256-add`](../examples/u256-add/)
- [`examples/std-btreemap`](../examples/std-btreemap/)
