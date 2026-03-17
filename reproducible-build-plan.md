# Plan: `cargo airbender build --reproducible`

## Context

Guest programs compiled by `cargo airbender build` are RISC-V binaries whose bytes feed into zkVM proofs. The sha256 of those bytes is embedded in the manifest and in verification keys (VKs). A non-reproducible build means two developers building the same source get different hashes, breaking independent VK verification and auditability of the proof system.

The fix is to route compilation inside a pinned Docker container — fixed OS image digest + fixed Rust nightly date + fixed RISC-V target — so the same source always produces the same `app.bin`/`app.elf`/`app.text` bytes. The reference implementation for the verifier binaries lives at `zksync-airbender/tools/reproduce/`.

## Files to Modify

| File | Change |
|---|---|
| `crates/airbender-core/src/manifest.rs` | Add `reproducible: bool` to `BuildMetadata` |
| `crates/airbender-build/src/errors.rs` | Add `DockerNotFound`, `DockerNotRunning`, `DockerBuildFailed` variants |
| `crates/airbender-build/src/config.rs` | Add `reproducible: bool` to `BuildConfig`; branch in `build_dist()` |
| `crates/airbender-build/src/lib.rs` | Add `mod docker;` |
| `crates/cargo-airbender/src/cli.rs` | Add `#[arg(long)] pub reproducible: bool` to `BuildArgs` |
| `crates/cargo-airbender/src/commands/build.rs` | Destructure and forward `reproducible` into `BuildConfig` |

## New Files

| File | Purpose |
|---|---|
| `crates/airbender-build/docker/Dockerfile.template` | Dockerfile template with `{{TOOLCHAIN_DATE}}` placeholder |
| `crates/airbender-build/src/docker.rs` | Docker orchestration module |

---

## Toolchain / Dockerfile Sync Strategy

`DEFAULT_GUEST_TOOLCHAIN` in `crates/airbender-build/src/constants.rs` is the single source of truth for the Rust nightly date. The Dockerfile **must never hardcode** the date — it instead uses a placeholder substituted at runtime.

### Template file

```dockerfile
# crates/airbender-build/docker/Dockerfile.template
#
# DO NOT hardcode a toolchain date here.
# {{TOOLCHAIN_DATE}} is substituted at runtime from DEFAULT_GUEST_TOOLCHAIN in constants.rs.
# To update the toolchain, change DEFAULT_GUEST_TOOLCHAIN in constants.rs only.
#
# Base image pinned by digest for reproducibility. Digest sourced from:
#   zksync-airbender/tools/reproduce/Dockerfile (same image works across toolchain dates
#   because the compiler is installed via rustup, not from apt packages).
# To update: docker pull debian:bullseye-slim && docker inspect --format='{{index .RepoDigests 0}}'
FROM debian:bullseye-slim@sha256:f527627d07c18abf87313c341ee8ef1b36f106baa8b6b6dc33f4c872d988b651

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl build-essential git libssl-dev pkg-config ca-certificates && \
    rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --no-modify-path --default-toolchain {{TOOLCHAIN_DATE}}

RUN rustup component add llvm-tools-preview rust-src && \
    rustup target add riscv32im-risc0-zkvm-elf && \
    cargo install cargo-binutils --locked

WORKDIR /build
```

### Substitution in `docker.rs`

```rust
const DOCKERFILE_TEMPLATE: &str = include_str!("../docker/Dockerfile.template");

fn dockerfile_contents() -> String {
    DOCKERFILE_TEMPLATE.replace("{{TOOLCHAIN_DATE}}", DEFAULT_GUEST_TOOLCHAIN)
}
```

This means:
- Updating `DEFAULT_GUEST_TOOLCHAIN = "nightly-2026-02-10"` in `constants.rs` is the **only change** needed to update the toolchain
- The image tag `airbender-build:nightly-2026-02-10` is also derived from `DEFAULT_GUEST_TOOLCHAIN` — a toolchain change automatically produces a new image tag and triggers a fresh `docker build`
- No manual editing of two files; no risk of divergence
- The base image digest stays fixed until explicitly updated (Debian version has no effect on compiler output since `rustup` owns the toolchain)

