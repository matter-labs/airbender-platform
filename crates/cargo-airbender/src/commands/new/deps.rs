use crate::error::{CliError, Result};
use std::path::{Path, PathBuf};

const DEFAULT_SDK_GIT_REPOSITORY: &str = "https://github.com/matter-labs/airbender-platform";
const DEFAULT_SDK_GIT_BRANCH: &str = "main";

pub(super) fn resolve_crate_dependency(
    destination_crate_dir: &Path,
    sdk_path: Option<&Path>,
    sdk_version: Option<&str>,
    crate_name: &str,
) -> Result<String> {
    if let Some(version) = sdk_version {
        if version.is_empty() {
            return Err(CliError::new("`--sdk-version` cannot be empty"));
        }

        return Ok(format!("version = \"{version}\""));
    }

    if let Some(sdk_path) = sdk_path {
        if !sdk_path.exists() {
            return Err(
                CliError::new(format!("SDK path `{}` does not exist", sdk_path.display()))
                    .with_hint(
                        "pass `--sdk-path` pointing to an existing airbender-platform checkout",
                    ),
            );
        }

        let crate_path = resolve_dependency_crate_path(sdk_path, crate_name)?;
        let sdk_relative = relative_path(destination_crate_dir, &crate_path)?;
        return Ok(format!("path = \"{}\"", sdk_relative.to_string_lossy()));
    }

    Ok(format!(
        "git = \"{DEFAULT_SDK_GIT_REPOSITORY}\", branch = \"{DEFAULT_SDK_GIT_BRANCH}\""
    ))
}

fn resolve_dependency_crate_path(sdk_path: &Path, crate_name: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    if sdk_path
        .file_name()
        .map(|name| name == crate_name)
        .unwrap_or(false)
    {
        candidates.push(sdk_path.to_path_buf());
    }
    candidates.push(sdk_path.join(crate_name));
    candidates.push(sdk_path.join("crates").join(crate_name));
    if let Some(parent) = sdk_path.parent() {
        candidates.push(parent.join(crate_name));
    }

    for candidate in candidates {
        if candidate.join("Cargo.toml").exists() {
            return candidate.canonicalize().map_err(|err| {
                CliError::with_source(
                    format!("failed to canonicalize `{}`", candidate.display()),
                    err,
                )
            });
        }
    }

    Err(CliError::new(format!(
        "failed to locate `{crate_name}` under `{}`",
        sdk_path.display()
    ))
    .with_hint("point `--sdk-path` to the workspace root or crate directory"))
}

fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let from = from.canonicalize().map_err(|err| {
        CliError::with_source(format!("failed to canonicalize `{}`", from.display()), err)
    })?;
    let to = to.canonicalize().map_err(|err| {
        CliError::with_source(format!("failed to canonicalize `{}`", to.display()), err)
    })?;

    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let mut common_len = 0usize;
    while common_len < from_components.len()
        && common_len < to_components.len()
        && from_components[common_len] == to_components[common_len]
    {
        common_len += 1;
    }

    let mut result = PathBuf::new();
    for _ in common_len..from_components.len() {
        result.push("..");
    }
    for component in &to_components[common_len..] {
        result.push(component.as_os_str());
    }

    Ok(result)
}
