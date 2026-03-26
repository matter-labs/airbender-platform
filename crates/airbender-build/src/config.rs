//! Build configuration and artifact packaging flow.

use crate::constants::DEFAULT_APP_NAME;
use crate::docker::ReproducibleBuild;
use crate::errors::Result;
use crate::utils::{run_command, sha256_file_hex};
use crate::{ArtifactEntry, BuildMetadata, Manifest, Profile, MANIFEST_VERSION_V1};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Input settings for guest compilation and dist packaging.
#[derive(Clone, Debug)]
pub struct BuildConfig {
    /// Project root containing `Cargo.toml`.
    pub project_dir: PathBuf,
    /// Output app folder name inside the dist root.
    pub app_name: String,
    /// Override for the binary name.
    pub bin_name: Option<String>,
    /// Override for the target triple.
    pub target: Option<String>,
    /// Build profile used for artifact extraction.
    pub profile: Profile,
    /// Output root directory for `dist/` artifacts.
    pub dist_dir: Option<PathBuf>,
    /// Additional arguments forwarded to `cargo build` and `cargo objcopy`.
    pub cargo_args: Vec<String>,
    /// When true, compilation runs inside a pinned Docker container for
    /// bit-for-bit reproducible output across host environments.
    pub reproducible: bool,
    /// Overrides the directory bind-mounted as `/src` inside the reproducible
    /// build container. Only needed when the guest has path dependencies that
    /// point outside its own cargo workspace root (e.g. in-tree monorepos where
    /// the guest shares crates with the host via `path = "../../.."`).
    /// Has no effect unless `reproducible` is also true.
    pub workspace_root_override: Option<PathBuf>,
}

impl BuildConfig {
    /// Creates a config with defaults for app name, profile, and dist root.
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
            app_name: DEFAULT_APP_NAME.to_string(),
            bin_name: None,
            target: None,
            profile: Profile::Release,
            dist_dir: None,
            cargo_args: Vec::new(),
            reproducible: false,
            workspace_root_override: None,
        }
    }

    /// Builds the guest binary and writes a dist package for this configuration.
    fn build_dist(&self) -> Result<DistArtifacts> {
        let cwd = std::env::current_dir()?;
        let params = self.resolve(&cwd)?;

        // Build extra config for `panic_immediate_abort`.
        // The same --config must be passed to both build and objcopy to prevent
        // a fall back to a cached artifact built without it.
        let extra_config = params
            .panic_immediate_abort
            .then_some(r#"build.rustflags=["-Zunstable-options","-Cpanic=immediate-abort"]"#);

        fs::create_dir_all(&params.dist_dir)?;

        let app_bin = params.dist_dir.join("app.bin");
        let app_elf = params.dist_dir.join("app.elf");
        let app_text = params.dist_dir.join("app.text");

        if self.reproducible {
            let mount_root = self.resolve_mount_root(&params.workspace_root, &cwd);
            ReproducibleBuild::new(
                &params.project_dir,
                &mount_root,
                &params.bin_name,
                params.target.as_deref(),
                self.profile,
                &self.cargo_args,
            )?
            .run(&params.dist_dir)?;
        } else {
            self.run_cargo_build(
                &params.project_dir,
                &params.bin_name,
                params.target.as_deref(),
                extra_config,
            )?;
            self.run_cargo_objcopy(
                &params.project_dir,
                &params.bin_name,
                params.target.as_deref(),
                extra_config,
                &["-O", "binary"],
                &app_bin,
            )?;
            self.run_cargo_objcopy(
                &params.project_dir,
                &params.bin_name,
                params.target.as_deref(),
                extra_config,
                &["-R", ".text"],
                &app_elf,
            )?;
            self.run_cargo_objcopy(
                &params.project_dir,
                &params.bin_name,
                params.target.as_deref(),
                extra_config,
                &["-O", "binary", "--only-section=.text"],
                &app_text,
            )?;
        }

        let bin_sha256 = sha256_file_hex(&app_bin)?;
        let elf_sha256 = sha256_file_hex(&app_elf)?;
        let text_sha256 = sha256_file_hex(&app_text)?;

        let manifest_path = params.dist_dir.join("manifest.toml");
        let manifest = Manifest {
            package: params.package_name,
            bin_name: params.manifest_bin_name,
            manifest: MANIFEST_VERSION_V1.to_string(),
            codec: format!("v{}", airbender_codec::AIRBENDER_CODEC_V0),
            target: params.target,
            bin: ArtifactEntry {
                path: "app.bin".to_string(),
                sha256: bin_sha256,
            },
            elf: ArtifactEntry {
                path: "app.elf".to_string(),
                sha256: elf_sha256,
            },
            text: ArtifactEntry {
                path: "app.text".to_string(),
                sha256: text_sha256,
            },
            build: BuildMetadata {
                profile: self.profile,
                reproducible: self.reproducible,
                git_branch: params.git_branch,
                git_commit: params.git_commit,
                is_dirty: params.git_is_dirty,
            },
        };
        manifest.write_to_file(&manifest_path)?;

        Ok(DistArtifacts {
            dist_dir: params.dist_dir,
            app_bin,
            app_elf,
            app_text,
            manifest: manifest_path,
        })
    }

    /// Runs `cargo build` using this config and optional target override.
    fn run_cargo_build(
        &self,
        project_dir: &Path,
        bin_name: &str,
        target: Option<&str>,
        extra_config: Option<&str>,
    ) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        if self.profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("--bin").arg(bin_name);
        if let Some(target) = target {
            cmd.arg("--target").arg(target);
        }
        if let Some(cfg) = extra_config {
            cmd.arg("--config").arg(cfg);
        }
        cmd.args(&self.cargo_args);
        cmd.current_dir(project_dir);
        run_command(cmd, "cargo build")
    }

    /// Runs `cargo objcopy` to generate one concrete output artifact.
    fn run_cargo_objcopy(
        &self,
        project_dir: &Path,
        bin_name: &str,
        target: Option<&str>,
        extra_config: Option<&str>,
        objcopy_args: &[&str],
        output: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.arg("objcopy");
        if self.profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("--bin").arg(bin_name);
        if let Some(target) = target {
            cmd.arg("--target").arg(target);
        }
        if let Some(cfg) = extra_config {
            cmd.arg("--config").arg(cfg);
        }
        cmd.args(&self.cargo_args);
        cmd.arg("--");
        cmd.args(objcopy_args);
        cmd.arg(output);
        cmd.current_dir(project_dir);
        run_command(cmd, "cargo objcopy")
    }

    /// Resolves the directory to bind-mounted as `/src` inside the reproducible build container.
    ///
    /// When `workspace_root_override` is set, that path is used directly (relative paths are
    /// resolved from invocation cwd, matching `--dist` semantics). Otherwise falls back to the
    /// cargo workspace root reported by `cargo metadata`, which is the guest directory itself for
    /// projects excluded from the top-level workspace.
    fn resolve_mount_root(&self, cargo_workspace_root: &Path, invocation_cwd: &Path) -> PathBuf {
        match &self.workspace_root_override {
            Some(p) if p.is_absolute() => p.clone(),
            Some(p) => invocation_cwd.join(p),
            None => cargo_workspace_root.to_path_buf(),
        }
    }
}

