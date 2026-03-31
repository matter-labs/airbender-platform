# CLI Reference

All commands are invoked as `cargo airbender <command>`.

```text
build          Build guest artifacts
new            Scaffold a host+guest project
run            Execute a guest binary
flamegraph     Profile guest execution
prove          Generate a proof
generate-vk    Generate verification keys
verify-proof   Verify a proof
clean          Remove Docker build resources
```

---

## `build`

Compiles guest code and packages artifacts into a dist directory.

```sh
cargo airbender build
```

The command auto-discovers the nearest guest `Cargo.toml` from the current directory. Use `--project` to specify it explicitly.

| Option | Description |
|--------|-------------|
| `--app-name <name>` | Output folder name under dist (default: `app`) |
| `--bin <name>` | Explicit Cargo binary target |
| `--target <triple>` | Override target triple |
| `--dist <path>` | Override dist root directory |
| `--project <path>` | Guest project directory |
| `--profile <debug\|release>` | Build profile (or use `--debug` / `--release`) |
| `--reproducible` | Deterministic build via pinned Docker container |
| `--workspace-root <path>` | Mount root for `--reproducible` (see below) |

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
cargo airbender build -- --features my_extra_feature
```

### Reproducible builds

`--reproducible` compiles inside a pinned Docker image (`debian:bullseye-slim`, fixed nightly toolchain). Two builds of the same source on any machine produce identical artifacts and SHA-256 hashes. Requires Docker.

### Monorepo path dependencies

If your guest has `path = "../../..."` dependencies pointing outside its cargo workspace root, the Docker container won't see them. Pass `--workspace-root` to widen the mount:

```sh
cargo airbender build --reproducible --workspace-root . --project examples/fibonacci/guest
```

End users depending on published crates (crates.io or git) don't need this.

**Cargo.lock note:** the guest must have a `Cargo.lock` generated with the same nightly toolchain used inside the container. Regenerate if needed:

```sh
cargo +nightly-2026-02-10 generate-lockfile --manifest-path <guest>/Cargo.toml
```

### Output layout

```text
dist/<app-name>/app.bin
dist/<app-name>/app.elf
dist/<app-name>/app.text
dist/<app-name>/manifest.toml
```

---

## `new`

Scaffolds a host+guest project.

```sh
cargo airbender new [path]
```

Runs interactively by default, asking for project name, `std` support, allocator, and prover backend. Pass `--yes` to skip prompts.

| Option | Description |
|--------|-------------|
| `--name <name>` | Project name |
| `--enable-std` | Enable std in the guest |
| `--allocator <talc\|bump\|custom>` | Allocator selection |
| `--prover-backend <dev\|gpu>` | Default prover backend |
| `--yes` | Non-interactive mode |
| `--sdk-path <path>` | Local SDK path |
| `--sdk-version <version>` | Published SDK version |

Prover backends:

- **`dev`** - mock proof envelope, no GPU needed. Use for development.
- **`gpu`** - real proving, requires NVIDIA GPU at runtime. Compile with `ZKSYNC_USE_CUDA_STUBS=true` if you don't have CUDA locally.

When `custom` allocator is selected, the guest includes an `allocator_init` hook and a sample allocator module you can replace.

---

## `run`

Executes a guest binary via the transpiler.

```sh
cargo airbender run ./dist/app/app.bin --input ./input.hex
```

| Option | Description |
|--------|-------------|
| `--input <file>` | Input file (required) |
| `--cycles <n>` | Cycle limit |
| `--text-path <file>` | Path to `.text` section (default: sibling of app.bin) |
| `--jit` | Enable transpiler JIT (x86_64 only) |

---

## `flamegraph`

Profiles guest execution and writes a flamegraph SVG.

```sh
cargo airbender flamegraph ./dist/app/app.bin --input ./input.hex --output flamegraph.svg
```

| Option | Description |
|--------|-------------|
| `--input <file>` | Input file (required) |
| `--output <file>` | Output SVG path (default: `flamegraph.svg`) |
| `--cycles <n>` | Cycle limit |
| `--sampling-rate <n>` | Sampling rate |
| `--inverse` | Inverse flamegraph |
| `--elf-path <file>` | Custom symbol source |

---

## `prove`

Generates a proof.

```sh
cargo airbender prove ./dist/app/app.bin --input ./input.hex --output proof.bin
```

| Option | Description |
|--------|-------------|
| `--backend <dev\|cpu\|gpu>` | Prover backend (default: `dev`) |
| `--level <base\|recursion-unrolled\|recursion-unified>` | Prover level (default: `recursion-unified`) |
| `--threads <n>` | Worker threads |
| `--output <file>` | Output proof file (required) |
| `--cycles <n>` | Cycle limit (dev and CPU backends) |
| `--ram-bound <bytes>` | RAM bound (CPU only) |

**Important:** `verify-proof` only accepts real proofs (CPU/GPU). Dev proofs are rejected with a clear error message.

The `cpu` backend is for debugging circuits. It can only prove the base layer and is slow. Use `gpu` for real end-to-end proving.

---

## `generate-vk`

Generates verification keys. Requires GPU support in `cargo-airbender` (enabled by default).

```sh
cargo airbender generate-vk ./dist/app/app.bin --output vk.bin
```

| Option | Description |
|--------|-------------|
| `--output <file>` | Output path (default: `vk.bin`) |
| `--level <base\|recursion-unrolled\|recursion-unified>` | VK level |

---

## `verify-proof`

Verifies a real proof against a verification key.

```sh
cargo airbender verify-proof ./proof.bin --vk ./vk.bin
```

| Option | Description |
|--------|-------------|
| `--vk <file>` | Verification key file (required) |
| `--expected-output <words>` | Expected public output (comma-separated, decimal or `0x` hex) |

When `--expected-output` is omitted, only proof/VK validity is checked (with a warning). Fewer than 8 words are zero-padded.

```sh
cargo airbender verify-proof ./proof.bin --vk ./vk.bin --expected-output 42
cargo airbender verify-proof ./proof.bin --vk ./vk.bin --expected-output 0x2a
```

---

## `clean`

Removes Docker resources from reproducible builds.

```sh
cargo airbender clean
```

Deletes the shared `airbender-cargo-registry` volume and any orphaned `airbender-build` containers. Only needed to reclaim disk space; containers are normally cleaned up automatically.

---

## Input File Format

Commands that accept `--input` expect hex-encoded `u32` words:

- Optional `0x` prefix
- Whitespace is ignored
- Total hex length must be a multiple of 8
- Each 8-hex chunk is one `u32`

Example file:

```text
00000001
29000000
```

Best practice: use `Inputs::push(...)` and `write_hex_file(...)` from the host to generate these files. See [Host Program API](./02-host-program-api.md).

## Logging

```sh
RUST_LOG=debug cargo airbender prove ./dist/app/app.bin --input ./input.hex --output proof.bin
```
