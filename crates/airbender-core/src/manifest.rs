//! Manifest schema shared between build and host tooling.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const MANIFEST_VERSION_V1: &str = "v1";
pub const CODEC_VERSION_V0: &str = "v0";

/// Build profile recorded in the manifest for reproducibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

/// Serialized manifest describing the build artifacts for a guest program.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Cargo package identity this dist bundle comes from.
    pub package: String,
    /// Optional binary target name when it differs from `package`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin_name: Option<String>,
    /// Manifest schema version for compatibility checks.
    pub manifest: String,
    /// Host/guest codec version used to encode runtime payloads.
    pub codec: String,
    /// Optional target triple used for the build.
    pub target: Option<String>,
    /// Binary image consumed by runtime and proving flows.
    pub bin: ArtifactEntry,
    /// ELF image used for symbol/debug workflows.
    pub elf: ArtifactEntry,
    /// Text-section image used by transpiler execution.
    pub text: ArtifactEntry,
    /// Build provenance metadata captured at packaging time.
    pub build: BuildMetadata,
}

/// One artifact entry recorded in the manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactEntry {
    /// Artifact path relative to the dist directory.
    pub path: String,
    /// SHA-256 digest for artifact integrity verification.
    pub sha256: String,
}

/// Build metadata captured while creating dist artifacts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildMetadata {
    /// Cargo build profile used to produce artifacts.
    pub profile: Profile,
    /// Git branch name at build time, or `N/A` if unavailable.
    pub git_branch: String,
    /// Git commit hash at build time, or `N/A` if unavailable.
    pub git_commit: String,
    /// Indicates unstaged changes at build time.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_dirty: bool,
}

/// Errors returned by manifest read, write, and parse operations.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse manifest: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize manifest: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("unsupported manifest version `{0}`")]
    UnsupportedManifestVersion(String),
}

impl Manifest {
    /// Read a manifest from a TOML file.
    pub fn read_from_file(path: &Path) -> Result<Self, ManifestError> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Write this manifest to a TOML file.
    pub fn write_to_file(&self, path: &Path) -> Result<(), ManifestError> {
        let payload = self.to_toml()?;
        fs::write(path, payload)?;
        Ok(())
    }

    /// Parse and validate a manifest from TOML text.
    pub fn parse(content: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(content)?;
        if manifest.manifest != MANIFEST_VERSION_V1 {
            return Err(ManifestError::UnsupportedManifestVersion(manifest.manifest));
        }
        Ok(manifest)
    }

