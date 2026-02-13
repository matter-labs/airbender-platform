# CLI Reference

`cargo airbender` is the main user CLI for project scaffolding, guest builds, execution, proving, and verification.

## Top-Level Commands

```text
build
new
run
flamegraph
run-transpiler
prove
generate-vk
verify-proof
```

## `cargo airbender build`

Builds guest artifacts into a dist app directory.

```sh
cargo airbender build --app-name app
```

Key options:

- `--app-name <name>`: output namespace under dist root (default: `app`)
- `--bin <name>`: explicit Cargo binary target
- `--target <triple>`: explicit target triple override (otherwise Cargo config defaults are used)
- `--dist <path>`: dist root directory (app folder is created under this root; relative paths are resolved from command invocation cwd)
- `--project <path>`: guest project directory
- `--profile <debug|release>`, `--debug`, `--release`

Forward extra Cargo flags after `--`:

```sh
cargo airbender build --app-name with_extra_feature -- --features my_extra_feature
```

Default artifact layout:

```text
dist/<app-name>/app.bin
dist/<app-name>/app.elf
dist/<app-name>/app.text
dist/<app-name>/manifest.toml
```

## `cargo airbender new`

Creates a new host+guest project template.

```sh
cargo airbender new [path]
```

This command always runs in interactive mode and asks:

- project name
- whether to enable `std`
- allocator mode (`talc`, `bump`, `custom`)
- prover backend (`dev`, `gpu`)

If `[path]` is omitted, the project is initialized in the current directory.
The destination directory must be empty.

Use `--yes` to skip prompts and run non-interactively.

Options:

- `--name <name>`: default project name for interactive mode (or value used with `--yes`)
- `--enable-std`: default `std` answer for interactive mode (or value used with `--yes`)
- `--allocator <talc|bump|custom>`: default allocator answer for interactive mode (or value used with `--yes`)
- `--prover-backend <dev|gpu>`: default prover backend answer for interactive mode (or value used with `--yes`)
- `--yes`: skip prompts and accept values from flags/defaults
- `--sdk-path <path>`: use local SDK path (workspace root, `crates/`, or crate path)
- `--sdk-version <version>`: use versioned SDK dependency

Prover backend choices:

- `dev`: transpiler-backed development flow that emits a mock proof envelope instead of running cryptographic proving
- `gpu`: real proving backend; requires a CUDA-capable NVIDIA GPU at runtime. You can compile with `ZKSYNC_USE_CUDA_STUBS=true`, but invoking proving without CUDA setup panics.

If `custom` allocator is chosen, the guest code will have `#[airbender::main(allocator_init = ...)]` and an explicit allocator
module you can replace. The `allocator_init` hook is required for `allocator-custom`.

Default behavior (when neither `--sdk-path` nor `--sdk-version` is provided):

- generated project depends on `airbender-sdk` from
  `https://github.com/popzxc/airbender-platform` (branch `main`)

Generated layout:

```text
<project>/
  .gitignore
  README.md
  guest/
    .cargo/config.toml
    Cargo.toml
    rust-toolchain.toml
    src/main.rs
  host/
    Cargo.toml
    rust-toolchain.toml
    src/main.rs
```

## `cargo airbender run`

Runs `app.bin` in simulator mode.

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

Options:

- `--input <file>` (required)
- `--cycles <n>` (optional cycle limit)

## `cargo airbender flamegraph`

Runs simulator with profiling and writes flamegraph output.

```sh
cargo airbender flamegraph ./dist/app/app.bin --input ./input.hex --output flamegraph.svg
```

Options include:

- `--sampling-rate <n>`
- `--inverse`
- `--elf-path <file>` (optional custom symbol source)

## `cargo airbender run-transpiler`

Runs `app.bin` via transpiler JIT.

```sh
cargo airbender run-transpiler ./dist/app/app.bin --input ./input.hex
```

Options:

- `--cycles <n>`
- `--text-path <file>`

## `cargo airbender prove`

Generates a bincode-encoded proof.

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output proof.bin
```

Key options:

- `--backend <dev|cpu|gpu>` (default: `dev`)
- `--threads <n>`
- `--cycles <n>`
- `--ram-bound <bytes>`
- `--level <base|recursion-unrolled|recursion-unified>` (default: `recursion-unified`)

Notes:

- `dev` backend runs transpiler execution and emits a dev proof envelope.
- `gpu` backend requires enabling the `airbender-host/gpu-prover` feature.
- `verify-proof` accepts only real proofs, so use `--backend cpu` or `--backend gpu` when preparing proofs for CLI verification.

## `cargo airbender generate-vk`

Generates verification keys and writes them as bincode.

```sh
cargo airbender generate-vk ./dist/app/app.bin --output vk.bin
```

Options:

- `--output <file>` (default: `vk.bin`)
- `--level <base|recursion-unrolled|recursion-unified>`

Notes:

- `generate-vk` requires GPU-enabled `cargo-airbender` (`--features gpu-prover`).
- Without that feature, the command fails before VK computation.
- Local install example: `cargo install --path crates/cargo-airbender --features gpu-prover --force`.

## `cargo airbender verify-proof`

Verifies a real proof against a real verification key file.

```sh
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

Options:

- `--vk <file>` (required)
- `--expected-output <words>` (optional): comma-separated public output words for `x10..x17` (decimal or `0x` hex)

Notes:

- dev proofs are rejected by this command with a dedicated error message.
- if `--expected-output` is omitted, CLI prints a warning and verifies proof/VK validity only.
- when `--expected-output` has fewer than 8 words, remaining words are zero-padded.

Examples:

```sh
cargo airbender verify-proof ./proof.bin --vk ./vk.bin --expected-output 42
cargo airbender verify-proof ./proof.bin --vk ./vk.bin --expected-output 42,0,0,0
cargo airbender verify-proof ./proof.bin --vk ./vk.bin --expected-output 0x2a
```

## Input File Format (`--input`)

Runtime/prover commands that accept `--input` expect hex-encoded `u32` words:

- optional `0x` prefix is allowed
- whitespace is ignored
- total hex length must be a multiple of 8
- each 8-hex chunk is parsed as one `u32`
- words must match guest input expectations (for `read::<T>()`, this means codec-framed payload words)

Recommended: construct inputs with `airbender_host::Inputs` (`push`, `push_bytes`) and write files with `write_hex_file(...)`. See [`docs/02-host-program-api.md`](./02-host-program-api.md).

Example file:

```text
00000001
29000000
```

## Logging

Set `RUST_LOG` to control verbosity:

```sh
RUST_LOG=debug cargo airbender prove ./dist/app/app.bin --input ./input.hex --output proof.bin
```
