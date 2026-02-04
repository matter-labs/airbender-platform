//! Manifest schema shared between build and host tooling.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const MANIFEST_FORMAT_VERSION: u32 = 1;

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
    pub format_version: u32,
    pub codec_version: u32,
    pub bin_name: String,
    pub target: Option<String>,
    pub profile: Profile,
    #[serde(alias = "app_bin")]
    pub bin_file: String,
    #[serde(alias = "app_elf")]
    pub elf_file: String,
    #[serde(alias = "app_text")]
    pub text_file: String,
    #[serde(default)]
    pub bin_sha256: String,
}

#[derive(Debug)]
pub enum ManifestError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    UnsupportedFormatVersion(u32),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io(err) => write!(f, "io error: {err}"),
            ManifestError::Parse(err) => write!(f, "failed to parse manifest: {err}"),
            ManifestError::Serialize(err) => write!(f, "failed to serialize manifest: {err}"),
            ManifestError::UnsupportedFormatVersion(version) => {
                write!(f, "unsupported format_version {version}")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<std::io::Error> for ManifestError {
    fn from(err: std::io::Error) -> Self {
        ManifestError::Io(err)
    }
}

pub fn read_manifest(path: &Path) -> Result<Manifest, ManifestError> {
    let content = fs::read_to_string(path)?;
    parse_manifest(&content)
}

pub fn write_manifest(path: &Path, manifest: &Manifest) -> Result<(), ManifestError> {
    let payload = manifest.to_toml()?;
    fs::write(path, payload)?;
    Ok(())
}

pub fn parse_manifest(content: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest = toml::from_str(content).map_err(ManifestError::Parse)?;
    if manifest.format_version != MANIFEST_FORMAT_VERSION {
        return Err(ManifestError::UnsupportedFormatVersion(
            manifest.format_version,
        ));
    }
    Ok(manifest)
}

impl Manifest {
    pub fn to_toml(&self) -> Result<String, ManifestError> {
        toml::to_string(self).map_err(ManifestError::Serialize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let manifest = Manifest {
            format_version: MANIFEST_FORMAT_VERSION,
            codec_version: 0,
            bin_name: "demo".to_string(),
            target: None,
            profile: Profile::Release,
            bin_file: "app.bin".to_string(),
            elf_file: "app.elf".to_string(),
            text_file: "app.text".to_string(),
            bin_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(),
        };
        let toml = manifest.to_toml().expect("serialize");
        assert!(toml.contains("bin_file"));
        assert!(toml.contains("elf_file"));
        assert!(toml.contains("text_file"));
        assert!(toml.contains("bin_sha256"));
        assert!(!toml.contains("app_bin"));
        assert!(!toml.contains("app_elf"));
        assert!(!toml.contains("app_text"));
        let parsed = parse_manifest(&toml).expect("parse");
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn rejects_unknown_format_version() {
        let mut manifest = Manifest {
            format_version: MANIFEST_FORMAT_VERSION,
            codec_version: 0,
            bin_name: "demo".to_string(),
            target: None,
            profile: Profile::Release,
            bin_file: "app.bin".to_string(),
            elf_file: "app.elf".to_string(),
            text_file: "app.text".to_string(),
            bin_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(),
        };
        manifest.format_version += 1;
        let toml = manifest.to_toml().expect("serialize");
        let err = parse_manifest(&toml).expect_err("error");
        assert!(matches!(err, ManifestError::UnsupportedFormatVersion(_)));
    }

    #[test]
    fn parses_legacy_artifact_field_names() {
        let legacy = r#"
format_version = 1
codec_version = 0
bin_name = "demo"
profile = "release"
app_bin = "app.bin"
app_elf = "app.elf"
app_text = "app.text"
"#;
        let manifest = parse_manifest(legacy).expect("parse legacy manifest");
        assert_eq!(manifest.bin_file, "app.bin");
        assert_eq!(manifest.elf_file, "app.elf");
        assert_eq!(manifest.text_file, "app.text");
        assert_eq!(manifest.bin_sha256, "");
    }
}
