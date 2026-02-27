//! Internal helpers for command execution, metadata loading, and validation.

use crate::errors::{BuildError, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use sha2::Digest;
use std::fmt::Write;
use std::path::{Component, Path};
use std::process::Command;

/// Build metadata extracted from the git repository when available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitMetadata {
    pub branch: String,
    pub commit: String,
    pub is_dirty: bool,
}

/// Runs a command and maps non-success exit codes into [`BuildError`].
pub(crate) fn run_command(mut cmd: Command, name: &str) -> Result<()> {
    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(BuildError::ProcessFailed {
            cmd: name.to_string(),
            status,
        })
    }
}

/// Validates that `app_name` is exactly one normal path segment.
///
/// This prevents accidental writes outside the dist root via separators,
/// absolute prefixes, or path traversal segments.
pub(crate) fn validate_app_name(app_name: &str) -> Result<()> {
    if app_name.is_empty() {
        return Err(BuildError::InvalidConfig(
            "app name must not be empty".to_string(),
        ));
    }
    if app_name.contains('/') || app_name.contains('\\') {
        return Err(BuildError::InvalidConfig(format!(
            "app name `{app_name}` must be a single path segment"
        )));
    }

    let mut components = Path::new(app_name).components();
    let first_component = components.next();
    let second_component = components.next();
    match (first_component, second_component) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(BuildError::InvalidConfig(format!(
            "app name `{app_name}` must be a single path segment"
        ))),
    }
}

/// Executes `cargo metadata --no-deps` for the provided manifest path.
pub(crate) fn load_metadata(manifest_path: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .map_err(|err| BuildError::InvalidConfig(format!("cargo metadata failed: {err}")))
}

/// Finds the package that corresponds to `manifest_path` within metadata output.
///
/// If an exact manifest match is absent, this falls back to Cargo's root package.
pub(crate) fn find_package<'a>(
    metadata: &'a Metadata,
    manifest_path: &Path,
) -> Result<&'a Package> {
    let manifest_path = manifest_path.canonicalize()?;
    let manifest_path =
        cargo_metadata::camino::Utf8PathBuf::from_path_buf(manifest_path).map_err(|path| {
            BuildError::InvalidConfig(format!(
                "manifest path is not valid UTF-8: {}",
                path.display()
            ))
        })?;

    if let Some(pkg) = metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path == manifest_path)
    {
        return Ok(pkg);
    }

    metadata
        .root_package()
        .ok_or(BuildError::MissingField("package.name"))
}

/// Resolves the binary target name used for build commands.
///
/// This enforces explicit selection when a package defines multiple binary targets.
pub(crate) fn resolve_bin_name(package: &Package, explicit_bin: Option<&str>) -> Result<String> {
    let bin_names: Vec<&str> = package
        .targets
        .iter()
        .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
        .map(|target| target.name.as_str())
        .collect();

    resolve_bin_name_from_candidates(&package.name, &bin_names, explicit_bin)
}

fn resolve_bin_name_from_candidates(
    package_name: &str,
    bin_names: &[&str],
    explicit_bin: Option<&str>,
) -> Result<String> {
    if let Some(explicit_bin) = explicit_bin {
        if bin_names.contains(&explicit_bin) {
            return Ok(explicit_bin.to_string());
        }

        let available = if bin_names.is_empty() {
            "<none>".to_string()
        } else {
            bin_names.join(", ")
        };

        return Err(BuildError::InvalidConfig(format!(
            "binary target `{explicit_bin}` not found in package `{package_name}`; available binaries: {available}"
        )));
    }

    match bin_names {
        [single] => Ok((*single).to_string()),
        [] => Err(BuildError::InvalidConfig(format!(
            "package `{package_name}` has no binary targets"
        ))),
        _ => Err(BuildError::InvalidConfig(format!(
            "package `{package_name}` has multiple binary targets ({}); pass `--bin <name>`",
            bin_names.join(", ")
        ))),
    }
}

