use crate::cli::{BuildArgs, BuildProfile};
use crate::error::{CliError, Result};
use crate::ui;
use airbender_build::{build_dist, BuildConfig, Profile, DEFAULT_GUEST_TARGET};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: BuildArgs) -> Result<()> {
    let BuildArgs {
        app_name,
        bin,
        target,
        dist,
        project,
        profile,
        debug,
        release,
        cargo_args,
    } = args;

    let project_dir = resolve_project_dir(project.as_deref())?;

    let mut config = BuildConfig::new(project_dir);
    config.app_name = app_name;
    config.bin_name = bin;
    config.target = target;
    config.dist_dir = dist;
    config.profile = resolve_profile(profile, debug, release);
    config.cargo_args = cargo_args;

    let artifacts = build_dist(&config).map_err(|err| {
        CliError::with_source("failed to build guest artifacts", err)
            .with_hint("set `RUST_LOG=info` if you need backend diagnostic logs")
    })?;

    ui::success("built guest artifacts");
    ui::field("dist", artifacts.dist_dir.display());
    ui::field("app.bin", artifacts.app_bin.display());
    ui::field("app.elf", artifacts.app_elf.display());
    ui::field("app.text", artifacts.app_text.display());
    ui::field("manifest", artifacts.manifest.display());
    ui::blank_line();
    ui::info("next step");
    ui::command(format!(
        "cargo airbender run \"{}\" --input <input.hex>",
        artifacts.app_bin.display()
    ));

    Ok(())
}

fn resolve_profile(profile: Option<BuildProfile>, debug: bool, release: bool) -> Profile {
    if debug {
        return Profile::Debug;
    }
    if release {
        return Profile::Release;
    }
    match profile {
        Some(BuildProfile::Debug) => Profile::Debug,
        Some(BuildProfile::Release) => Profile::Release,
        None => Profile::Release,
    }
}

fn resolve_project_dir(project: Option<&Path>) -> Result<PathBuf> {
    let invocation_cwd = std::env::current_dir()
        .map_err(|err| CliError::with_source("failed to resolve current working directory", err))?;
    resolve_project_dir_from(project, &invocation_cwd)
}

