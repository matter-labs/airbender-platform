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
- `--target <triple>`: explicit target triple
- `--dist <path>`: dist root directory (app folder is created under this root)
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
cargo airbender new ./my-guest
```

Options:

- `--name <name>`: override package name
- `--enable-std`: generate guest with `airbender-sdk` `std` feature
- `--sdk-path <path>`: use local SDK path (workspace root, `crates/`, or crate path)
- `--sdk-version <version>`: use versioned SDK dependency

Default behavior (when neither `--sdk-path` nor `--sdk-version` is provided):

- generated project depends on `airbender-sdk` from
  `https://github.com/popzxc/airbender-platform` (branch `main`)

Generated layout:

```text
<project>/
  README.md
  guest/
    Cargo.toml
    src/main.rs
  host/
    Cargo.toml
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

- `--backend <cpu|gpu>` (default: `gpu`)
- `--threads <n>`
- `--cycles <n>`
- `--ram-bound <bytes>`
- `--level <base|recursion-unrolled|recursion-unified>` (default: `recursion-unified`)

## `cargo airbender generate-vk`

Generates verification keys and writes them as bincode.

```sh
cargo airbender generate-vk ./dist/app/app.bin --output vk.bin
```

Options:

- `--output <file>` (default: `vk.bin`)
- `--level <base|recursion-unrolled|recursion-unified>`

## `cargo airbender verify-proof`

Verifies a proof against a verification key file.

```sh
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

Options:

- `--vk <file>` (required)
- `--level <base|recursion-unrolled|recursion-unified>`

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
