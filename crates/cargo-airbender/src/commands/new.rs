use crate::cli::NewArgs;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const TEMPLATE_FILES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../../templates/guest/Cargo.toml"),
    ),
    (
        "src/main.rs",
        include_str!("../../templates/guest/src/main.rs"),
    ),
];

pub fn run(args: NewArgs) -> Result<()> {
    let project_name = args.name.or_else(|| {
        args.path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
    });
    let project_name = project_name.context("while attempting to determine project name")?;

    ensure_empty_dir(&args.path)?;
    fs::create_dir_all(&args.path).with_context(|| {
        format!(
            "while attempting to create destination directory {}",
            args.path.display()
        )
    })?;

    let destination_dir = args
        .path
        .canonicalize()
        .with_context(|| format!("while attempting to canonicalize {}", args.path.display()))?;
    let sdk_dependency = resolve_sdk_dependency(&destination_dir, args.sdk_path, args.sdk_version)?;
    let replacements = [
        ("__AIRBENDER_PROJECT_NAME__", project_name.as_str()),
        ("__AIRBENDER_SDK_DEP__", sdk_dependency.as_str()),
    ];

    write_template(&args.path, &replacements)?;
    tracing::info!("created project at {}", args.path.display());
    Ok(())
}

fn resolve_sdk_dependency(
    destination_dir: &Path,
    sdk_path: Option<PathBuf>,
    sdk_version: Option<String>,
) -> Result<String> {
    if let Some(version) = sdk_version {
        if version.is_empty() {
            bail!("--sdk-version cannot be empty");
        }
        return Ok(format!("version = \"{version}\""));
    }

    // TODO: switch the default to crates.io once airbender-sdk is published.
    let sdk_path = sdk_path.unwrap_or_else(default_sdk_path);
    if !sdk_path.exists() {
        bail!(
            "failed to locate airbender-sdk at {} (pass --sdk-path <path> or --sdk-version <version>)",
            sdk_path.display()
        );
    }

    let sdk_path = sdk_path
        .canonicalize()
        .with_context(|| format!("while attempting to canonicalize {}", sdk_path.display()))?;
    let sdk_relative = relative_path(destination_dir, &sdk_path)?;
    Ok(format!("path = \"{}\"", sdk_relative.to_string_lossy()))
}

fn default_sdk_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("crates")
        .join("airbender-sdk")
}

fn ensure_empty_dir(path: &Path) -> Result<()> {
    if path.exists()
        && path
            .read_dir()
            .with_context(|| format!("while attempting to list {}", path.display()))?
            .next()
            .is_some()
    {
        bail!("destination directory is not empty: {}", path.display());
    }
    Ok(())
}

fn write_template(destination_root: &Path, replacements: &[(&str, &str)]) -> Result<()> {
    for (relative_path, source) in TEMPLATE_FILES {
        let destination_path = destination_root.join(relative_path);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("while attempting to create {}", parent.display()))?;
        }

        let mut content = source.to_string();
        for (from, to) in replacements {
            content = content.replace(from, to);
        }
        fs::write(&destination_path, content)
            .with_context(|| format!("while attempting to write {}", destination_path.display()))?;
    }
    Ok(())
}

fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let from = from
        .canonicalize()
        .with_context(|| format!("while attempting to canonicalize {}", from.display()))?;
    let to = to
        .canonicalize()
        .with_context(|| format!("while attempting to canonicalize {}", to.display()))?;

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
