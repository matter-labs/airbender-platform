//! Build configuration and artifact packaging flow.

use crate::constants::DEFAULT_APP_NAME;
use crate::errors::Result;
use crate::utils::{
    find_package, load_metadata, run_command, select_bin_name, sha256_file_hex, validate_app_name,
};
use crate::{Manifest, Profile, MANIFEST_FORMAT_VERSION};
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
        }
    }

    /// Builds the guest binary and writes a dist package for this configuration.
    fn build_dist(&self) -> Result<DistArtifacts> {
        let invocation_cwd = std::env::current_dir()?;
        let project_dir = self.resolve_project_dir(&invocation_cwd);

        let app_name = self.resolve_app_name()?;
        let bin_name = self.resolve_bin_name(&project_dir)?;
        let target = self.resolve_target()?;
        let dist_dir = self.resolve_dist_dir(&app_name, &project_dir, &invocation_cwd);
        fs::create_dir_all(&dist_dir)?;

        self.run_cargo_build(&project_dir, &bin_name, target.as_deref())?;

        let app_bin = dist_dir.join("app.bin");
        let app_elf = dist_dir.join("app.elf");
        let app_text = dist_dir.join("app.text");

        self.run_cargo_objcopy(
            &project_dir,
            &bin_name,
            target.as_deref(),
            &["-O", "binary"],
            &app_bin,
        )?;
        self.run_cargo_objcopy(
            &project_dir,
            &bin_name,
            target.as_deref(),
            &["-R", ".text"],
            &app_elf,
        )?;
        self.run_cargo_objcopy(
            &project_dir,
            &bin_name,
            target.as_deref(),
            &["-O", "binary", "--only-section=.text"],
            &app_text,
        )?;

        let bin_sha256 = sha256_file_hex(&app_bin)?;

        let manifest_path = dist_dir.join("manifest.toml");
        let manifest = Manifest {
            format_version: MANIFEST_FORMAT_VERSION,
            codec_version: airbender_codec::AIRBENDER_CODEC_V0,
            bin_name,
            target,
            profile: self.profile,
            bin_file: "app.bin".to_string(),
            elf_file: "app.elf".to_string(),
            text_file: "app.text".to_string(),
            bin_sha256,
        };
        manifest.write_to_file(&manifest_path)?;

        Ok(DistArtifacts {
            dist_dir,
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

        cmd.args(&self.cargo_args);
        cmd.arg("--");
        cmd.args(objcopy_args);
        cmd.arg(output);
        cmd.current_dir(project_dir);
        run_command(cmd, "cargo objcopy")
    }

    /// Resolves the binary name from config or Cargo metadata.
    fn resolve_bin_name(&self, project_dir: &Path) -> Result<String> {
        if let Some(bin) = &self.bin_name {
            return Ok(bin.clone());
        }

        let manifest_path = project_dir.join("Cargo.toml");
        let metadata = load_metadata(&manifest_path)?;
        let package = find_package(&metadata, &manifest_path)?;
        Ok(select_bin_name(package))
    }

    /// Validates and returns the configured app name.
    fn resolve_app_name(&self) -> Result<String> {
        validate_app_name(&self.app_name)?;
        Ok(self.app_name.clone())
    }

    /// Resolves the build target, using explicit override when provided.
    fn resolve_target(&self) -> Result<Option<String>> {
        Ok(self.target.clone())
    }

    /// Resolves the project directory relative to the command invocation cwd.
    fn resolve_project_dir(&self, invocation_cwd: &Path) -> PathBuf {
        if self.project_dir.is_absolute() {
            self.project_dir.clone()
        } else {
            invocation_cwd.join(&self.project_dir)
        }
    }

    /// Resolves the final dist directory for this app configuration.
    ///
    /// `--dist` follows standard CLI semantics: relative paths are interpreted
    /// from command invocation cwd, not from the guest project directory.
    fn resolve_dist_dir(
        &self,
        app_name: &str,
        project_dir: &Path,
        invocation_cwd: &Path,
    ) -> PathBuf {
        let dist_root = self.dist_dir.clone().map_or_else(
            || project_dir.join("dist"),
            |dist_dir| {
                if dist_dir.is_absolute() {
                    dist_dir
                } else {
                    invocation_cwd.join(dist_dir)
                }
            },
        );
        dist_root.join(app_name)
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
    fn defaults_to_no_target_override() {
        let config = BuildConfig::new(PathBuf::from("."));
        let target = config.resolve_target().expect("target resolution");
        assert_eq!(target, None);
    }

    #[test]
    fn accepts_explicit_target_override() {
        let mut config = BuildConfig::new(PathBuf::from("."));
        config.target = Some("riscv32im-risc0-zkvm-elf".to_string());
        let target = config.resolve_target().expect("target resolution");
        assert_eq!(target.as_deref(), Some("riscv32im-risc0-zkvm-elf"));
    }

    #[test]
    fn defaults_to_app_name() {
        let config = BuildConfig::new(PathBuf::from("."));
        let app_name = config.resolve_app_name().expect("app-name resolution");
        assert_eq!(app_name, DEFAULT_APP_NAME);
    }

    #[test]
    fn accepts_custom_app_name() {
        let mut config = BuildConfig::new(PathBuf::from("."));
        config.app_name = "gpu-profile".to_string();
        let app_name = config.resolve_app_name().expect("app-name resolution");
        assert_eq!(app_name, "gpu-profile");
    }

    #[test]
    fn rejects_nested_app_name() {
        let mut config = BuildConfig::new(PathBuf::from("."));
        config.app_name = "profiles/gpu".to_string();
        let err = config.resolve_app_name().expect_err("invalid app-name");
        assert!(matches!(err, crate::BuildError::InvalidConfig(_)));
    }

    #[test]
    fn resolves_default_dist_dir_under_project_root() {
        let mut config = BuildConfig::new(PathBuf::from("/workspace/project"));
        config.app_name = "gpu-profile".to_string();
        let app_name = config.resolve_app_name().expect("app-name resolution");
        let invocation_cwd = Path::new("/workspace/caller");
        let project_dir = config.resolve_project_dir(invocation_cwd);
        let dist_dir = config.resolve_dist_dir(&app_name, &project_dir, invocation_cwd);
        assert_eq!(
            dist_dir,
            PathBuf::from("/workspace/project/dist/gpu-profile")
        );
    }

    #[test]
    fn resolves_custom_relative_dist_root_from_invocation_cwd() {
        let mut config = BuildConfig::new(PathBuf::from("/workspace/project"));
        config.dist_dir = Some(PathBuf::from("builds"));
        config.app_name = "gpu-profile".to_string();
        let app_name = config.resolve_app_name().expect("app-name resolution");
        let invocation_cwd = Path::new("/workspace/caller");
        let project_dir = config.resolve_project_dir(invocation_cwd);
        let dist_dir = config.resolve_dist_dir(&app_name, &project_dir, invocation_cwd);
        assert_eq!(
            dist_dir,
            PathBuf::from("/workspace/caller/builds/gpu-profile")
        );
    }

    #[test]
    fn resolves_relative_project_dir_from_invocation_cwd() {
        let config = BuildConfig::new(PathBuf::from("examples/fibonacci/guest"));
        let invocation_cwd = Path::new("/workspace/repo");
        let project_dir = config.resolve_project_dir(invocation_cwd);

        assert_eq!(
            project_dir,
            PathBuf::from("/workspace/repo/examples/fibonacci/guest")
        );
    }

    #[test]
    fn resolves_custom_absolute_dist_root_without_rebasing() {
        let mut config = BuildConfig::new(PathBuf::from("/workspace/project"));
        config.dist_dir = Some(PathBuf::from("/workspace/builds"));
        config.app_name = "gpu-profile".to_string();
        let app_name = config.resolve_app_name().expect("app-name resolution");
        let invocation_cwd = Path::new("/workspace/caller");
        let project_dir = config.resolve_project_dir(invocation_cwd);
        let dist_dir = config.resolve_dist_dir(&app_name, &project_dir, invocation_cwd);

        assert_eq!(dist_dir, PathBuf::from("/workspace/builds/gpu-profile"));
    }
}
