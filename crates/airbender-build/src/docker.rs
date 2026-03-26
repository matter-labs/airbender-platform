//! Docker-based reproducible build support.
//!
//! Runs `cargo build` and `cargo objcopy` inside a pinned container so the
//! same source always produces bit-for-bit identical artifacts regardless of
//! the host toolchain or OS environment.
//!
//! # Build strategy
//!
//! ```text
//! docker run -v workspace:/src:ro  →  docker cp <artifacts out>  →  docker rm
//! ```
//!
//! Source is bind-mounted read-only (no host writes). Artifacts are copied out
//! with `docker cp`, which writes files as the host user — no root-owned files
//! ever land on the host filesystem. The container is always removed on return,
//! whether by success, error, or panic.
//!
//! # Volume strategy
//!
//! | Volume | Scope | Lifetime |
//! |---|---|---|
//! | `airbender-cargo-registry` | shared across all projects | persistent (crate download cache) |
//!
//! The `/cargo-target` and `/dist` directories live in the container's writable
//! layer and are discarded when the container is removed at end of build.
//!
//! # Image tag
//!
//! The image tag is `airbender-build:<toolchain>` where `<toolchain>` is
//! `DEFAULT_GUEST_TOOLCHAIN`. To update the toolchain or rotate the base image
//! digest, change `DEFAULT_GUEST_TOOLCHAIN` in `constants.rs`; the new tag
//! forces a fresh `docker build`.
//!
//! # Cleanup
//!
//! Use [`clean_reproducible_volumes`] (exposed as `cargo airbender clean`) to remove
//! the shared registry cache and any stopped `airbender-build` containers left by
//! interrupted builds.

use crate::constants::{DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
use crate::errors::{BuildError, Result};
use crate::utils::run_command;
use airbender_core::host::manifest::Profile;
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Returns the Dockerfile for the reproducible build image.
///
/// Base image digest sourced from `zksync-airbender/tools/reproduce/Dockerfile`.
/// To rotate the base image digest, update the sha256 hash below and bump
/// `DEFAULT_GUEST_TOOLCHAIN` in `constants.rs` to force a fresh `docker build`.
fn dockerfile_contents() -> String {
    format!(
        r#"FROM debian:bullseye-slim@sha256:f527627d07c18abf87313c341ee8ef1b36f106baa8b6b6dc33f4c872d988b651

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        build-essential \
        clang \
        git \
        libssl-dev \
        pkg-config \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --no-modify-path --default-toolchain {DEFAULT_GUEST_TOOLCHAIN}

RUN rustup component add llvm-tools-preview rust-src && \
    cargo install cargo-binutils --locked

WORKDIR /build
"#
    )
}

fn docker_image_tag() -> String {
    format!("airbender-build:{DEFAULT_GUEST_TOOLCHAIN}")
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

    // Pass `-` as the build context so Docker reads the Dockerfile from stdin —
    // no temp files or directories needed.
    let mut child = Command::new("docker")
        .args(["build", "--platform", "linux/amd64", "-t", &tag, "-"])
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(dockerfile_contents().as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(BuildError::ProcessFailed {
            cmd: "docker build".to_string(),
            status,
        });
    }
    Ok(())
}

/// RAII guard that force-removes a named Docker container when dropped.
struct TempContainer(String);

impl Drop for TempContainer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.0])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Generates a unique container name for the current build run.
///
/// XORs nanosecond timestamp with the process ID so that two concurrent builds
/// launched at the same instant (same `nanos`) from different processes still
/// get distinct names, and two builds in the same process at different times
/// (same `pid`) also get distinct names.
fn container_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let id = (nanos ^ (std::process::id() as u128)) as u64;
    format!("airbender-build-{id:016x}")
}

/// Builds the `sh -c` command string: `cargo build` then three `cargo objcopy` invocations.
fn build_container_cmd(
    bin_name: &str,
    target: &str,
    profile_flag: &str,
    cargo_args: &[String],
) -> String {
    let mut cargo_flags: Vec<&str> = Vec::new();
    if !profile_flag.is_empty() {
        cargo_flags.push(profile_flag);
    }
    cargo_flags.extend(["--bin", bin_name, "--target", target, "--locked"]);
    let user_flags = cargo_args.join(" ");
    let cargo_flags = if user_flags.is_empty() {
        cargo_flags.join(" ")
    } else {
        format!("{} {user_flags}", cargo_flags.join(" "))
    };

    let build = format!("cargo build {cargo_flags}");
    let obj_bin = format!("cargo objcopy {cargo_flags} -- -O binary /dist/app.bin");
    let obj_elf = format!("cargo objcopy {cargo_flags} -- -R .text /dist/app.elf");
    let obj_text =
        format!("cargo objcopy {cargo_flags} -- -O binary --only-section=.text /dist/app.text");

    format!("mkdir -p /dist && {build} && {obj_bin} && {obj_elf} && {obj_text}")
}

/// A resolved, ready-to-run reproducible build.
///
/// `new` performs all pre-flight checks (Cargo.lock, Docker availability, image)
/// and resolves workspace layout. `run` executes the container and extracts artifacts.
#[derive(Debug)]
pub(crate) struct ReproducibleBuild {
    tag: String,
    workspace_root: PathBuf,
    workdir: String,
    build_cmd: String,
    project_display: String,
}

