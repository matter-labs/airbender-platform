use crate::error::Result;
use crate::prover::ProverLevel;
use crate::receipt::Receipt;
use crate::security::SecurityLevel;
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
    pub fn security(&self) -> SecurityLevel {
        match self {
            Self::Dev(proof) => proof.security,
            Self::Real(proof) => proof.security,
        }
    }

    pub fn debug_info(&self) -> String {
        match self {
            Self::Dev(proof) => format!(
                "dev proof: security={} bits, cycles={}, output={:?}",
                proof.security, proof.cycles, proof.receipt.output
            ),
            Self::Real(proof) => {
                format!(
                    "real proof: security={} bits, {}",
                    proof.security,
                    proof.inner.debug_info()
                )
            }
        }
    }
}

/// Development proof emitted by the transpiler-based prover.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DevProof {
    pub security: SecurityLevel,
    pub app_bin_hash: [u8; 32],
    pub input_words_hash: [u8; 32],
    pub receipt: Receipt,
    pub cycles: u64,
}

/// Real cryptographic proof emitted by CPU/GPU provers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RealProof {
    security: SecurityLevel,
    level: ProverLevel,
    inner: execution_utils::unrolled::UnrolledProgramProof,
}

impl RealProof {
    pub(crate) fn new(
        security: SecurityLevel,
        level: ProverLevel,
        inner: execution_utils::unrolled::UnrolledProgramProof,
    ) -> Self {
        Self {
            security,
            level,
            inner,
        }
    }

    pub fn security(&self) -> SecurityLevel {
        self.security
    }

    pub fn level(&self) -> ProverLevel {
        self.level
    }

    /// Returns the wrapped unrolled proof.
    ///
    /// Using the raw proof directly is not recommended and is not covered by
    /// the stable `airbender-host` public API. This is exposed for rare cases,
    /// for example when a project depends on both `airbender-host` and direct
    /// Airbender crates at the same time.
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