fn resolve_project_dir_from(project: Option<&Path>, invocation_cwd: &Path) -> Result<PathBuf> {
    if let Some(project_dir) = project {
        let project_dir = if project_dir.is_absolute() {
            project_dir.to_path_buf()
        } else {
            invocation_cwd.join(project_dir)
        };
        return if project_dir.join("Cargo.toml").is_file() {
            Ok(project_dir)
        } else {
            Err(missing_manifest_error(&project_dir))
        };
    }

    for candidate in invocation_cwd.ancestors() {
        if !candidate.join("Cargo.toml").is_file() {
            continue;
        }

        let is_guest = is_guest_project_dir(candidate).map_err(|err| {
            CliError::with_source(
                format!("failed to inspect guest project `{}`", candidate.display()),
                err,
            )
        })?;
        if is_guest {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(missing_manifest_error(invocation_cwd))
}

fn is_guest_project_dir(project_dir: &Path) -> std::io::Result<bool> {
    let cargo_config = project_dir.join(".cargo/config.toml");
    if !cargo_config.is_file() {
        return Ok(false);
    }

    let cargo_config = fs::read_to_string(cargo_config)?;
    Ok(cargo_config_targets_guest(&cargo_config))
}

fn cargo_config_targets_guest(cargo_config: &str) -> bool {
    cargo_config.lines().map(str::trim).any(|line| {
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        if key.trim() != "target" {
            return false;
        }

        let value = value.split('#').next().unwrap_or(value).trim();
        value.trim_matches(|ch| ch == '"' || ch == '\'') == DEFAULT_GUEST_TARGET
    })
}

fn missing_manifest_error(project_dir: &Path) -> CliError {
    CliError::new(format!(
        "guest project `{}` does not contain a Cargo.toml",
        project_dir.display()
    ))
    .with_hint("use --project <path-to-guest-crate>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use airbender_build::DEFAULT_GUEST_TOOLCHAIN;
    use std::fs;

    #[test]
    fn resolves_project_dir_from_parent_manifest_when_project_is_omitted() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let project_dir = temp_dir.path().join("guest");
        let nested_dir = project_dir.join("src").join("nested");

        write_guest_project(&project_dir);
        fs::create_dir_all(&nested_dir).expect("create nested guest directory");

        let resolved = resolve_project_dir_from(None, &nested_dir).expect("resolve project dir");

        assert_eq!(resolved, project_dir);
    }

    #[test]
    fn resolves_relative_explicit_project_from_invocation_cwd() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let invocation_cwd = temp_dir.path().join("workspace");
        let project_dir = invocation_cwd.join("guest");

        write_package_manifest(&project_dir, "guest", "");

        let resolved = resolve_project_dir_from(Some(Path::new("guest")), &invocation_cwd)
            .expect("resolve explicit project");

        assert_eq!(resolved, project_dir);
    }

    #[test]
    fn returns_hint_when_only_non_guest_manifests_exist_in_ancestors() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let project_dir = temp_dir.path().join("helper");
        let nested_dir = project_dir.join("src");

        write_package_manifest(
            &project_dir,
            "helper",
            "\n[dependencies]\nairbender-sdk = \"0.1\"\n",
        );
        fs::create_dir_all(&nested_dir).expect("create nested host directory");

        let err = resolve_project_dir_from(None, &nested_dir).expect_err("missing guest manifest");

        assert_eq!(
            err.to_string(),
            format!(
                "guest project `{}` does not contain a Cargo.toml",
                nested_dir.display()
            )
        );
        assert_eq!(err.hint(), Some("use --project <path-to-guest-crate>"));
    }

    #[test]
    fn skips_workspace_manifests_when_project_is_omitted() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let project_dir = temp_dir.path().join("host");
        let nested_dir = project_dir.join("src");

        write_file(
            &temp_dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"host\"]\n",
        );
        write_package_manifest(&project_dir, "host", "");
        fs::create_dir_all(&nested_dir).expect("create nested host directory");

        let err = resolve_project_dir_from(None, &nested_dir).expect_err("missing guest manifest");

        assert_eq!(
            err.to_string(),
            format!(
                "guest project `{}` does not contain a Cargo.toml",
                nested_dir.display()
            )
        );
        assert_eq!(err.hint(), Some("use --project <path-to-guest-crate>"));
    }

    #[test]
    fn returns_hint_when_no_manifest_exists_in_ancestors() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let nested_dir = temp_dir.path().join("guest").join("src");
        fs::create_dir_all(&nested_dir).expect("create nested guest directory");

        let err = resolve_project_dir_from(None, &nested_dir).expect_err("missing manifest");

        assert_eq!(
            err.to_string(),
            format!(
                "guest project `{}` does not contain a Cargo.toml",
                nested_dir.display()
            )
        );
        assert_eq!(err.hint(), Some("use --project <path-to-guest-crate>"));
    }

    #[test]
    fn resolves_project_dir_when_guest_toolchain_is_customized() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let project_dir = temp_dir.path().join("guest");
        let nested_dir = project_dir.join("src").join("nested");

        write_guest_project(&project_dir);
        write_file(
            &project_dir.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"nightly\"\n",
        );
        fs::create_dir_all(&nested_dir).expect("create nested guest directory");

        let resolved = resolve_project_dir_from(None, &nested_dir).expect("resolve project dir");

        assert_eq!(resolved, project_dir);
    }

    fn write_package_manifest(project_dir: &Path, name: &str, suffix: &str) {
        write_file(
            &project_dir.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{suffix}"
            ),
        );
        write_file(&project_dir.join("src/main.rs"), "fn main() {}\n");
    }

    fn write_guest_project(project_dir: &Path) {
        write_package_manifest(project_dir, "guest", "");
        write_file(
            &project_dir.join(".cargo/config.toml"),
            &format!("[build]\ntarget = \"{DEFAULT_GUEST_TARGET}\"\n"),
        );
        write_file(
            &project_dir.join("rust-toolchain.toml"),
            &format!("[toolchain]\nchannel = \"{DEFAULT_GUEST_TOOLCHAIN}\"\n"),
        );
    }

    fn write_file(path: &Path, contents: &str) {
        let parent = path
            .parent()
            .expect("test file should have a parent directory");
        fs::create_dir_all(parent).expect("create test directory");
        fs::write(path, contents).expect("write test file");
    }
}
