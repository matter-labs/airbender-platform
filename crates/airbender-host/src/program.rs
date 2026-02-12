use crate::error::{HostError, Result};
use crate::prover::{CpuProverBuilder, GpuProverBuilder};
use crate::runner::{SimulatorRunnerBuilder, TranspilerRunnerBuilder};
use crate::vk::{compute_unified_vk, verify_proof, UnifiedVk};
use airbender_core::manifest::Manifest;
use sha3::Digest;
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

    /// Create a CPU prover builder bound to this program.
    pub fn cpu_prover(&self) -> CpuProverBuilder {
        CpuProverBuilder::new(self.app_bin())
    }

    /// Compute the unified verification key for this program.
    pub fn compute_vk(&self) -> Result<UnifiedVk> {
        compute_unified_vk(self.app_bin())
    }

    /// Verify a proof against a unified verification key.
    pub fn verify(
        &self,
        proof: &execution_utils::unrolled::UnrolledProgramProof,
        vk: &UnifiedVk,
    ) -> Result<()> {
        let app_bin_hash = hash_app_bin(self.app_bin())?;
        verify_proof(proof, vk, Some(app_bin_hash))
    }
}

fn hash_app_bin(path: &Path) -> Result<[u8; 32]> {
    let bytes = std::fs::read(path)?;
    Ok(sha3::Keccak256::digest(&bytes).into())
}