/// Computes a lowercase hex SHA-256 digest for a file.
pub(crate) fn sha256_file_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let digest = sha2::Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to string cannot fail");
    }
    Ok(encoded)
}

/// Resolves git metadata for the guest project repository.
///
/// If metadata cannot be resolved, this returns fallback values:
/// - branch = "N/A"
/// - commit = "N/A"
/// - is_dirty = true
pub(crate) fn resolve_git_metadata(project_dir: &Path) -> GitMetadata {
    let Some(branch) = run_git_stdout(project_dir, &["rev-parse", "--abbrev-ref", "HEAD"]) else {
        return fallback_git_metadata();
    };
    let Some(commit) = run_git_stdout(project_dir, &["rev-parse", "HEAD"]) else {
        return fallback_git_metadata();
    };
    let Some(is_dirty) = has_unstaged_changes(project_dir) else {
        return fallback_git_metadata();
    };

    GitMetadata {
        branch,
        commit,
        is_dirty,
    }
}

fn run_git_stdout(project_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn has_unstaged_changes(project_dir: &Path) -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let status = String::from_utf8(output.stdout).ok()?;
    for line in status.lines() {
        if line.len() < 2 {
            continue;
        }

        let bytes = line.as_bytes();
        let unstaged = bytes[1] != b' ';
        if unstaged {
            return Some(true);
        }
    }

    Some(false)
}

fn fallback_git_metadata() -> GitMetadata {
    GitMetadata {
        branch: "N/A".to_string(),
        commit: "N/A".to_string(),
        is_dirty: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_when_git_metadata_is_unavailable() {
        let dir = unique_temp_dir_path("git-metadata-fallback");
        std::fs::create_dir_all(&dir).expect("create temp directory");

        let metadata = resolve_git_metadata(&dir);
        assert_eq!(metadata.branch, "N/A");
        assert_eq!(metadata.commit, "N/A");
        assert!(metadata.is_dirty);

        std::fs::remove_dir_all(&dir).expect("remove temp directory");
    }

    #[test]
    fn resolve_bin_name_uses_single_candidate_without_explicit_override() {
        let resolved = resolve_bin_name_from_candidates("guest", &["guest"], None)
            .expect("single binary should resolve");

        assert_eq!(resolved, "guest");
    }

    #[test]
    fn resolve_bin_name_rejects_ambiguous_multi_bin_without_explicit_override() {
        let err = resolve_bin_name_from_candidates("guest", &["alpha", "beta"], None)
            .expect_err("multi-bin package should require --bin");

        assert_eq!(
            err.to_string(),
            "invalid config: package `guest` has multiple binary targets (alpha, beta); pass `--bin <name>`"
        );
    }

    #[test]
    fn resolve_bin_name_validates_explicit_bin_against_candidates() {
        let err = resolve_bin_name_from_candidates("guest", &["alpha", "beta"], Some("gamma"))
            .expect_err("unknown explicit bin must fail fast");

        assert_eq!(
            err.to_string(),
            "invalid config: binary target `gamma` not found in package `guest`; available binaries: alpha, beta"
        );
    }

    #[test]
    fn resolve_bin_name_accepts_explicit_candidate() {
        let resolved = resolve_bin_name_from_candidates("guest", &["alpha", "beta"], Some("beta"))
            .expect("known explicit bin should resolve");

        assert_eq!(resolved, "beta");
    }

    #[test]
    fn resolve_bin_name_rejects_packages_without_binaries() {
        let err = resolve_bin_name_from_candidates("guest", &[], None)
            .expect_err("package without binaries should fail");

        assert_eq!(
            err.to_string(),
            "invalid config: package `guest` has no binary targets"
        );
    }

    fn unique_temp_dir_path(label: &str) -> std::path::PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "airbender-build-utils-{label}-{}-{now}",
            std::process::id()
        ))
    }
}
