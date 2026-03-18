use crate::cli::{BuildArgs, BuildProfile};
use crate::error::{CliError, Result};
use crate::ui;
use airbender_build::{build_dist, BuildConfig, Profile};
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
    match project {
        Some(project_dir) => {
            let project_dir = resolve_project_path(project_dir, invocation_cwd);
            ensure_manifest_exists(&project_dir)?;
            Ok(project_dir)
        }
        None => {
            find_project_dir(invocation_cwd).ok_or_else(|| missing_manifest_error(invocation_cwd))
        }
    }
}

fn resolve_project_path(project_dir: &Path, invocation_cwd: &Path) -> PathBuf {
    if project_dir.is_absolute() {
        project_dir.to_path_buf()
    } else {
        invocation_cwd.join(project_dir)
    }
}

fn find_project_dir(start_dir: &Path) -> Option<PathBuf> {
    start_dir
        .ancestors()
        .find(|candidate| candidate.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

fn ensure_manifest_exists(project_dir: &Path) -> Result<()> {
    if project_dir.join("Cargo.toml").is_file() {
        Ok(())
    } else {
        Err(missing_manifest_error(project_dir))
    }
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);

            let unique_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "cargo-airbender-build-tests-{}-{timestamp}-{unique_id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temporary test directory");

            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn resolves_project_dir_from_parent_manifest_when_project_is_omitted() {
        let temp_dir = TempDir::new();
        let project_dir = temp_dir.path().join("guest");
        let nested_dir = project_dir.join("src").join("nested");

        fs::create_dir_all(&nested_dir).expect("create nested guest directory");
        fs::write(
            project_dir.join("Cargo.toml"),
            "[package]\nname = \"guest\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write guest manifest");

        let resolved = resolve_project_dir_from(None, &nested_dir).expect("resolve project dir");

        assert_eq!(resolved, project_dir);
    }

    #[test]
    fn resolves_relative_explicit_project_from_invocation_cwd() {
        let temp_dir = TempDir::new();
        let invocation_cwd = temp_dir.path().join("workspace");
        let project_dir = invocation_cwd.join("guest");

        fs::create_dir_all(&project_dir).expect("create guest project directory");
        fs::write(
            project_dir.join("Cargo.toml"),
            "[package]\nname = \"guest\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write guest manifest");

        let resolved = resolve_project_dir_from(Some(Path::new("guest")), &invocation_cwd)
            .expect("resolve explicit project");

        assert_eq!(resolved, project_dir);
    }

    #[test]
    fn returns_hint_when_no_manifest_exists_in_ancestors() {
        let temp_dir = TempDir::new();
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
}
