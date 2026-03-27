//! Resolved dist app paths produced by a successful build.

use std::path::{Path, PathBuf};

use crate::errors::Result;
use crate::utils::sha256_file_hex;

/// Resolved output paths for dist app directory.
///
/// All artifact paths are derived from `dir` at construction time.
#[derive(Debug)]
pub struct DistApp {
    dir: PathBuf,
    manifest: PathBuf,
    bin: PathBuf,
    elf: PathBuf,
    text: PathBuf,
}

impl DistApp {
    pub fn new(dist_dir: PathBuf) -> Self {
        Self {
            manifest: dist_dir.join("manifest.toml"),
            bin: dist_dir.join("app.bin"),
            elf: dist_dir.join("app.elf"),
            text: dist_dir.join("app.text"),
            dir: dist_dir,
        }
    }

    /// Dist app directory (`<dist-root>/<app-name>/`).
    pub fn dir(&self) -> &Path {
        &self.dir
    }
    /// Path to `manifest.toml` - toml file consisting of flags the binary was built with.
    pub fn manifest(&self) -> &Path {
        &self.manifest
    }

    /// Path to `app.bin` - raw binary consumed by runtime and proving flows.
    pub fn bin(&self) -> &Path {
        &self.bin
    }

    /// Path to `app.elf` - ELF image used for symbol and debug workflows.
    pub fn elf(&self) -> &Path {
        &self.elf
    }

    /// Path to `app.text` - text-section image used by the transpiler.
    pub fn text(&self) -> &Path {
        &self.text
    }
}

/// Output produced by one successful build/package invocation.
#[derive(Clone, Debug)]
pub struct DistArtifact {
    /// Absolute path to the artifact file.
    pub path: PathBuf,
    /// SHA-256 digest for artifact integrity verification.
    pub sha256: String,
}

impl DistArtifact {
    pub fn new(path: PathBuf) -> Result<Self> {
        let sha256 = sha256_file_hex(&path)?;
        Ok(DistArtifact { path, sha256 })
    }
}

/// Artifacts produced by one successful build/package invocation.
#[derive(Clone, Debug)]
pub struct DistArtifacts {
    /// Dist app directory used for this build.
    pub dir: PathBuf,
    /// Raw binary image consumed by the runtime and proving flows.
    pub app_bin: DistArtifact,
    /// ELF image used for symbol and debug workflows.
    pub app_elf: DistArtifact,
    /// Text-section image used by the transpiler.
    pub app_text: DistArtifact,
    /// Path to `manifest.toml`.
    pub manifest_path: PathBuf,
}