impl ReproducibleBuild {
    /// Validates pre-conditions and resolves build parameters.
    ///
    /// Fails fast with [`BuildError::LockfileNotReady`] if `Cargo.lock` is absent,
    /// or with Docker errors if the daemon is unreachable or the image cannot be built.
    pub(crate) fn new(
        project_dir: &Path,
        workspace_root: &Path,
        bin_name: &str,
        target: Option<&str>,
        profile: Profile,
        cargo_args: &[String],
    ) -> Result<Self> {
        if !project_dir.join("Cargo.lock").exists() {
            return Err(BuildError::LockfileNotReady {
                project: project_dir.display().to_string(),
                toolchain: DEFAULT_GUEST_TOOLCHAIN,
            });
        }

        ensure_docker_available()?;
        ensure_image_built()?;

        let target = target.unwrap_or(DEFAULT_GUEST_TARGET);
        let profile_flag = if profile == Profile::Release {
            "--release"
        } else {
            ""
        };

        let project_abs = project_dir
            .canonicalize()
            .unwrap_or_else(|_| project_dir.to_path_buf());
        let project_rel = project_abs
            .strip_prefix(workspace_root)
            .unwrap_or(Path::new(""));

        Ok(Self {
            tag: docker_image_tag(),
            workdir: format!("/src/{}", project_rel.display()),
            build_cmd: build_container_cmd(bin_name, target, profile_flag, cargo_args),
            project_display: project_dir.display().to_string(),
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    /// Runs the build container and copies `app.bin`, `app.elf`, `app.text` into `dist_dir`.
    pub(crate) fn run(&self, dist_dir: &Path) -> Result<()> {
        // Guard registered before any Docker call — no orphan window.
        let name = container_name();
        let _guard = TempContainer(name.clone());
        self.run_container(&name)?;
        self.cp_artifacts(&name, dist_dir)
    }

    /// Starts the container and waits for it to exit, capturing stderr for error remapping.
    fn run_container(&self, name: &str) -> Result<()> {
        let mut cmd = Command::new("docker");
        cmd.args([
            "run",
            "--name",
            name,
            "--platform",
            "linux/amd64",
            "--workdir",
            &self.workdir,
            "-e",
            "CARGO_TARGET_DIR=/cargo-target",
            "-v",
            &format!("{}:/src:ro", self.workspace_root.display()),
            "-v",
            "airbender-cargo-registry:/usr/local/cargo/registry",
            &self.tag,
            "sh",
            "-c",
            &self.build_cmd,
        ]);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let mut build_stderr = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            stderr.read_to_string(&mut build_stderr)?;
        }
        let status = child.wait()?;

        eprint!("{build_stderr}");

        if !status.success() {
            if build_stderr.contains("cannot update the lock file") {
                return Err(BuildError::LockfileNotReady {
                    project: self.project_display.clone(),
                    toolchain: DEFAULT_GUEST_TOOLCHAIN,
                });
            }
            return Err(BuildError::ProcessFailed {
                cmd: "docker run".to_string(),
                status,
            });
        }
        Ok(())
    }

    /// Copies artifacts from `/dist` inside the container to `dist_dir` on the host.
    fn cp_artifacts(&self, name: &str, dist_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dist_dir)?;
        let src = format!("{name}:/dist/.");
        let mut cmd = Command::new("docker");
        cmd.args(["cp", &src, dist_dir.to_str().unwrap()]);
        run_command(cmd, "docker cp (artifacts out)")
    }
}

/// Removes the shared `airbender-cargo-registry` volume and any stopped
/// `airbender-build` containers left by interrupted builds.
///
/// Returns the number of resources removed.
pub fn clean_reproducible_volumes() -> Result<usize> {
    let vol_output = Command::new("docker")
        .args(["volume", "ls", "-q", "--filter", "name=airbender"])
        .output()?;
    let vol_stdout = String::from_utf8_lossy(&vol_output.stdout);
    let volumes: Vec<&str> = vol_stdout.lines().filter(|l| !l.is_empty()).collect();
    let vol_count = volumes.len();
    if vol_count > 0 {
        let mut cmd = Command::new("docker");
        cmd.args(["volume", "rm"]);
        cmd.args(&volumes);
        run_command(cmd, "docker volume rm")?;
    }

    let ctr_output = Command::new("docker")
        .args([
            "container",
            "ls",
            "-a",
            "-q",
            "--filter",
            "ancestor=airbender-build",
        ])
        .output()?;
    let ctr_stdout = String::from_utf8_lossy(&ctr_output.stdout);
    let containers: Vec<&str> = ctr_stdout.lines().filter(|l| !l.is_empty()).collect();
    let ctr_count = containers.len();
    if ctr_count > 0 {
        let mut cmd = Command::new("docker");
        cmd.args(["rm", "-f"]);
        cmd.args(&containers);
        run_command(cmd, "docker rm")?;
    }

    Ok(vol_count + ctr_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::BuildError;

    #[test]
    fn dockerfile_contents_contains_toolchain_date() {
        let contents = dockerfile_contents();
        assert!(contents.contains(DEFAULT_GUEST_TOOLCHAIN));
    }

    #[test]
    fn docker_image_tag_contains_toolchain() {
        let tag = docker_image_tag();
        assert!(tag.starts_with("airbender-build:"));
        assert!(tag.contains(DEFAULT_GUEST_TOOLCHAIN));
    }

    #[test]
    fn docker_image_tag_is_deterministic() {
        assert_eq!(docker_image_tag(), docker_image_tag());
    }

    #[test]
    fn reproducible_build_errors_when_lockfile_missing() {
        let tmp = std::env::temp_dir().join("airbender_test_lockfile_missing");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"guest\"\n").unwrap();

        let result = ReproducibleBuild::new(
            &tmp,
            &tmp,
            "guest",
            None,
            airbender_core::host::manifest::Profile::Release,
            &[],
        );

        std::fs::remove_dir_all(&tmp).ok();

        let err = result.unwrap_err();
        assert!(
            matches!(err, BuildError::LockfileNotReady { .. }),
            "expected LockfileNotReady, got: {err:?}"
        );
        assert!(err.to_string().contains(DEFAULT_GUEST_TOOLCHAIN));
    }
}
