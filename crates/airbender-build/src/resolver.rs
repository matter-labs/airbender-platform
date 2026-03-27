//! Resolution of [`BuildConfig`] inputs into fully-anchored [`ResolvedBuildParams`].

use crate::build::DistApp;
use crate::config::BuildConfig;
use crate::errors::{BuildError, Result};
use crate::metadata::CargoMetadata;
use crate::DEFAULT_GUEST_TARGET;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Fully resolved build inputs derived from [`BuildConfig`], the guest manifest, and git state.
///
/// All path and flag ambiguities are settled here; downstream steps operate only on these values.
#[derive(Debug)]
pub struct ResolvedBuildParams {
    /// Absolute path to the guest project root.
    pub project_dir: PathBuf,
    /// Cargo package name of the guest.
    pub package_name: String,
    /// Cargo binary target selected for this build.
    pub bin_name: String,
    /// Binary name written to `manifest.toml`; `None` when it equals `package_name`.
    pub manifest_bin_name: Option<String>,
    /// Target triple; defaults to [`DEFAULT_GUEST_TARGET`] when not explicitly set.
    pub target: String,
    /// Absolute paths to the dist app.
    pub dist_app: DistApp,
    /// Mount root used for reproducible builds.
    pub mount_root: PathBuf,
    /// Effective `panic_immediate_abort` flag after merging CLI and manifest settings.
    pub panic_immediate_abort: bool,
    /// Git metadata for the project.
    pub git: GitMetadata,
}

impl ResolvedBuildParams {
    /// Resolves all build inputs: validates config, loads the guest manifest, anchors paths,
    /// applies flag precedence, and reads git state.
    ///
    /// This is the single resolution point. All downstream steps use the returned params directly.
    pub fn resolve(build_config: &BuildConfig, cwd: &Path) -> Result<Self> {
        Self::validate_app_name(&build_config.app_name)?;

        let project_dir = Self::resolve_project_dir(build_config, cwd);
        let dist_dir = Self::resolve_dist_dir(build_config, &project_dir, cwd);
        let target = build_config
            .target
            .clone()
            .unwrap_or(DEFAULT_GUEST_TARGET.to_string());
        let project_metadata = CargoMetadata::load(&project_dir.join("Cargo.toml"))?;
        let bin_name = Self::resolve_bin_name(build_config, &project_metadata)?;
        let mount_root = Self::resolve_mount_root(build_config, &project_metadata, cwd);
        let panic_immediate_abort = project_metadata.panic_immediate_abort(build_config.profile);
        // Omit bin_name from the manifest when it matches the package name — the common case.
        // Downstream tooling treats an absent bin_name as identical to package_name.
        let manifest_bin_name =
            (bin_name != project_metadata.package_name).then(|| bin_name.clone());
        let git = GitMetadata::load(&project_dir);

        Ok(Self {
            project_dir,
            package_name: project_metadata.package_name,
            bin_name,
            manifest_bin_name,
            target,
            dist_app: DistApp::new(dist_dir),
            mount_root,
            panic_immediate_abort,
            git,
        })
    }

    fn resolve_project_dir(build_config: &BuildConfig, invocation_cwd: &Path) -> PathBuf {
        if build_config.project_dir.is_absolute() {
            build_config.project_dir.clone()
        } else {
            invocation_cwd.join(&build_config.project_dir)
        }
    }

    /// `--dist` follows standard CLI semantics: relative paths are interpreted
    /// from command invocation cwd, not from the guest project directory.
    fn resolve_dist_dir(
        build_config: &BuildConfig,
        project_dir: &Path,
        invocation_cwd: &Path,
    ) -> PathBuf {
        let dist_root = build_config.dist_dir.as_ref().map_or_else(
            || project_dir.join("dist"),
            |d| {
                if d.is_absolute() {
                    d.clone()
                } else {
                    invocation_cwd.join(d)
                }
            },
        );
        dist_root.join(&build_config.app_name)
    }