### Image digest update procedure

When the base OS digest needs to be rotated (e.g. for a Debian security patch):
1. Run `docker pull debian:bullseye-slim`
2. Run `docker inspect --format='{{index .RepoDigests 0}}' debian:bullseye-slim`
3. Update the `FROM` line digest in `Dockerfile.template`
4. Commit — the PR diff makes the base image change explicit and reviewable

---

## Step 1 — `manifest.rs`: add `reproducible` to `BuildMetadata`

Same pattern as `is_dirty`: `#[serde(default, skip_serializing_if = "is_false")]`.
Old manifests without the field deserialize with `reproducible = false` (backward compatible).

```rust
// crates/airbender-core/src/manifest.rs
pub struct BuildMetadata {
    pub profile: Profile,
    pub git_branch: String,
    pub git_commit: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_dirty: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub reproducible: bool,   // new
}
```

Add two tests:
- `reproducible_field_omitted_when_false` — serialize with `false`, assert key absent
- `reproducible_field_present_when_true` — serialize with `true`, assert `reproducible = true`

---

## Step 2 — `errors.rs`: add Docker error variants

```rust
// crates/airbender-build/src/errors.rs
#[error("docker not found: install Docker and ensure it is on PATH")]
DockerNotFound,

#[error("docker daemon is not running: start Docker Desktop or the docker service")]
DockerNotRunning,

#[error("failed to build Docker image for reproducible build")]
DockerBuildFailed,
```

`DockerNotFound` vs `DockerNotRunning` are distinct so the user gets an actionable message in both cases.

---

## Step 3 — `docker/Dockerfile.template`

Create the template file as shown in the sync strategy section above. Key properties:
- Base image: `debian:bullseye-slim@sha256:f527627d07c18abf87313c341ee8ef1b36f106baa8b6b6dc33f4c872d988b651` (same digest as `zksync-airbender/tools/reproduce/Dockerfile`)
- Toolchain: `{{TOOLCHAIN_DATE}}` placeholder — **never hardcoded**
- Target: `riscv32im-risc0-zkvm-elf` (matches `DEFAULT_GUEST_TARGET`)
- No `COPY` — source is always volume-mounted at runtime

---

## Step 4 — `docker.rs`: orchestration module

### Public API

```rust
// crates/airbender-build/src/docker.rs
pub(crate) fn run_reproducible_build(
    project_dir: &Path,
    bin_name: &str,
    target: Option<&str>,
    profile: Profile,
    dist_dir: &Path,
    cargo_args: &[String],
) -> Result<()>
```

### Internal helpers (in call order)

**`ensure_docker_available()`**

```rust
fn ensure_docker_available() -> Result<()> {
    let result = Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .status();
    match result {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err(BuildError::DockerNotRunning),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(BuildError::DockerNotFound),
        Err(e) => Err(BuildError::Io(e)),
    }
}
```

**`docker_image_tag()`**

```rust
fn docker_image_tag() -> String {
    format!("airbender-build:{}", DEFAULT_GUEST_TOOLCHAIN)
}
```

**`ensure_image_built()`**

```rust
fn ensure_image_built() -> Result<()> {
    let tag = docker_image_tag();

    // Skip build if image with this tag already exists (Docker layer cache)
    let exists = Command::new("docker")
        .args(["image", "inspect", &tag])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .status()?.success();
    if exists { return Ok(()); }

    // Substitute toolchain date into template and write to tempdir
    let dockerfile = dockerfile_contents();
    let dockerfile_path = std::env::temp_dir()
        .join(format!("airbender-{}.dockerfile", DEFAULT_GUEST_TOOLCHAIN));
    std::fs::write(&dockerfile_path, dockerfile)?;

    // Empty context dir — no COPY directives in the Dockerfile
    let ctx_dir = std::env::temp_dir().join("airbender-docker-ctx");
    std::fs::create_dir_all(&ctx_dir)?;

    let status = Command::new("docker")
        .args(["build", "--platform", "linux/amd64",
               "-t", &tag,
               "-f", dockerfile_path.to_str().unwrap(),
               ctx_dir.to_str().unwrap()])
        .status()?;

    if !status.success() {
        return Err(BuildError::DockerBuildFailed);
    }
    Ok(())
}
```