    /// Serialize this manifest to TOML text.
    pub fn to_toml(&self) -> Result<String, ManifestError> {
        Ok(toml::to_string(self)?)
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let manifest = Manifest {
            package: "demo".to_string(),
            bin_name: None,
            manifest: MANIFEST_VERSION_V1.to_string(),
            codec: CODEC_VERSION_V0.to_string(),
            target: None,
            bin: ArtifactEntry {
                path: "app.bin".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            },
            elf: ArtifactEntry {
                path: "app.elf".to_string(),
                sha256: "b8f1d1d6b064577aa66013024e69c0dcde721573ae58da439b84e1c862437288"
                    .to_string(),
            },
            text: ArtifactEntry {
                path: "app.text".to_string(),
                sha256: "0476bd0bb997b387b72f721b3f0f38f112e43e32f151e1d1f2ec20bc7c5ad5a6"
                    .to_string(),
            },
            build: BuildMetadata {
                profile: Profile::Release,
                git_branch: "main".to_string(),
                git_commit: "abc123".to_string(),
                is_dirty: false,
            },
        };
        let toml = manifest.to_toml().expect("serialize");
        let first_line = toml
            .lines()
            .find(|line| !line.trim().is_empty())
            .expect("manifest must have at least one line");
        assert_eq!(first_line, "package = \"demo\"");
        assert!(!toml.contains("bin_name"));
        assert!(toml.contains("[bin]"));
        assert!(toml.contains("[elf]"));
        assert!(toml.contains("[text]"));
        assert!(toml.contains("[build]"));
        assert!(!toml.contains("is_dirty"));
        let parsed = Manifest::parse(&toml).expect("parse");
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn includes_dirty_flag_when_true() {
        let manifest = Manifest {
            package: "demo".to_string(),
            bin_name: None,
            manifest: MANIFEST_VERSION_V1.to_string(),
            codec: CODEC_VERSION_V0.to_string(),
            target: None,
            bin: ArtifactEntry {
                path: "app.bin".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            },
            elf: ArtifactEntry {
                path: "app.elf".to_string(),
                sha256: "b8f1d1d6b064577aa66013024e69c0dcde721573ae58da439b84e1c862437288"
                    .to_string(),
            },
            text: ArtifactEntry {
                path: "app.text".to_string(),
                sha256: "0476bd0bb997b387b72f721b3f0f38f112e43e32f151e1d1f2ec20bc7c5ad5a6"
                    .to_string(),
            },
            build: BuildMetadata {
                profile: Profile::Release,
                git_branch: "main".to_string(),
                git_commit: "abc123".to_string(),
                is_dirty: true,
            },
        };

        let toml = manifest.to_toml().expect("serialize");
        assert!(toml.contains("is_dirty = true"));
    }

    #[test]
    fn includes_bin_name_when_present() {
        let manifest = Manifest {
            package: "demo".to_string(),
            bin_name: Some("worker".to_string()),
            manifest: MANIFEST_VERSION_V1.to_string(),
            codec: CODEC_VERSION_V0.to_string(),
            target: None,
            bin: ArtifactEntry {
                path: "app.bin".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            },
            elf: ArtifactEntry {
                path: "app.elf".to_string(),
                sha256: "b8f1d1d6b064577aa66013024e69c0dcde721573ae58da439b84e1c862437288"
                    .to_string(),
            },
            text: ArtifactEntry {
                path: "app.text".to_string(),
                sha256: "0476bd0bb997b387b72f721b3f0f38f112e43e32f151e1d1f2ec20bc7c5ad5a6"
                    .to_string(),
            },
            build: BuildMetadata {
                profile: Profile::Release,
                git_branch: "main".to_string(),
                git_commit: "abc123".to_string(),
                is_dirty: false,
            },
        };

        let toml = manifest.to_toml().expect("serialize");
        assert!(toml.contains("bin_name = \"worker\""));
        let parsed = Manifest::parse(&toml).expect("parse");
        assert_eq!(parsed.bin_name.as_deref(), Some("worker"));
    }

    #[test]
    fn rejects_unknown_manifest_version() {
        let mut manifest = Manifest {
            package: "demo".to_string(),
            bin_name: None,
            manifest: MANIFEST_VERSION_V1.to_string(),
            codec: CODEC_VERSION_V0.to_string(),
            target: None,
            bin: ArtifactEntry {
                path: "app.bin".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            },
            elf: ArtifactEntry {
                path: "app.elf".to_string(),
                sha256: "b8f1d1d6b064577aa66013024e69c0dcde721573ae58da439b84e1c862437288"
                    .to_string(),
            },
            text: ArtifactEntry {
                path: "app.text".to_string(),
                sha256: "0476bd0bb997b387b72f721b3f0f38f112e43e32f151e1d1f2ec20bc7c5ad5a6"
                    .to_string(),
            },
            build: BuildMetadata {
                profile: Profile::Release,
                git_branch: "main".to_string(),
                git_commit: "abc123".to_string(),
                is_dirty: false,
            },
        };
        manifest.manifest = "v2".to_string();
        let toml = manifest.to_toml().expect("serialize");
        let err = Manifest::parse(&toml).expect_err("error");
        assert!(matches!(err, ManifestError::UnsupportedManifestVersion(_)));
    }
}
