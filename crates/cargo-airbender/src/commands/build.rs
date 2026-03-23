use crate::cli::{BuildArgs, BuildProfile};
use crate::error::{CliError, Result};
use crate::ui;
use airbender_build::{build_dist, BuildConfig, Profile, DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
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
        reproducible,
    } = args;

    let project_dir = match project {
        Some(path) => path,
        None => {
            let invocation_cwd = std::env::current_dir().map_err(|err| {
                CliError::with_source("failed to resolve current working directory", err)
            })?;
            discover_project_dir_from(&invocation_cwd)?
        }
    };

    let manifest_path = project_dir.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Err(missing_manifest_error(&project_dir));
    }

    let mut config = BuildConfig::new(project_dir);
    config.app_name = app_name;
    config.bin_name = bin;
    config.target = target;
    config.dist_dir = dist;
    config.profile = resolve_profile(profile, debug, release);
    config.cargo_args = cargo_args;
    config.reproducible = reproducible;

    let artifacts = build_dist(&config).map_err(|err| {
        CliError::with_source("failed to build guest artifacts", err)
            .with_hint("set `RUST_LOG=info` if you need backend diagnostic logs")
    })?;

    ui::success("built guest artifacts");
    if reproducible {
        ui::info("reproducible build (Docker)");
        ui::field("toolchain", DEFAULT_GUEST_TOOLCHAIN);
    }
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

fn discover_project_dir_from(invocation_cwd: &Path) -> Result<PathBuf> {
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
    cargo_config_targets_guest(&cargo_config)
}

fn cargo_config_targets_guest(cargo_config: &str) -> std::io::Result<bool> {
    let cargo_config = cargo_config.parse::<toml::Table>().map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to parse .cargo/config.toml: {err}"),
        )
    })?;

    Ok(cargo_config
        .get("build")
        .and_then(toml::Value::as_table)
        .and_then(|build| build.get("target"))
        .and_then(toml::Value::as_str)
        == Some(DEFAULT_GUEST_TARGET))
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
    use std::fs;

    #[test]
    fn discovers_project_dir_from_parent_manifest_when_project_is_omitted() {
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let project_dir = temp_dir.path().join("guest");
        let nested_dir = project_dir.join("src").join("nested");

        write_guest_project(&project_dir);
        fs::create_dir_all(&nested_dir).expect("create nested guest directory");

        let resolved = discover_project_dir_from(&nested_dir).expect("resolve project dir");

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

        let err = discover_project_dir_from(&nested_dir).expect_err("missing guest manifest");

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

        let err = discover_project_dir_from(&nested_dir).expect_err("missing guest manifest");

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

        let err = discover_project_dir_from(&nested_dir).expect_err("missing manifest");

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
    fn detects_guest_target_from_cargo_config_toml() {
        let cargo_config = format!(
            "[build]\nrustflags = [\"-C\", \"link-arg=-Tmemory.x\"]\ntarget = \"{DEFAULT_GUEST_TARGET}\"\n"
        );

        let targets_guest =
            cargo_config_targets_guest(&cargo_config).expect("parse guest cargo config");

        assert!(targets_guest);
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
    }

    fn write_file(path: &Path, contents: &str) {
        let parent = path
            .parent()
            .expect("test file should have a parent directory");
        fs::create_dir_all(parent).expect("create test directory");
        fs::write(path, contents).expect("write test file");
    }
}