**`docker_run_cargo()` — no shell injection**

Each of the four cargo invocations (1x build + 3x objcopy) gets its own discrete `docker run` call using `cmd.arg()` per token — no `/bin/bash -c` string, no escaping needed.

```rust
fn docker_run_cargo(
    tag: &str,
    project_dir: &Path,
    dist_dir: &Path,
    target_volume: &str,
    subcmd: &str,
    fixed_args: &[&str],
    user_args: &[String],
    objcopy_args: &[&str],
) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.args(["run", "--rm", "--platform", "linux/amd64", "--workdir", "/src"]);
    cmd.args(["-v", &format!("{}:/src:ro", project_dir.display())]);
    cmd.args(["-v", &format!("{}:/out:rw", dist_dir.display())]);
    cmd.args(["-v", "airbender-cargo-registry:/usr/local/cargo/registry"]);
    cmd.args(["-v", &format!("{target_volume}:/src/target")]);
    cmd.arg(tag);
    cmd.args(["cargo", subcmd]);
    cmd.args(fixed_args);
    cmd.args(user_args);
    if !objcopy_args.is_empty() {
        cmd.arg("--");
        cmd.args(objcopy_args);
    }
    run_command(cmd, &format!("docker run cargo {subcmd}"))
    // run_command is the existing utility in utils.rs
}
```

**Volume mounts:**
- `/src` — project source, read-only
- `/out` — dist_dir, read-write (artifacts written here by objcopy)
- `airbender-cargo-registry` — shared Docker named volume for crate downloads (persists across builds)
- `<target_volume>` — per-project Docker named volume for incremental build cache; name = `airbender-cargo-target-<first-8-bytes-of-sha256(project_dir-canonical-path)>` using the `sha2` crate already in `airbender-build/Cargo.toml`

**Fixed args per call (all include `--locked`):**

| Invocation | fixed_args | objcopy_args |
|---|---|---|
| cargo build | `[profile_flag, "--bin", bin, "--target", target, "--locked"]` | _(none)_ |
| objcopy → app.bin | same | `["-O", "binary", "/out/app.bin"]` |
| objcopy → app.elf | same | `["-R", ".text", "/out/app.elf"]` |
| objcopy → app.text | same | `["-O", "binary", "--only-section=.text", "/out/app.text"]` |

`--locked` is always injected. The project must have a `Cargo.lock` (cargo will error clearly if absent). This is required for reproducibility — without it, dependency versions could differ between runs.

### `run_reproducible_build()` — main entry

```rust
pub(crate) fn run_reproducible_build(
    project_dir: &Path, bin_name: &str, target: Option<&str>,
    profile: Profile, dist_dir: &Path, cargo_args: &[String],
) -> Result<()> {
    ensure_docker_available()?;
    ensure_image_built()?;

    let tag = docker_image_tag();
    let target_str = target.unwrap_or(DEFAULT_GUEST_TARGET);
    let profile_flag = if profile == Profile::Release { "--release" } else { "" };

    // Per-project named volume for target/ cache
    let project_key = {
        use sha2::{Digest, Sha256};
        let canonical = project_dir.canonicalize().unwrap_or_else(|_| project_dir.to_path_buf());
        let hash = Sha256::digest(canonical.to_string_lossy().as_bytes());
        // hex encode without a hex crate: format each byte inline
        hash[..8].iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let target_volume = format!("airbender-cargo-target-{project_key}");

    let fixed = &[profile_flag, "--bin", bin_name, "--target", target_str, "--locked"];

    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "build", fixed, cargo_args, &[])?;
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "objcopy", fixed, cargo_args, &["-O", "binary", "/out/app.bin"])?;
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "objcopy", fixed, cargo_args, &["-R", ".text", "/out/app.elf"])?;
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "objcopy", fixed, cargo_args, &["-O", "binary", "--only-section=.text", "/out/app.text"])?;
    Ok(())
}
```

