//! Docker-based reproducible build support.
//!
//! Runs `cargo build` and `cargo objcopy` inside a pinned container so the
//! same source always produces bit-for-bit identical artifacts regardless of
//! the host toolchain or OS environment.

use crate::constants::{DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
use crate::errors::{BuildError, Result};
use crate::utils::run_command;
use airbender_core::host::manifest::Profile;
use sha2::{Digest, Sha256};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Dockerfile template embedded at compile time.
/// `{{TOOLCHAIN_DATE}}` is replaced at runtime with `DEFAULT_GUEST_TOOLCHAIN`.
const DOCKERFILE_TEMPLATE: &str = include_str!("../docker/Dockerfile.template");

/// Returns the Dockerfile contents with the toolchain date substituted in.
fn dockerfile_contents() -> String {
    DOCKERFILE_TEMPLATE.replace("{{TOOLCHAIN_DATE}}", DEFAULT_GUEST_TOOLCHAIN)
}

/// Returns the Docker image tag for the current toolchain, e.g. `airbender-build:nightly-2026-02-10`.
/// Changing `DEFAULT_GUEST_TOOLCHAIN` automatically produces a new tag and triggers a fresh build.
fn docker_image_tag() -> String {
    format!("airbender-build:{}", DEFAULT_GUEST_TOOLCHAIN)
}

/// Checks that Docker is installed and the daemon is reachable.
fn ensure_docker_available() -> Result<()> {
    let result = Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match result {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err(BuildError::DockerNotRunning),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(BuildError::DockerNotFound),
        Err(e) => Err(BuildError::Io(e)),
    }
}

/// Builds the Docker image if it does not already exist for the current toolchain tag.
/// On subsequent calls the image is found by tag and the build is skipped entirely.
fn ensure_image_built() -> Result<()> {
    let tag = docker_image_tag();

    let exists = Command::new("docker")
        .args(["image", "inspect", &tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success();
    if exists {
        return Ok(());
    }

    // Write the substituted Dockerfile to a stable temp path.
    let dockerfile_path = std::env::temp_dir()
        .join(format!("airbender-{}.dockerfile", DEFAULT_GUEST_TOOLCHAIN));
    std::fs::write(&dockerfile_path, dockerfile_contents())?;

    // Use an empty directory as the build context — the Dockerfile has no COPY directives.
    let ctx_dir = std::env::temp_dir().join("airbender-docker-ctx");
    std::fs::create_dir_all(&ctx_dir)?;

    let mut cmd = Command::new("docker");
    cmd.args([
        "build",
        "--platform",
        "linux/amd64",
        "-t",
        &tag,
        "-f",
        dockerfile_path.to_str().unwrap(),
        ctx_dir.to_str().unwrap(),
    ]);
    run_command(cmd, "docker build")
}

/// Runs a single `cargo <subcmd>` invocation inside the pinned container.
///
/// Volume mounts:
/// - `/src`  — project source directory, read-only
/// - `/out`  — dist output directory, read-write (objcopy writes artifacts here)
/// - `airbender-cargo-registry` — shared named volume for downloaded crate sources
/// - `<target_volume>` — per-project named volume for incremental build cache
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
}

/// Compiles a guest program and produces `app.bin`, `app.elf`, and `app.text` inside `dist_dir`,
/// using a pinned Docker container for bit-for-bit reproducible output.
///
/// `--locked` is always passed so that `Cargo.lock` pins dependency versions exactly.
/// The project must have a committed `Cargo.lock`; cargo will error clearly if it is absent.
pub(crate) fn run_reproducible_build(
    project_dir: &Path,
    bin_name: &str,
    target: Option<&str>,
    profile: Profile,
    dist_dir: &Path,
    cargo_args: &[String],
) -> Result<()> {
    ensure_docker_available()?;
    ensure_image_built()?;

    let tag = docker_image_tag();
    let target_str = target.unwrap_or(DEFAULT_GUEST_TARGET);
    let profile_flag = if profile == Profile::Release {
        "--release"
    } else {
        ""
    };

    // Stable per-project volume name derived from the canonical project path.
    // Uses the first 8 bytes of SHA-256 to keep the name short and Docker-safe.
    let project_key = {
        let canonical = project_dir
            .canonicalize()
            .unwrap_or_else(|_| project_dir.to_path_buf());
        let hash = Sha256::digest(canonical.to_string_lossy().as_bytes());
        hash[..8].iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let target_volume = format!("airbender-cargo-target-{project_key}");

    // Build args shared across all four invocations. --locked is always included.
    let fixed: &[&str] = if profile_flag.is_empty() {
        &["--bin", bin_name, "--target", target_str, "--locked"]
    } else {
        &[profile_flag, "--bin", bin_name, "--target", target_str, "--locked"]
    };

    // 1. Compile.
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "build", fixed, cargo_args, &[])?;

    // 2–4. Extract binary artifacts.
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "objcopy", fixed, cargo_args, &["-O", "binary", "/out/app.bin"])?;
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "objcopy", fixed, cargo_args, &["-R", ".text", "/out/app.elf"])?;
    docker_run_cargo(&tag, project_dir, dist_dir, &target_volume,
        "objcopy", fixed, cargo_args, &["-O", "binary", "--only-section=.text", "/out/app.text"])?;

    Ok(())
}
