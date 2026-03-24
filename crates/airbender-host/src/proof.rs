use crate::error::Result;
use crate::prover::ProverLevel;
use crate::receipt::Receipt;
use sha3::Digest;
use std::path::Path;

/// Wrapper around all proof flavors produced by host provers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(clippy::large_enum_variant)] // We don't want to optimize for the efficiency of the dev proof.
pub enum Proof {
    Dev(DevProof),
    Real(RealProof),
}

impl Proof {
    pub fn debug_info(&self) -> String {
        match self {
            Self::Dev(proof) => format!(
                "dev proof: cycles={}, output={:?}",
                proof.cycles, proof.receipt.output
            ),
            Self::Real(proof) => proof.inner.debug_info(),
        }
    }
}

/// Development proof emitted by the transpiler-based prover.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DevProof {
    pub app_bin_hash: [u8; 32],
    pub input_words_hash: [u8; 32],
    pub receipt: Receipt,
    pub cycles: u64,
}

/// Real cryptographic proof emitted by CPU/GPU provers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RealProof {
    level: ProverLevel,
    inner: execution_utils::unrolled::UnrolledProgramProof,
}

impl RealProof {
    pub(crate) fn new(
        level: ProverLevel,
        inner: execution_utils::unrolled::UnrolledProgramProof,
    ) -> Self {
        Self { level, inner }
    }

    pub fn level(&self) -> ProverLevel {
        self.level
    }

    /// Returns the wrapped unrolled proof.
    ///
    /// Direct use of the inner proof is not recommended. This is exposed as an
    /// escape hatch and is not considered part of the stable `airbender-host`
    /// public API.
    pub fn into_inner(self) -> execution_utils::unrolled::UnrolledProgramProof {
        self.inner
    }

    pub(crate) fn inner(&self) -> &execution_utils::unrolled::UnrolledProgramProof {
        &self.inner
    }
}

pub(crate) fn hash_app_bin(path: &Path) -> Result<[u8; 32]> {
    let bytes = std::fs::read(path)?;
    Ok(sha3::Keccak256::digest(&bytes).into())
}

pub(crate) fn hash_input_words(input_words: &[u32]) -> [u8; 32] {
    let mut hasher = sha3::Keccak256::new();
    for word in input_words {
        hasher.update(word.to_le_bytes());
    }
    hasher.finalize().into()
}