---

## Step 5 — `config.rs`: add field and branch

**Add to `BuildConfig`:**

```rust
pub struct BuildConfig {
    // ... existing fields ...
    /// Run compilation inside a pinned Docker container for bit-for-bit reproducibility.
    pub reproducible: bool,
}
```

Initialize to `false` in `BuildConfig::new()`.

**Branch in `build_dist()` (around lines 58–84 today):**

```rust
if self.reproducible {
    docker::run_reproducible_build(
        &project_dir, &manifest_names.bin_name,
        target.as_deref(), self.profile, &dist_dir, &self.cargo_args,
    )?;
} else {
    self.run_cargo_build(&project_dir, &manifest_names.bin_name, target.as_deref())?;
    self.run_cargo_objcopy(..., &["-O", "binary"], &app_bin)?;
    self.run_cargo_objcopy(..., &["-R", ".text"], &app_elf)?;
    self.run_cargo_objcopy(..., &["-O", "binary", "--only-section=.text"], &app_text)?;
}
```

Everything after (sha256 hashing, manifest writing) runs on both paths unchanged.

**Thread into `BuildMetadata`:**

```rust
build: BuildMetadata {
    profile: self.profile,
    git_branch: git_metadata.branch,
    git_commit: git_metadata.commit,
    is_dirty: git_metadata.is_dirty,
    reproducible: self.reproducible,   // new
},
```

---

## Step 6 — `cli.rs`: add flag

```rust
// In BuildArgs
#[arg(long)]
pub reproducible: bool,
```

Add test `parse_build_reproducible_flag`.

---

## Step 7 — `commands/build.rs`: thread through

Destructure `reproducible` from `BuildArgs`, set `config.reproducible = reproducible`.
Add to success output:

```rust
if reproducible {
    ui::info("reproducible build (Docker)");
    ui::field("toolchain", airbender_build::DEFAULT_GUEST_TOOLCHAIN);
}
```

---

## Step 8 — `lib.rs`

```rust
mod docker;   // pub(crate) — not part of external API
```

---

## Dependency changes

No new workspace dependencies:
- `sha2` already in `airbender-build/Cargo.toml` — used for target volume name
- Hex encoding done inline (`format!("{b:02x}")` per byte)
- No `tempfile` needed — Dockerfile written to `std::env::temp_dir()` with stable name

---

## Implementation order

1. `manifest.rs` — data-only, all existing tests still pass
2. `errors.rs` — additive, no breakage
3. `docker/Dockerfile.template` — new file with `{{TOOLCHAIN_DATE}}` placeholder
4. `docker.rs` — new module
5. `lib.rs` — add `mod docker;`
6. `config.rs` — add field + branch + thread into metadata
7. `cli.rs` — add flag + test
8. `commands/build.rs` — thread through + success output

---

## Verification

**Unit tests (no Docker):**
- `manifest.rs`: `reproducible_field_omitted_when_false`, `reproducible_field_present_when_true`
- `config.rs`: `reproducible_flag_defaults_to_false`
- `cli.rs`: `parse_build_reproducible_flag`

**Manual end-to-end (requires Docker):**
```bash
cargo airbender build --project examples/fibonacci/guest --reproducible
sha256sum examples/fibonacci/guest/dist/app/app.bin > first.txt

cargo airbender build --project examples/fibonacci/guest --reproducible
sha256sum examples/fibonacci/guest/dist/app/app.bin > second.txt

diff first.txt second.txt   # must be empty
grep reproducible examples/fibonacci/guest/dist/app/manifest.toml
# reproducible = true
```

**Cross-machine test:** run on two machines with different host toolchains — hashes must match.

**Toolchain sync check:** optionally add a `#[test]` that parses `Dockerfile.template` and asserts it contains `{{TOOLCHAIN_DATE}}` (not a hardcoded date), guarding against accidental hardcoding during future edits.