/// Output paths produced by one successful build/package invocation.
#[derive(Clone, Debug)]
pub struct DistArtifacts {
    /// Dist app directory used for this build.
    pub dist_dir: PathBuf,
    /// Path to `app.bin`.
    pub app_bin: PathBuf,
    /// Path to `app.elf`.
    pub app_elf: PathBuf,
    /// Path to `app.text`.
    pub app_text: PathBuf,
    /// Path to `manifest.toml`.
    pub manifest: PathBuf,
}

/// Builds and packages guest artifacts using the provided configuration.
pub fn build_dist(config: &BuildConfig) -> Result<DistArtifacts> {
    config.build_dist()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproducible_flag_defaults_to_false() {
        let config = BuildConfig::new(PathBuf::from("."));
        assert!(!config.reproducible);
    }

    #[test]
    fn workspace_root_override_defaults_to_none() {
        let config = BuildConfig::new(PathBuf::from("."));
        assert!(config.workspace_root_override.is_none());
    }

    #[test]
    fn mount_root_falls_back_to_cargo_workspace_root_when_override_absent() {
        let config = BuildConfig::new(PathBuf::from("."));
        let cargo_root = Path::new("/workspace/project/guest");
        let invocation_cwd = Path::new("/workspace/caller");
        let mount_root = config.resolve_mount_root(cargo_root, invocation_cwd);
        assert_eq!(mount_root, PathBuf::from("/workspace/project/guest"));
    }

    #[test]
    fn mount_root_uses_absolute_override_without_rebasing() {
        let mut config = BuildConfig::new(PathBuf::from("."));
        config.workspace_root_override = Some(PathBuf::from("/repo"));
        let cargo_root = Path::new("/workspace/project/guest");
        let invocation_cwd = Path::new("/workspace/caller");
        let mount_root = config.resolve_mount_root(cargo_root, invocation_cwd);
        assert_eq!(mount_root, PathBuf::from("/repo"));
    }

    #[test]
    fn mount_root_resolves_relative_override_from_invocation_cwd() {
        let mut config = BuildConfig::new(PathBuf::from("."));
        config.workspace_root_override = Some(PathBuf::from("."));
        let cargo_root = Path::new("/workspace/project/guest");
        let invocation_cwd = Path::new("/workspace/caller");
        let mount_root = config.resolve_mount_root(cargo_root, invocation_cwd);
        assert_eq!(mount_root, PathBuf::from("/workspace/caller"));
    }
}