    /// Resolves the directory to bind-mounted as `/src` inside the reproducible build container.
    ///
    /// When `workspace_root_override` is set, that path is used directly (relative paths are
    /// resolved from invocation cwd, matching `--dist` semantics). Otherwise falls back to the
    /// cargo workspace root reported by `cargo metadata`, which is the guest directory itself for
    /// projects excluded from the top-level workspace.
    fn resolve_mount_root(
        build_config: &BuildConfig,
        metadata: &CargoMetadata,
        invocation_cwd: &Path,
    ) -> PathBuf {
        match &build_config.workspace_root_override {
            Some(p) if p.is_absolute() => p.clone(),
            Some(p) => invocation_cwd.join(p),
            None => metadata.workspace_root.to_path_buf(),
        }
    }

    /// Validates that `app_name` is exactly one normal path segment.
    ///
    /// This prevents accidental writes outside the dist root via separators,
    /// absolute prefixes, or path traversal segments.
    fn validate_app_name(app_name: &str) -> Result<()> {
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

    /// Resolves the binary target name for a guest project manifest.
    ///
    /// This enforces explicit selection when a package defines multiple binary targets.
    fn resolve_bin_name(build_config: &BuildConfig, metadata: &CargoMetadata) -> Result<String> {
        let package_name = &metadata.package_name;
        let bin_names = metadata
            .bin_targets
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>();
        if let Some(explicit_bin) = build_config.bin_name.as_deref() {
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

        match bin_names.as_slice() {
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
}

#[derive(Debug)]
pub struct GitMetadata {
    /// Git branch, or `"N/A"` if unavailable.
    pub branch: String,
    /// Git commit hash, or `"N/A"` if unavailable.
    pub commit: String,
    /// Whether the working tree has uncommitted changes.
    pub is_dirty: bool,
}

impl Default for GitMetadata {
    fn default() -> Self {
        Self {
            branch: "N/A".to_string(),
            commit: "N/A".to_string(),
            is_dirty: true,
        }
    }
}

impl GitMetadata {
    fn load(project_dir: &Path) -> Self {
        (|| {
            let branch = Self::run_git_stdout(project_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
            let commit = Self::run_git_stdout(project_dir, &["rev-parse", "HEAD"])?;
            let is_dirty = Self::has_unstaged_changes(project_dir)?;
            Some(Self {
                branch,
                commit,
                is_dirty,
            })
        })()
        .unwrap_or_default()
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
            None
        } else {
            Some(trimmed.to_string())
        }
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
            if line.len() >= 2 && line.as_bytes()[1] != b' ' {
                return Some(true);
            }
        }
        Some(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BuildConfig;

    fn fibonacci_project_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/fibonacci/guest")
    }

    fn write_minimal_project(dir: &Path) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"test-guest\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write Cargo.toml");
        std::fs::create_dir(dir.join("src")).expect("create src");
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").expect("write main.rs");
    }

    #[test]
    fn resolve_defaults() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_minimal_project(dir.path());
        let config = BuildConfig::new(dir.path());
        let params = ResolvedBuildParams::resolve(&config, dir.path()).expect("resolve");

        assert_eq!(params.package_name, "test-guest");
        assert_eq!(params.bin_name, "test-guest");
        assert_eq!(params.manifest_bin_name, None);
        assert_eq!(params.target, DEFAULT_GUEST_TARGET);
        assert!(!params.panic_immediate_abort);
        assert_eq!(params.dist_app.dir(), dir.path().join("dist/app"));
    }

    #[test]
    fn resolve_rejects_invalid_app_name() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_minimal_project(dir.path());
        let mut config = BuildConfig::new(dir.path());
        config.app_name = "foo/bar".to_string();
        let err = ResolvedBuildParams::resolve(&config, dir.path()).expect_err("invalid app name");
        assert!(matches!(err, crate::BuildError::InvalidConfig(_)));
    }

    #[test]
    fn resolve_panic_immediate_abort_reads_manifest() {
        let project_dir = fibonacci_project_dir();
        let config = BuildConfig::new(&project_dir);
        let params = ResolvedBuildParams::resolve(&config, &project_dir).expect("resolve");
        assert!(!params.panic_immediate_abort);
    }

