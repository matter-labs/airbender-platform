use crate::error::{HostError, Result};
use crate::proof::{hash_app_bin, hash_input_words, Proof, RealProof};
use crate::prover::ProverLevel;
use crate::security::SecurityLevel;
use crate::vk::{compute_real_vk, verify_proof, RealVk};
use airbender_core::guest::Commit;
use std::path::{Path, PathBuf};

/// Wrapper around all verification-key flavors.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum VerificationKey {
    Dev(DevVerificationKey),
    Real(RealVerificationKey),
}

impl VerificationKey {
    pub fn security(&self) -> SecurityLevel {
        match self {
            Self::Dev(vk) => vk.security,
            Self::Real(vk) => vk.vk.security,
        }
    }
}

/// Development verification key.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DevVerificationKey {
    pub security: SecurityLevel,
    pub app_bin_hash: [u8; 32],
}

/// Real verification key wrapper (base / recursion-unrolled / recursion-unified).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RealVerificationKey {
    pub vk: RealVk,
}

impl RealVerificationKey {
    pub fn level(&self) -> ProverLevel {
        self.vk.level
    }
}

/// Verification checks requested by the caller.
#[derive(Clone, Copy, Default)]
pub struct VerificationRequest<'a> {
    expected_output: Option<&'a dyn Commit>,
    expected_input_words: Option<&'a [u32]>,
}

impl<'a> VerificationRequest<'a> {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_expected_output(mut self, expected_output: &'a dyn Commit) -> Self {
        self.expected_output = Some(expected_output);
        self
    }

    pub fn with_expected_input_words(mut self, expected_input_words: &'a [u32]) -> Self {
        self.expected_input_words = Some(expected_input_words);
        self
    }

    pub fn real(expected_output: &'a dyn Commit) -> Self {
        Self::empty().with_expected_output(expected_output)
    }

    pub fn dev(expected_input_words: &'a [u32], expected_output: &'a dyn Commit) -> Self {
        Self::empty()
            .with_expected_input_words(expected_input_words)
            .with_expected_output(expected_output)
    }

    fn expected_output(self) -> Option<&'a dyn Commit> {
        self.expected_output
    }

    fn expected_input_words(self) -> Option<&'a [u32]> {
        self.expected_input_words
    }
}

/// Verifier interface shared by dev and real verifiers.
pub trait Verifier {
    fn generate_vk(&self, security: SecurityLevel) -> Result<VerificationKey>;

    fn verify(
        &self,
        proof: &Proof,
        vk: &VerificationKey,
        request: VerificationRequest<'_>,
    ) -> Result<()>;
}

/// Builder for a development verifier.
pub struct DevVerifierBuilder {
    app_bin_path: PathBuf,
}

impl DevVerifierBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
        }
    }

    pub fn build(self) -> Result<DevVerifier> {
        DevVerifier::new(&self.app_bin_path)
    }
}

/// Builder for a real verifier.
pub struct RealVerifierBuilder {
    app_bin_path: PathBuf,
    level: ProverLevel,
}

impl RealVerifierBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>, level: ProverLevel) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            level,
        }
    }

    pub fn build(self) -> Result<RealVerifier> {
        RealVerifier::new(&self.app_bin_path, self.level)
    }
}

/// Development verifier implementation.
pub struct DevVerifier {
    app_bin_hash: [u8; 32],
}

impl DevVerifier {
    fn new(app_bin_path: &Path) -> Result<Self> {
        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let app_bin_hash = hash_app_bin(&app_bin_path)?;
        Ok(Self { app_bin_hash })
    }

    pub fn generate_vk(&self, security: SecurityLevel) -> Result<VerificationKey> {
        Ok(VerificationKey::Dev(DevVerificationKey {
            security,
            app_bin_hash: self.app_bin_hash,
        }))
    }
}

impl Verifier for DevVerifier {
    fn generate_vk(&self, security: SecurityLevel) -> Result<VerificationKey> {
        DevVerifier::generate_vk(self, security)
    }

    fn verify(
        &self,
        proof: &Proof,
        vk: &VerificationKey,
        request: VerificationRequest<'_>,
    ) -> Result<()> {
        let proof = match proof {
            Proof::Dev(proof) => proof,
            Proof::Real(_) => {
                return Err(HostError::Verification(
                    "dev verifier cannot verify real proofs".to_string(),
                ));
            }
        };
        let vk = match vk {
            VerificationKey::Dev(vk) => vk,
            VerificationKey::Real(_) => {
                return Err(HostError::Verification(
                    "dev verifier requires a dev verification key".to_string(),
                ));
            }
        };

        ensure_proof_vk_security_matches(proof.security, vk.security)?;

        if vk.app_bin_hash != self.app_bin_hash {
            return Err(HostError::Verification(
                "dev verification key does not match current program".to_string(),
            ));
        }

        if proof.app_bin_hash != self.app_bin_hash {
            return Err(HostError::Verification(
                "dev proof was produced for a different program".to_string(),
            ));
        }

        let expected_input_words = request.expected_input_words().ok_or_else(|| {
            HostError::Verification("dev verification requires expected input words".to_string())
        })?;
        let expected_input_hash = hash_input_words(expected_input_words);
        if proof.input_words_hash != expected_input_hash {
            return Err(HostError::Verification(
                "dev proof input hash does not match expected input words".to_string(),
            ));
        }

        let expected_output = request.expected_output().ok_or_else(|| {
            HostError::Verification("dev verification requires expected output".to_string())
        })?;
        let expected_words = expected_output.commit_words();
        if proof.receipt.output != expected_words {
            return Err(HostError::Verification(format!(
                "public output mismatch: expected {expected_words:?}, got {:?}",
                proof.receipt.output
            )));
        }

        Ok(())
    }
}

