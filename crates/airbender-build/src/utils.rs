//! Internal helpers for command execution, metadata loading, and validation.

use crate::errors::{BuildError, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use sha2::Digest;
use std::fmt::Write;
use std::path::{Component, Path};
use std::process::Command;

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

/// Picks the primary binary target name for build commands.
///
/// If no explicit binary target exists, this falls back to package name.
pub(crate) fn select_bin_name(package: &Package) -> String {
    package
        .targets
        .iter()
        .find(|target| target.kind.iter().any(|kind| kind == "bin"))
        .map(|target| target.name.clone())
        .unwrap_or_else(|| package.name.clone())
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
