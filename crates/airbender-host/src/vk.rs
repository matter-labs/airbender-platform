use crate::error::{HostError, Result};
use crate::prover::ProverLevel;
use crate::security::SecurityLevel;
use airbender_core::guest::Commit;
use prover_pipeline::{verify_artifact, ProgramSource, ProofArtifact};
use sha3::Digest;
use std::fs;
use std::path::{Path, PathBuf};

/// Verification key for real proofs.
///
/// With the GKR-based recursion pipeline the trusted per-layer parameters are
/// recomputed at verification time from the program itself (`app.bin` /
/// `app.text`) and the checked-in recursion verifier binaries, so the key only
/// pins *which* program and pipeline shape a proof must attest to.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RealVk {
    pub security: SecurityLevel,
    pub level: ProverLevel,
    /// keccak256 of `app.bin`.
    pub app_bin_hash: [u8; 32],
    /// keccak256 of `app.text`.
    pub app_text_hash: [u8; 32],
}

pub fn compute_real_vk(
    app_bin_path: &Path,
    level: ProverLevel,
    security: SecurityLevel,
) -> Result<RealVk> {
    let resolved_bin_path = resolve_bin_path(app_bin_path)?;
    let resolved_text_path = resolve_text_path(&resolved_bin_path)?;
    Ok(RealVk {
        security,
        level,
        app_bin_hash: hash_file(&resolved_bin_path)?,
        app_text_hash: hash_file(&resolved_text_path)?,
    })
}

/// Verify a real proof artifact for the program at `app_bin_path` (its
/// `app.text` sibling is resolved automatically) against `vk`.
///
/// Returns the verifier's public output (`x10..x25`): the first 8 words are the
/// program output, the last 8 the authenticated recursion chain.
pub fn verify_proof(
    proof: &ProofArtifact,
    vk: &RealVk,
    app_bin_path: &Path,
    expected_app_bin_hash: Option<[u8; 32]>,
    expected_output: Option<&dyn Commit>,
) -> Result<[u32; 16]> {
    verify_app_bin_hash(expected_app_bin_hash, vk.app_bin_hash)?;

    let resolved_bin_path = resolve_bin_path(app_bin_path)?;
    let resolved_text_path = resolve_text_path(&resolved_bin_path)?;
    if hash_file(&resolved_bin_path)? != vk.app_bin_hash {
        return Err(HostError::Verification(
            "app.bin hash does not match verification key".to_string(),
        ));
    }
    if hash_file(&resolved_text_path)? != vk.app_text_hash {
        return Err(HostError::Verification(
            "app.text hash does not match verification key".to_string(),
        ));
    }

    let proof_level = ProverLevel::from_proof_target(proof.target);
    if proof_level != vk.level {
        return Err(HostError::Verification(format!(
            "proof level {proof_level:?} does not match verification key level {:?}",
            vk.level
        )));
    }
    let proof_security = SecurityLevel::from_pipeline(proof.security_level);
    if proof_security != vk.security {
        return Err(HostError::Verification(format!(
            "proof security {proof_security} bits does not match verification key security {} bits",
            vk.security
        )));
    }

    let source = ProgramSource::from_paths(
        path_to_string(&resolved_bin_path)?,
        Some(path_to_string(&resolved_text_path)?),
    );
    let verifier_output = verify_artifact(proof, &source)
        .map_err(|err| HostError::Verification(format!("proof verification failed: {err}")))?;
    verify_expected_output(expected_output, verifier_output)?;
    Ok(verifier_output)
}

fn verify_expected_output(
    expected_output: Option<&dyn Commit>,
    verifier_output: [u32; 16],
) -> Result<()> {
    let Some(expected_output) = expected_output else {
        return Ok(());
    };

    let expected_words = expected_output.commit_words();
    let mut actual_words = [0u32; 8];
    actual_words.copy_from_slice(&verifier_output[..8]);

    if expected_words != actual_words {
        return Err(HostError::Verification(format!(
            "public output mismatch: expected {expected_words:?}, got {actual_words:?}"
        )));
    }

    Ok(())
}

fn verify_app_bin_hash(
    expected_app_bin_hash: Option<[u8; 32]>,
    actual_app_bin_hash: [u8; 32],
) -> Result<()> {
    if let Some(expected) = expected_app_bin_hash {
        if expected != actual_app_bin_hash {
            return Err(HostError::Verification(
                "app.bin hash does not match verification key".to_string(),
            ));
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<[u8; 32]> {
    let bytes = fs::read(path)?;
    Ok(sha3::Keccak256::digest(&bytes).into())
}

fn path_to_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| HostError::Verification("app path is not valid UTF-8".to_string()))
}

fn resolve_bin_path(path: &Path) -> Result<PathBuf> {
    let base_path = base_path(path)?;
    let app_bin_path = PathBuf::from(format!("{base_path}.bin"));

    if !app_bin_path.exists() {
        return Err(HostError::Verification(format!(
            "binary not found: {}",
            app_bin_path.display()
        )));
    }

    Ok(app_bin_path)
}

fn resolve_text_path(app_bin_path: &Path) -> Result<PathBuf> {
    let mut app_text_path = app_bin_path.to_path_buf();
    app_text_path.set_extension("text");

    if !app_text_path.exists() {
        return Err(HostError::Verification(format!(
            "text file not found: {}",
            app_text_path.display()
        )));
    }

    Ok(app_text_path)
}

fn base_path(app_bin_path: &Path) -> Result<String> {
    let path_str = app_bin_path
        .to_str()
        .ok_or_else(|| HostError::Verification("app path is not valid UTF-8".to_string()))?;
    if let Some(stripped) = path_str.strip_suffix(".bin") {
        Ok(stripped.to_string())
    } else {
        Ok(path_str.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::verify_expected_output;

    #[test]
    fn verify_expected_output_accepts_matching_words() {
        let mut verifier_output = [0u32; 16];
        verifier_output[0] = 42;

        verify_expected_output(Some(&42u32), verifier_output).expect("matching output must verify");
    }

    #[test]
    fn verify_expected_output_rejects_mismatch() {
        let verifier_output = [0u32; 16];

        let err = verify_expected_output(Some(&1u32), verifier_output)
            .expect_err("mismatching output must fail verification");
        assert!(err.to_string().contains("public output mismatch"));
    }
}
