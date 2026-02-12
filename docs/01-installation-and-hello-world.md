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

Create a new guest project:

```sh
cargo airbender new ./hello-airbender
```

By default, generated projects depend on `airbender-sdk` from this repository (`main` branch). You can override this with:

- `--sdk-path <path>` for a local checkout
- `--sdk-version <version>` once published versions are available

The generated guest reads a `u32` input and returns `value + 1`.

Build the guest:

```sh
cd hello-airbender
cargo airbender build
```

This produces artifacts in `dist/app/` by default:

- `dist/app/app.bin`
- `dist/app/app.elf`
- `dist/app/app.text`
- `dist/app/manifest.toml`

Create an input file (`u32` words encoded as hex, 8 hex chars per word):

```sh
printf '00000029' > input.hex
```

Run in the simulator:

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

`0x00000029` is decimal `41`, so the template guest should produce `42` in output register `x10`.

Generate and verify a proof:

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output ./proof.bin
cargo airbender generate-vk ./dist/app/app.bin --output ./vk.bin
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

## Prefer Full End-to-End Examples

For complete guest + host applications, start with:

- [`examples/fibonacci`](../examples/fibonacci/)
- [`examples/u256-add`](../examples/u256-add/)
- [`examples/std-btreemap`](../examples/std-btreemap/)
