use crate::error::{HostError, Result};
use crate::prover::{CpuProverBuilder, DevProverBuilder, GpuProverBuilder, ProverLevel};
use crate::runner::{SimulatorRunnerBuilder, TranspilerRunnerBuilder};
use crate::verifier::{DevVerifierBuilder, RealVerifierBuilder};
use airbender_core::host::manifest::Manifest;
use sha2::Digest;
use std::path::{Path, PathBuf};

/// Loaded Airbender program distribution, including manifest and artifacts.
#[derive(Clone, Debug)]
pub struct Program {
    dist_dir: PathBuf,
    manifest: Manifest,
    app_bin: PathBuf,
    app_elf: PathBuf,
    app_text: PathBuf,
}

impl Program {
    pub fn load(dist_dir: impl AsRef<Path>) -> Result<Self> {
        let dist_dir = dist_dir.as_ref().to_path_buf();
        let manifest_path = dist_dir.join("manifest.toml");
        let manifest = Manifest::read_from_file(&manifest_path)
            .map_err(|err| HostError::InvalidManifest(err.to_string()))?;
        if manifest.codec_version != airbender_codec::AIRBENDER_CODEC_V0 {
            return Err(HostError::InvalidManifest(format!(
                "unsupported codec_version {}",
                manifest.codec_version
            )));
        }

        let app_bin = dist_dir.join(&manifest.bin_file);
        let app_elf = dist_dir.join(&manifest.elf_file);
        let app_text = dist_dir.join(&manifest.text_file);

        for path in [&app_bin, &app_elf, &app_text] {
            if !path.exists() {
                return Err(HostError::InvalidManifest(format!(
                    "missing artifact: {}",
                    path.display()
                )));
            }
        }

        verify_manifest_bin_sha256(&app_bin, &manifest.bin_sha256)?;

        Ok(Self {
            dist_dir,
            manifest,
            app_bin,
            app_elf,
            app_text,
        })
    }

    pub fn dist_dir(&self) -> &Path {
        &self.dist_dir
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn app_bin(&self) -> &Path {
        &self.app_bin
    }

    pub fn app_elf(&self) -> &Path {
        &self.app_elf
    }

    pub fn app_text(&self) -> &Path {
        &self.app_text
    }

    /// Create a simulator runner builder bound to this program.
    pub fn simulator_runner(&self) -> SimulatorRunnerBuilder {
        SimulatorRunnerBuilder::new(self.app_bin())
    }

    /// Create a transpiler runner builder bound to this program.
    pub fn transpiler_runner(&self) -> TranspilerRunnerBuilder {
        TranspilerRunnerBuilder::new(self.app_bin())
    }

    /// Create a GPU prover builder bound to this program.
    pub fn gpu_prover(&self) -> GpuProverBuilder {
        GpuProverBuilder::new(self.app_bin())
    }

    /// Create a development prover builder bound to this program.
    pub fn dev_prover(&self) -> DevProverBuilder {
        DevProverBuilder::new(self.app_bin())
    }

    /// Create a CPU prover builder bound to this program.
    pub fn cpu_prover(&self) -> CpuProverBuilder {
        CpuProverBuilder::new(self.app_bin())
    }

    /// Create a development verifier builder bound to this program.
    pub fn dev_verifier(&self) -> DevVerifierBuilder {
        DevVerifierBuilder::new(self.app_bin())
    }

    /// Create a real verifier builder bound to this program.
    pub fn real_verifier(&self, level: ProverLevel) -> RealVerifierBuilder {
        RealVerifierBuilder::new(self.app_bin(), level)
    }
}

fn verify_manifest_bin_sha256(app_bin: &Path, expected_hex: &str) -> Result<()> {
    if expected_hex.is_empty() {
        return Err(HostError::InvalidManifest(
            "missing `bin_sha256` in manifest; rebuild artifacts with current tooling".to_string(),
        ));
    }

    if expected_hex.len() != 64 || !expected_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostError::InvalidManifest(format!(
            "invalid `bin_sha256` in manifest: `{expected_hex}`"
        )));
    }

    let actual_hex = sha256_file_hex(app_bin)?;
    if !expected_hex.eq_ignore_ascii_case(&actual_hex) {
        return Err(HostError::InvalidManifest(format!(
            "`bin_sha256` mismatch for {}: expected `{expected_hex}`, got `{actual_hex}`",
            app_bin.display()
        )));
    }

    Ok(())
}

fn sha256_file_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let digest = sha2::Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to string cannot fail");
    }

    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_matching_manifest_digest() {
        let temp_file = unique_temp_file_path("matching-digest");
        std::fs::write(&temp_file, b"hello world").expect("write test file");
        let expected = sha256_file_hex(&temp_file).expect("compute expected digest");

        verify_manifest_bin_sha256(&temp_file, &expected).expect("digest verification must pass");

        std::fs::remove_file(&temp_file).expect("remove test file");
    }

    #[test]
    fn rejects_mismatching_manifest_digest() {
        let temp_file = unique_temp_file_path("mismatching-digest");
        std::fs::write(&temp_file, b"hello world").expect("write test file");
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";

        let err = verify_manifest_bin_sha256(&temp_file, wrong)
            .expect_err("digest verification must fail for mismatching hash");
        assert!(err.to_string().contains("bin_sha256` mismatch"));

        std::fs::remove_file(&temp_file).expect("remove test file");
    }

    #[test]
    fn rejects_missing_manifest_digest() {
        let temp_file = unique_temp_file_path("missing-digest");
        std::fs::write(&temp_file, b"hello world").expect("write test file");

        let err = verify_manifest_bin_sha256(&temp_file, "")
            .expect_err("digest verification must fail when digest is missing");
        assert!(err.to_string().contains("missing `bin_sha256`"));

        std::fs::remove_file(&temp_file).expect("remove test file");
    }

    fn unique_temp_file_path(label: &str) -> PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "airbender-host-program-{label}-{}-{now}.bin",
            std::process::id()
        ))
    }
}