/// Real verifier implementation.
pub struct RealVerifier {
    app_bin_path: PathBuf,
    app_bin_hash: [u8; 32],
    level: ProverLevel,
}

impl RealVerifier {
    fn new(app_bin_path: &Path, level: ProverLevel) -> Result<Self> {
        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let app_bin_hash = hash_app_bin(&app_bin_path)?;
        Ok(Self {
            app_bin_path,
            app_bin_hash,
            level,
        })
    }

    pub fn generate_vk(&self, security: SecurityLevel) -> Result<VerificationKey> {
        let vk = compute_real_vk(&self.app_bin_path, self.level, security)?;
        Ok(VerificationKey::Real(RealVerificationKey { vk }))
    }
}

impl Verifier for RealVerifier {
    fn generate_vk(&self, security: SecurityLevel) -> Result<VerificationKey> {
        RealVerifier::generate_vk(self, security)
    }

    fn verify(
        &self,
        proof: &Proof,
        vk: &VerificationKey,
        request: VerificationRequest<'_>,
    ) -> Result<()> {
        if request.expected_input_words().is_some() {
            return Err(HostError::Verification(
                "real verifier cannot validate input words".to_string(),
            ));
        }

        let proof = match proof {
            Proof::Real(proof) => proof,
            Proof::Dev(_) => {
                return Err(HostError::Verification(
                    "real verifier cannot verify dev proofs".to_string(),
                ));
            }
        };
        let vk = match vk {
            VerificationKey::Real(RealVerificationKey { vk }) => vk,
            VerificationKey::Dev(_) => {
                return Err(HostError::Verification(
                    "real verifier requires a real verification key".to_string(),
                ));
            }
        };

        if vk.level != proof.level() || self.level != proof.level() {
            return Err(HostError::Verification(format!(
                "proof level {:?} does not match verification key level {:?} / verifier level {:?}",
                proof.level(),
                vk.level,
                self.level
            )));
        }
        ensure_proof_vk_security_matches(proof.security(), vk.security)?;

        verify_proof(
            proof.inner(),
            vk,
            &self.app_bin_path,
            Some(self.app_bin_hash),
            request.expected_output(),
        )?;
        Ok(())
    }
}

/// Verify a real proof envelope against a real verification key.
///
/// The recursion pipeline recomputes the trusted per-layer parameters from the
/// program, so the program's `app.bin` (with its `app.text` sibling) must be
/// supplied. This helper validates proof/VK compatibility, that the program
/// matches the key, and optional expected public output.
pub fn verify_real_proof_with_vk(
    proof: &RealProof,
    vk: &VerificationKey,
    app_bin_path: &Path,
    expected_output: Option<&dyn Commit>,
) -> Result<()> {
    let vk = match vk {
        VerificationKey::Real(RealVerificationKey { vk }) => vk,
        VerificationKey::Dev(_) => {
            return Err(HostError::Verification(
                "real proofs require real verification keys".to_string(),
            ));
        }
    };
    if vk.level != proof.level() {
        return Err(HostError::Verification(format!(
            "proof level {:?} does not match verification key level {:?}",
            proof.level(),
            vk.level
        )));
    }
    ensure_proof_vk_security_matches(proof.security(), vk.security)?;
    verify_proof(proof.inner(), vk, app_bin_path, None, expected_output)?;
    Ok(())
}

fn ensure_proof_vk_security_matches(
    proof_security: SecurityLevel,
    vk_security: SecurityLevel,
) -> Result<()> {
    if proof_security != vk_security {
        return Err(HostError::Verification(format!(
            "proof security {} bits does not match verification key security {} bits",
            proof_security, vk_security
        )));
    }
    Ok(())
}

fn resolve_app_bin_path(path: &Path) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| HostError::Verification("app path is not valid UTF-8".to_string()))?;
    let base = path_str.strip_suffix(".bin").unwrap_or(path_str);
    let app_bin_path = PathBuf::from(format!("{base}.bin"));
    if !app_bin_path.exists() {
        return Err(HostError::Verification(format!(
            "binary not found: {}",
            app_bin_path.display()
        )));
    }
    Ok(app_bin_path)
}
