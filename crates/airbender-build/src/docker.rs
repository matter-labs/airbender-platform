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
use std::io::{self, Read};
use std::path::{Path, PathBuf};
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
    let dockerfile_path =
        std::env::temp_dir().join(format!("airbender-{}.dockerfile", DEFAULT_GUEST_TOOLCHAIN));
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

/// Walks up from `start` to find the nearest directory containing a `Cargo.toml`
/// with a `[workspace]` section. Falls back to `start` if none is found.
fn find_workspace_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.exists() {
            if let Ok(contents) = std::fs::read_to_string(&manifest) {
                if contents.contains("[workspace]") {
                    return dir;
                }
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return start.to_path_buf(),
        }
    }
}

/// Shared parameters for `docker run cargo …` invocations.
struct DockerRunArgs<'a> {
    tag: &'a str,
    workspace_root: &'a Path,
    workdir: &'a str,
    dist_dir: &'a Path,
    target_volume: &'a str,
    fixed_args: &'a [&'a str],
    user_args: &'a [String],
}

impl DockerRunArgs<'_> {
    fn base_command(&self) -> Command {
        let mut cmd = Command::new("docker");
        cmd.args([
            "run",
            "--rm",
            "--platform",
            "linux/amd64",
            "--workdir",
            self.workdir,
            "-e",
            "CARGO_TARGET_DIR=/cargo-target",
        ]);
        cmd.args(["-v", &format!("{}:/src:ro", self.workspace_root.display())]);
        cmd.args(["-v", &format!("{}:/out:rw", self.dist_dir.display())]);
        cmd.args(["-v", "airbender-cargo-registry:/usr/local/cargo/registry"]);
        cmd.args(["-v", &format!("{}:/cargo-target", self.target_volume)]);
        cmd.arg(self.tag);
        cmd
    }
}

/// Runs a single `cargo <subcmd>` invocation inside the pinned container.
///
/// Volume mounts:
/// - `/src`       — workspace root, read-only (so path dependencies resolve correctly)
/// - `/out`       — dist output directory, read-write (objcopy writes artifacts here)
/// - `airbender-cargo-registry` — shared named volume for downloaded crate sources
/// - `<target_volume>` — per-project named volume for incremental build cache
fn docker_run_cargo(args: &DockerRunArgs<'_>, subcmd: &str, objcopy_args: &[&str]) -> Result<()> {
    let mut cmd = args.base_command();
    cmd.args(["cargo", subcmd]);
    cmd.args(args.fixed_args);
    cmd.args(args.user_args);
    if !objcopy_args.is_empty() {
        cmd.arg("--");
        cmd.args(objcopy_args);
    }
    run_command(cmd, &format!("docker run cargo {subcmd}"))
}

/// Like `docker_run_cargo` but captures stderr so that a `--locked` failure from cargo
/// (caused by a Cargo.lock generated with a different toolchain) is remapped to
/// `BuildError::LockfileNotReady` with an actionable fix command.
///
/// Captured stderr is always forwarded to the process's stderr so build output is not lost.
fn docker_run_cargo_build(args: &DockerRunArgs<'_>, project_display: &str) -> Result<()> {
    let mut cmd = args.base_command();
    cmd.args(["cargo", "build"]);
    cmd.args(args.fixed_args);
    cmd.args(args.user_args);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let mut stderr_buf = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        stderr.read_to_string(&mut stderr_buf)?;
    }
    let status = child.wait()?;

    // Always forward captured stderr so build diagnostics are visible.
    eprint!("{stderr_buf}");

    if !status.success() {
        if stderr_buf.contains("cannot update the lock file") {
            return Err(BuildError::LockfileNotReady {
                project: project_display.to_string(),
                toolchain: DEFAULT_GUEST_TOOLCHAIN,
            });
        }
        return Err(BuildError::ProcessFailed {
            cmd: "docker run cargo build".to_string(),
            status,
        });
    }
    Ok(())
}

