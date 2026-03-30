# CLI Reference

`cargo airbender` is the main user CLI for project scaffolding, guest builds, execution, proving, and verification.

## Top-Level Commands

```text
build
new
run
flamegraph
prove
generate-vk
verify-proof
clean
```

## `cargo airbender build`

Builds guest artifacts into a dist app directory.

```sh
cargo airbender build --app-name app
```

When `--project` is omitted, the command searches the current directory and its parents for the nearest guest `Cargo.toml`.

Key options:

- `--app-name <name>`: output namespace under dist root (default: `app`)
- `--bin <name>`: explicit Cargo binary target
- `--target <triple>`: explicit target triple override (otherwise Cargo config defaults are used)
- `--dist <path>`: dist root directory (app folder is created under this root; relative paths are resolved from command invocation cwd)
- `--project <path>`: guest project directory
- `--profile <debug|release>`, `--debug`, `--release`
- `--reproducible`: build inside a pinned Docker container for bit-for-bit identical output across machines and toolchain versions; automatically passes `--locked` to cargo
- `--workspace-root <path>`: override the directory bind-mounted as `/src` inside the container; only needed with `--reproducible` when the guest has path dependencies pointing outside its own cargo workspace root (see [Monorepo path dependencies](#monorepo-path-dependencies) below); requires `--reproducible`

### `panic-immediate-abort`

Replaces all panic call sites with an immediate trap instruction, eliminating panic formatting and unwinding infrastructure and significantly reducing binary size. Enable per-profile in the guest `Cargo.toml`:

```toml
[package.metadata]
airbender.profile.release = { panic-immediate-abort = true }
airbender.profile.debug   = { panic-immediate-abort = true }
```

Supported profile keys are `"release"` and `"debug"`.

Forward extra Cargo flags after `--`:

```sh
cargo airbender build --app-name with_extra_feature -- --features my_extra_feature
```

Reproducible build:

```sh
cargo airbender build --reproducible
```

This bind-mounts the workspace read-only into a temporary Docker container, compiles the guest inside a pinned image (`debian:bullseye-slim` at a fixed digest, Rust nightly pinned to the same date as `DEFAULT_GUEST_TOOLCHAIN`), copies the artifacts back to the host with `docker cp`, and removes the container. Two builds of the same source on any machine will produce identical `app.bin`/`app.elf`/`app.text` bytes and identical SHA-256 hashes in `manifest.toml`. Requires Docker.

### Monorepo path dependencies

The `--reproducible` flag bind-mounts the guest project's cargo workspace root as `/src` inside the container. For most projects this is the guest directory itself. If the guest's `Cargo.toml` contains `path = "../../.."` dependencies pointing to crates outside that directory (typical in platform monorepos where the guest shares crates with the host via local paths), those crates will not be present inside the container and the build will fail.

Pass `--workspace-root` to expand the mount to a directory that contains all referenced crates:

```sh
cargo airbender build --reproducible --workspace-root . --project examples/fibonacci/guest
```

This is a developer-only scenario. End users whose guest depends on published crates (crates.io or git) never need this flag.

**Cargo.lock prerequisite:** the guest project must have a `Cargo.lock` committed and generated with the same nightly toolchain used inside the container. If your lock file was created with a different toolchain, regenerate it once before the first reproducible build:

```sh
rustup toolchain install nightly-2026-02-10
cargo +nightly-2026-02-10 generate-lockfile --manifest-path <guest>/Cargo.toml
git add <guest>/Cargo.lock && git commit -m "chore: regenerate Cargo.lock for reproducible builds"
```

When `--reproducible` is used, `manifest.toml` records:

```toml
target = "riscv32im-risc0-zkvm-elf"

[build]
profile = "release"
reproducible = true
...
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

By default, this command runs in interactive mode and asks:

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
  `https://github.com/matter-labs/airbender-platform` (branch `main`)

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

Runs `app.bin` via transpiler execution.

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

Options:

- `--input <file>` (required)
- `--cycles <n>` (optional cycle limit)
- `--text-path <file>` (optional path to `.text` file; defaults to sibling of `app.bin`)
- `--jit`: enable transpiler JIT on x86_64 (without this flag, transpiler runs in non-JIT mode)

## `cargo airbender flamegraph`

Runs transpiler execution with profiling and writes flamegraph output.

```sh
cargo airbender flamegraph ./dist/app/app.bin --input ./input.hex --output flamegraph.svg
```

Options include:

- `--sampling-rate <n>`
- `--inverse`
- `--elf-path <file>` (optional custom symbol source)

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
- `cpu` backend can only generate proofs for the base layer, and is not meant to be used outside of the debugging of the airbender itself.
- `gpu` backend requires GPU support in `cargo-airbender` (enabled by default).
- `--cycles` and `--ram-bound` are ignored on `gpu`/`dev` backends.
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

- `generate-vk` requires GPU support in `cargo-airbender` (enabled by default).
- If you installed with `--no-default-features`, the command fails before VK computation.
- Local install example with GPU support disabled: `cargo install --path crates/cargo-airbender --no-default-features --force`.

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

## `cargo airbender clean`

Removes Docker resources created by reproducible builds to reclaim disk space.

```sh
cargo airbender clean
```

Removes the shared `airbender-cargo-registry` volume and any stopped `airbender-build`
containers left by interrupted builds.

Notes:

- Each build run uses an isolated temporary container that is removed automatically on
  success or failure. `clean` is only needed to reclaim the crate download cache or remove
  containers orphaned by a hard kill (`SIGKILL`/OOM).
- After `cargo airbender clean`, the next `--reproducible` build re-downloads all crate
  sources from crates.io before compiling.

## Input File Format (`--input`)

Runtime/prover commands that accept `--input` expect hex-encoded `u32` words:

- optional `0x` prefix is allowed
- whitespace is ignored
- total hex length must be a multiple of 8
- each 8-hex chunk is parsed as one `u32`
- words must match guest input expectations (for `read::<T>()`, this means Airbender input wire-framed payload words)

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
