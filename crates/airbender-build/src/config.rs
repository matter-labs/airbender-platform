//! Build configuration and artifact packaging flow.

use crate::build::ReproducibleBuild;
use crate::build::{DistArtifact, DistArtifacts, LocalBuild};
use crate::constants::DEFAULT_APP_NAME;
use crate::errors::Result;
use crate::resolver::ResolvedBuildParams;
use crate::{ArtifactEntry, BuildMetadata, Manifest, Profile, MANIFEST_VERSION_V1};
use std::fs;
use std::path::{Path, PathBuf};

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
        let params = ResolvedBuildParams::resolve(self, &cwd)?;

        // Build extra config for `panic_immediate_abort`.
        // The same --config must be passed to both build and objcopy to prevent
        // a fall back to a cached artifact built without it.
        let extra_config = params
            .panic_immediate_abort
            .then_some(r#"build.rustflags=["-Zunstable-options","-Cpanic=immediate-abort"]"#);

        fs::create_dir_all(params.dist_app.dir())?;

        if self.reproducible {
            ReproducibleBuild::new(&params)?.run(self.profile, &self.cargo_args, extra_config)?;
        } else {
            LocalBuild::new(&params).run(self.profile, &self.cargo_args, extra_config)?;
        }

        let artifacts = DistArtifacts {
            dir: params.dist_app.dir().to_path_buf(),
            app_bin: DistArtifact::new(params.dist_app.bin().to_path_buf())?,
            app_elf: DistArtifact::new(params.dist_app.elf().to_path_buf())?,
            app_text: DistArtifact::new(params.dist_app.text().to_path_buf())?,
            manifest_path: params.dist_app.manifest().to_path_buf(),
        };

        fn file_name(p: &Path) -> String {
            p.file_name()
                .expect("must be valid")
                .to_str()
                .expect("must be valid")
                .to_string()
        }
        let manifest = Manifest {
            package: params.package_name,
            bin_name: params.manifest_bin_name,
            manifest: MANIFEST_VERSION_V1.to_string(),
            codec: format!("v{}", airbender_codec::AIRBENDER_CODEC_V0),
            target: params.target,
            bin: ArtifactEntry {
                path: file_name(params.dist_app.bin()),
                sha256: artifacts.app_bin.sha256.clone(),
            },
            elf: ArtifactEntry {
                path: file_name(params.dist_app.elf()),
                sha256: artifacts.app_elf.sha256.clone(),
            },
            text: ArtifactEntry {
                path: file_name(params.dist_app.text()),
                sha256: artifacts.app_text.sha256.clone(),
            },
            build: BuildMetadata {
                profile: self.profile,
                reproducible: self.reproducible,
                git_branch: params.git.branch,
                git_commit: params.git.commit,
                is_dirty: params.git.is_dirty,
            },
        };
        manifest.write_to_file(params.dist_app.manifest())?;

        Ok(artifacts)
    }
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
}
