//! Typed view of the guest project's `Cargo.toml` loaded from `cargo metadata`.

use crate::errors::{BuildError, Result};
use crate::Profile;
use cargo_metadata::{Metadata, MetadataCommand, Package};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Combined cargo and airbender metadata for a guest project, loaded in a single
/// `cargo metadata` invocation.
pub struct ProjectManifest {
    /// Cargo package name.
    pub package_name: String,
    /// Binary target names declared by the package.
    pub bin_targets: Vec<String>,
    /// Typed `[package.metadata.airbender]` settings, defaulting to empty if absent.
    pub airbender: AirbenderConfig,
}

/// Contents of `[package.metadata.airbender]` in the guest `Cargo.toml`.
#[derive(Deserialize, Default)]
pub struct AirbenderConfig {
    /// Per-profile build settings, keyed by profile name (`"debug"`, `"release"`).
    #[serde(default)]
    pub profile: HashMap<String, AirbenderProfileConfig>,
}

/// Per-profile airbender build settings under
/// `[package.metadata.airbender.profile.<name>]`.
#[derive(Deserialize, Default)]
pub struct AirbenderProfileConfig {
    /// Enable `panic_immediate_abort` build-std feature for this profile.
    #[serde(default, rename = "panic-immediate-abort")]
    pub panic_immediate_abort: bool,
}

fn load_metadata(manifest_path: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .map_err(|err| BuildError::InvalidConfig(format!("cargo metadata failed: {err}")))
}

fn find_package<'a>(metadata: &'a Metadata, manifest_path: &Path) -> Result<&'a Package> {
    let manifest_path = manifest_path.canonicalize()?;
    let manifest_path =
        cargo_metadata::camino::Utf8PathBuf::from_path_buf(manifest_path).map_err(|path| {
            BuildError::InvalidConfig(format!(
                "manifest path is not valid UTF-8: {}",
                path.display()
            ))
        })?;

    if let Some(pkg) = metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path == manifest_path)
    {
        return Ok(pkg);
    }

    metadata
        .root_package()
        .ok_or(BuildError::MissingField("package.name"))
}

impl ProjectManifest {
    /// Loads the guest project manifest from the `Cargo.toml` at `manifest_path`.
    ///
    /// Calls `cargo metadata` once and deserializes both cargo fields and
    /// `[package.metadata.airbender]` settings. Unknown airbender keys are ignored.
    pub fn load(manifest_path: &Path) -> Result<Self> {
        let metadata = load_metadata(manifest_path)?;
        let package = find_package(&metadata, manifest_path)?;
        let bin_targets = package
            .targets
            .iter()
            .filter(|t| t.kind.iter().any(|k| k == "bin"))
            .map(|t| t.name.clone())
            .collect();
        let airbender =
            serde_json::from_value(package.metadata["airbender"].clone()).unwrap_or_default();
        Ok(Self {
            package_name: package.name.clone(),
            bin_targets,
            airbender,
        })
    }

    /// Returns the effective `panic_immediate_abort` for the given profile.
    ///
    /// Reads `package.metadata.airbender.profile.<profile>.panic-immediate-abort`.
    /// Defaults to `false` if the key is absent.
    pub fn panic_immediate_abort(&self, profile: Profile) -> bool {
        self.airbender
            .profile
            .get(profile.as_str())
            .map(|p| p.panic_immediate_abort)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(json: &str) -> AirbenderConfig {
        serde_json::from_str(json).expect("parse airbender config")
    }

    fn make_manifest(
        package_name: &str,
        bin_targets: &[&str],
        airbender: AirbenderConfig,
    ) -> ProjectManifest {
        ProjectManifest {
            package_name: package_name.to_string(),
            bin_targets: bin_targets.iter().map(|s| s.to_string()).collect(),
            airbender,
        }
    }

    #[test]
    fn panic_immediate_abort_returns_false_when_profile_absent() {
        let manifest = make_manifest("guest", &["guest"], AirbenderConfig::default());
        assert!(!manifest.panic_immediate_abort(Profile::Release));
        assert!(!manifest.panic_immediate_abort(Profile::Debug));
    }

    #[test]
    fn panic_immediate_abort_reads_release_profile() {
        let manifest = make_manifest(
            "guest",
            &["guest"],
            make_config(r#"{"profile": {"release": {"panic-immediate-abort": true}}}"#),
        );
        assert!(manifest.panic_immediate_abort(Profile::Release));
        assert!(!manifest.panic_immediate_abort(Profile::Debug));
    }

    #[test]
    fn panic_immediate_abort_reads_debug_profile() {
        let manifest = make_manifest(
            "guest",
            &["guest"],
            make_config(r#"{"profile": {"debug": {"panic-immediate-abort": true}}}"#),
        );
        assert!(!manifest.panic_immediate_abort(Profile::Release));
        assert!(manifest.panic_immediate_abort(Profile::Debug));
    }

    #[test]
    fn panic_immediate_abort_defaults_false_when_key_missing() {
        let manifest = make_manifest(
            "guest",
            &["guest"],
            make_config(r#"{"profile": {"release": {}}}"#),
        );
        assert!(!manifest.panic_immediate_abort(Profile::Release));
    }

    #[test]
    fn panic_immediate_abort_independent_per_profile() {
        let manifest = make_manifest(
            "guest",
            &["guest"],
            make_config(
                r#"{"profile": {"release": {"panic-immediate-abort": true}, "debug": {"panic-immediate-abort": true}}}"#,
            ),
        );
        assert!(manifest.panic_immediate_abort(Profile::Release));
        assert!(manifest.panic_immediate_abort(Profile::Debug));
    }

    #[test]
    fn airbender_config_defaults_on_unknown_metadata() {
        let config: AirbenderConfig =
            serde_json::from_str(r#"{"unknown-key": 42}"#).expect("should tolerate unknown keys");
        assert!(config.profile.is_empty());
    }

    #[test]
    fn airbender_config_defaults_on_null_metadata() {
        let config: AirbenderConfig =
            serde_json::from_value(serde_json::Value::Null).unwrap_or_default();
        assert!(config.profile.is_empty());
    }

    #[test]
    fn load_reads_package_name_and_bin_targets() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/fibonacci/guest/Cargo.toml");
        let manifest = ProjectManifest::load(&manifest_path).expect("load fibonacci manifest");
        assert_eq!(manifest.package_name, "airbender-fibonacci");
        assert_eq!(manifest.bin_targets, vec!["airbender-fibonacci"]);
    }

    #[test]
    fn load_defaults_airbender_config_when_metadata_absent() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"no-meta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write Cargo.toml");
        std::fs::create_dir(temp_dir.path().join("src")).expect("create src");
        std::fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}").expect("write main.rs");
        let manifest =
            ProjectManifest::load(&temp_dir.path().join("Cargo.toml")).expect("load manifest");
        assert!(!manifest.panic_immediate_abort(Profile::Release));
        assert!(!manifest.panic_immediate_abort(Profile::Debug));
    }

    #[test]
    fn load_reads_airbender_metadata() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/fibonacci/guest/Cargo.toml");
        let manifest = ProjectManifest::load(&manifest_path).expect("load fibonacci manifest");
        assert!(!manifest.panic_immediate_abort(Profile::Release));
        assert!(!manifest.panic_immediate_abort(Profile::Debug));
    }
}