    #[test]
    fn resolve_manifest_bin_name_omitted_when_matches_package() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_minimal_project(dir.path());
        let config = BuildConfig::new(dir.path());
        let params = ResolvedBuildParams::resolve(&config, dir.path()).expect("resolve");
        assert_eq!(params.manifest_bin_name, None);
    }

    #[test]
    fn resolve_manifest_bin_name_set_when_differs_from_package() {
        let dir = tempfile::tempdir().expect("create temp dir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-guest\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"other-bin\"\npath = \"src/main.rs\"\n",
        ).expect("write Cargo.toml");
        std::fs::create_dir(dir.path().join("src")).expect("create src");
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").expect("write main.rs");
        let mut config = BuildConfig::new(dir.path());
        config.bin_name = Some("other-bin".to_string());
        let params = ResolvedBuildParams::resolve(&config, dir.path()).expect("resolve");
        assert_eq!(params.manifest_bin_name, Some("other-bin".to_string()));
    }

    #[test]
    fn resolve_propagates_explicit_target() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_minimal_project(dir.path());
        let mut config = BuildConfig::new(dir.path());
        config.target = Some("riscv32im-risc0-custom-elf".to_string());
        let params = ResolvedBuildParams::resolve(&config, dir.path()).expect("resolve");
        assert_eq!(params.target, "riscv32im-risc0-custom-elf");
    }

    #[test]
    fn resolves_relative_project_dir_from_invocation_cwd() {
        let config = BuildConfig::new("examples/fibonacci/guest");
        let project_dir =
            ResolvedBuildParams::resolve_project_dir(&config, Path::new("/workspace/repo"));
        assert_eq!(
            project_dir,
            PathBuf::from("/workspace/repo/examples/fibonacci/guest")
        );
    }

    #[test]
    fn resolves_absolute_project_dir_without_rebasing() {
        let config = BuildConfig::new("/workspace/project");
        let project_dir =
            ResolvedBuildParams::resolve_project_dir(&config, Path::new("/other/cwd"));
        assert_eq!(project_dir, PathBuf::from("/workspace/project"));
    }

    #[test]
    fn resolves_default_dist_dir_under_project_root() {
        let mut config = BuildConfig::new("/workspace/project");
        config.app_name = "gpu-profile".to_string();
        let dist_dir = ResolvedBuildParams::resolve_dist_dir(
            &config,
            Path::new("/workspace/project"),
            Path::new("/workspace/caller"),
        );
        assert_eq!(
            dist_dir,
            PathBuf::from("/workspace/project/dist/gpu-profile")
        );
    }

    #[test]
    fn resolves_custom_relative_dist_root_from_invocation_cwd() {
        let mut config = BuildConfig::new("/workspace/project");
        config.app_name = "gpu-profile".to_string();
        config.dist_dir = Some(PathBuf::from("builds"));
        let dist_dir = ResolvedBuildParams::resolve_dist_dir(
            &config,
            Path::new("/workspace/project"),
            Path::new("/workspace/caller"),
        );
        assert_eq!(
            dist_dir,
            PathBuf::from("/workspace/caller/builds/gpu-profile")
        );
    }

    #[test]
    fn resolves_custom_absolute_dist_root_without_rebasing() {
        let mut config = BuildConfig::new("/workspace/project");
        config.app_name = "gpu-profile".to_string();
        config.dist_dir = Some(PathBuf::from("/workspace/builds"));
        let dist_dir = ResolvedBuildParams::resolve_dist_dir(
            &config,
            Path::new("/workspace/project"),
            Path::new("/workspace/caller"),
        );
        assert_eq!(dist_dir, PathBuf::from("/workspace/builds/gpu-profile"));
    }

    #[test]
    fn git_metadata_falls_back_when_unavailable() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let metadata = GitMetadata::load(dir.path());
        assert_eq!(metadata.branch, "N/A");
        assert_eq!(metadata.commit, "N/A");
        assert!(metadata.is_dirty);
    }

    // --- validate_app_name ---

    #[test]
    fn validate_app_name_accepts_simple_name() {
        assert!(ResolvedBuildParams::validate_app_name("app").is_ok());
        assert!(ResolvedBuildParams::validate_app_name("my-app").is_ok());
        assert!(ResolvedBuildParams::validate_app_name("my_app_123").is_ok());
    }

    #[test]
    fn validate_app_name_rejects_empty() {
        let err = ResolvedBuildParams::validate_app_name("").expect_err("empty name must fail");
        assert!(matches!(err, crate::BuildError::InvalidConfig(_)));
    }

    #[test]
    fn validate_app_name_rejects_slash() {
        let err = ResolvedBuildParams::validate_app_name("foo/bar").expect_err("slash must fail");
        assert!(matches!(err, crate::BuildError::InvalidConfig(_)));
    }

    #[test]
    fn validate_app_name_rejects_backslash() {
        let err =
            ResolvedBuildParams::validate_app_name("foo\\bar").expect_err("backslash must fail");
        assert!(matches!(err, crate::BuildError::InvalidConfig(_)));
    }

    #[test]
    fn validate_app_name_rejects_dotdot() {
        let err = ResolvedBuildParams::validate_app_name("..").expect_err("dotdot must fail");
        assert!(matches!(err, crate::BuildError::InvalidConfig(_)));
    }

    // --- resolve_bin_name ---

    fn make_metadata(package_name: &str, bin_targets: &[&str]) -> CargoMetadata {
        CargoMetadata {
            package_name: package_name.to_string(),
            bin_targets: bin_targets.iter().map(|s| s.to_string()).collect(),
            airbender: Default::default(),
            workspace_root: PathBuf::new(),
        }
    }

    #[test]
    fn resolve_bin_name_uses_single_candidate_without_explicit_override() {
        let config = BuildConfig::new(".");
        let resolved =
            ResolvedBuildParams::resolve_bin_name(&config, &make_metadata("guest", &["guest"]))
                .expect("single binary should resolve");
        assert_eq!(resolved, "guest");
    }

    #[test]
    fn resolve_bin_name_rejects_ambiguous_multi_bin_without_explicit_override() {
        let config = BuildConfig::new(".");
        let err = ResolvedBuildParams::resolve_bin_name(
            &config,
            &make_metadata("guest", &["alpha", "beta"]),
        )
        .expect_err("multi-bin package should require --bin");
        assert_eq!(
            err.to_string(),
            "invalid config: package `guest` has multiple binary targets (alpha, beta); pass `--bin <name>`"
        );
    }

    #[test]
    fn resolve_bin_name_validates_explicit_bin_against_candidates() {
        let mut config = BuildConfig::new(".");
        config.bin_name = Some("gamma".to_string());
        let err = ResolvedBuildParams::resolve_bin_name(
            &config,
            &make_metadata("guest", &["alpha", "beta"]),
        )
        .expect_err("unknown explicit bin must fail fast");
        assert_eq!(
            err.to_string(),
            "invalid config: binary target `gamma` not found in package `guest`; available binaries: alpha, beta"
        );
    }

    #[test]
    fn resolve_bin_name_accepts_explicit_candidate() {
        let mut config = BuildConfig::new(".");
        config.bin_name = Some("beta".to_string());
        let resolved = ResolvedBuildParams::resolve_bin_name(
            &config,
            &make_metadata("guest", &["alpha", "beta"]),
        )
        .expect("known explicit bin should resolve");
        assert_eq!(resolved, "beta");
    }

    #[test]
    fn resolve_bin_name_rejects_packages_without_binaries() {
        let config = BuildConfig::new(".");
        let err = ResolvedBuildParams::resolve_bin_name(&config, &make_metadata("guest", &[]))
            .expect_err("package without binaries should fail");
        assert_eq!(
            err.to_string(),
            "invalid config: package `guest` has no binary targets"
        );
    }
}