/// Compiles a guest program and produces `app.bin`, `app.elf`, and `app.text` inside `dist_dir`,
/// using a pinned Docker container for bit-for-bit reproducible output.
///
/// `--locked` is passed to pin dependency versions exactly. The project must have a committed
/// `Cargo.lock` generated with `DEFAULT_GUEST_TOOLCHAIN`; a pre-flight check surfaces a
/// `LockfileNotReady` error with the fix command if the file is absent or incompatible.
pub(crate) fn run_reproducible_build(
    project_dir: &Path,
    bin_name: &str,
    target: Option<&str>,
    profile: Profile,
    dist_dir: &Path,
    cargo_args: &[String],
) -> Result<()> {
    // Validate Cargo.lock exists before spinning up Docker. The lock file must be committed
    // and generated with DEFAULT_GUEST_TOOLCHAIN; otherwise --locked will fail inside the
    // container. We can't distinguish "missing" from "wrong toolchain" without running cargo,
    // so both cases are covered by a single pre-flight error with the fix command.
    if !project_dir.join("Cargo.lock").exists() {
        return Err(BuildError::LockfileNotReady {
            project: project_dir.display().to_string(),
            toolchain: DEFAULT_GUEST_TOOLCHAIN,
        });
    }

    ensure_docker_available()?;
    ensure_image_built()?;

    let tag = docker_image_tag();
    let target_str = target.unwrap_or(DEFAULT_GUEST_TARGET);
    let profile_flag = if profile == Profile::Release {
        "--release"
    } else {
        ""
    };

    // Mount the workspace root so path dependencies (e.g. ../../../crates/foo) resolve inside
    // the container. The workdir is the guest project's path relative to the workspace root,
    // prefixed with /src.
    let canonical_project = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    let workspace_root = find_workspace_root(&canonical_project);
    let rel = canonical_project
        .strip_prefix(&workspace_root)
        .unwrap_or(Path::new(""));
    let workdir = format!("/src/{}", rel.display());

    // Stable per-project volume name derived from the canonical project path.
    // Uses the first 8 bytes of SHA-256 to keep the name short and Docker-safe.
    let project_key = {
        let hash = Sha256::digest(canonical_project.to_string_lossy().as_bytes());
        hash[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let target_volume = format!("airbender-cargo-target-{project_key}");

    // Build args shared across all four invocations.
    // --target-dir is passed via CARGO_TARGET_DIR env var (cargo objcopy doesn't accept the flag).
    let fixed: &[&str] = if profile_flag.is_empty() {
        &["--bin", bin_name, "--target", target_str, "--locked"]
    } else {
        &[
            profile_flag,
            "--bin",
            bin_name,
            "--target",
            target_str,
            "--locked",
        ]
    };

    let run_args = DockerRunArgs {
        tag: &tag,
        workspace_root: &workspace_root,
        workdir: &workdir,
        dist_dir,
        target_volume: &target_volume,
        fixed_args: fixed,
        user_args: cargo_args,
    };

    // 1. Compile — uses the lockfile-detecting variant so a wrong-toolchain Cargo.lock
    //    surfaces as LockfileNotReady rather than a generic process failure.
    docker_run_cargo_build(&run_args, &project_dir.display().to_string())?;

    // 2–4. Extract binary artifacts.
    docker_run_cargo(&run_args, "objcopy", &["-O", "binary", "/out/app.bin"])?;
    docker_run_cargo(&run_args, "objcopy", &["-R", ".text", "/out/app.elf"])?;
    docker_run_cargo(
        &run_args,
        "objcopy",
        &["-O", "binary", "--only-section=.text", "/out/app.text"],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::BuildError;

    #[test]
    fn dockerfile_contents_substitutes_toolchain_date() {
        let contents = dockerfile_contents();
        assert!(!contents.contains("{{TOOLCHAIN_DATE}}"));
        assert!(contents.contains(DEFAULT_GUEST_TOOLCHAIN));
    }

    #[test]
    fn dockerfile_template_has_no_hardcoded_date() {
        assert!(DOCKERFILE_TEMPLATE.contains("{{TOOLCHAIN_DATE}}"));
        assert!(!DOCKERFILE_TEMPLATE.contains(DEFAULT_GUEST_TOOLCHAIN));
    }

    #[test]
    fn docker_image_tag_contains_toolchain() {
        let tag = docker_image_tag();
        assert!(tag.starts_with("airbender-build:"));
        assert!(tag.ends_with(DEFAULT_GUEST_TOOLCHAIN));
    }

    #[test]
    fn find_workspace_root_finds_ancestor_with_workspace_section() {
        let tmp = std::env::temp_dir().join("airbender_test_workspace_root");
        let nested = tmp.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(nested.join("Cargo.toml"), "[package]\nname = \"pkg\"\n").unwrap();

        let result = find_workspace_root(&nested);
        assert_eq!(result, tmp);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn find_workspace_root_falls_back_to_start_when_no_workspace_found() {
        let tmp = std::env::temp_dir().join("airbender_test_no_workspace");
        let nested = tmp.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();

        let result = find_workspace_root(&nested);
        assert_eq!(result, nested);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn find_workspace_root_stops_at_nearest_workspace() {
        let tmp = std::env::temp_dir().join("airbender_test_nested_workspace");
        let inner = tmp.join("inner");
        let pkg = inner.join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(inner.join("Cargo.toml"), "[workspace]\n").unwrap();

        let result = find_workspace_root(&pkg);
        assert_eq!(result, inner);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn reproducible_build_errors_when_lockfile_missing() {
        let tmp = std::env::temp_dir().join("airbender_test_lockfile_missing");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"guest\"\n").unwrap();

        let result = run_reproducible_build(
            &tmp,
            "guest",
            None,
            airbender_core::host::manifest::Profile::Release,
            &tmp,
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
