use crate::error::{HostError, Result};
use crate::prover::ProverLevel;
use execution_utils::setups;
use execution_utils::unified_circuit::verify_proof_in_unified_layer;
use execution_utils::unrolled::{
    compute_setup_for_machine_configuration, get_unrolled_circuits_artifacts_for_machine_type,
    verify_unrolled_layer_proof, UnrolledProgramProof, UnrolledProgramSetup,
};
use risc_v_simulator::cycle::{
    IMStandardIsaConfigWithUnsignedMulDiv, IWithoutByteAccessIsaConfigWithDelegation,
};
use sha3::Digest;
use std::fs;
use std::path::{Path, PathBuf};

/// Unified verification key bundle for recursion.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnifiedVk {
    pub app_bin_hash: [u8; 32],
    pub unified_setup: UnrolledProgramSetup,
    pub unified_layouts: setups::CompiledCircuitsSet,
}

/// Unrolled verification key bundle for base or recursion-unrolled layers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnrolledVk {
    pub app_bin_hash: [u8; 32],
    pub setup: UnrolledProgramSetup,
    pub compiled_layouts: setups::CompiledCircuitsSet,
}

pub fn compute_unified_vk(app_bin_path: &Path) -> Result<UnifiedVk> {
    let app_bin_hash = hash_app_bin(app_bin_path)?;

    // TODO: cache unified setup/layout artifacts on disk to avoid recomputing on every run.
    let (binary, binary_u32) =
        setups::pad_binary(execution_utils::unrolled_gpu::RECURSION_UNIFIED_BIN.to_vec());
    let (text, _) =
        setups::pad_binary(execution_utils::unrolled_gpu::RECURSION_UNIFIED_TXT.to_vec());

    let unified_setup =
        execution_utils::unified_circuit::compute_unified_setup_for_machine_configuration::<
            IWithoutByteAccessIsaConfigWithDelegation,
        >(&binary, &text);
    let unified_layouts = execution_utils::setups::get_unified_circuit_artifact_for_machine_type::<
        IWithoutByteAccessIsaConfigWithDelegation,
    >(&binary_u32);

    Ok(UnifiedVk {
        app_bin_hash,
        unified_setup,
        unified_layouts,
    })
}

pub fn compute_unrolled_vk(app_bin_path: &Path, level: ProverLevel) -> Result<UnrolledVk> {
    if level == ProverLevel::RecursionUnified {
        return Err(HostError::Verification(
            "unified verification keys must be generated with compute_unified_vk".to_string(),
        ));
    }

    let resolved_bin_path = resolve_bin_path(app_bin_path)?;
    let app_bin_hash = hash_app_bin(&resolved_bin_path)?;

    let (binary, binary_u32, text) = match level {
        ProverLevel::Base => {
            let app_text_path = resolve_text_path(&resolved_bin_path)?;
            let (binary, binary_u32) = setups::read_and_pad_binary(&resolved_bin_path);
            let (text, _) = setups::read_and_pad_binary(&app_text_path);
            (binary, binary_u32, text)
        }
        ProverLevel::RecursionUnrolled => {
            let (binary, binary_u32) =
                setups::pad_binary(execution_utils::unrolled_gpu::RECURSION_UNROLLED_BIN.to_vec());
            let (text, _) =
                setups::pad_binary(execution_utils::unrolled_gpu::RECURSION_UNROLLED_TXT.to_vec());
            (binary, binary_u32, text)
        }
        ProverLevel::RecursionUnified => {
            return Err(HostError::Verification(
                "unified verification keys must be generated with compute_unified_vk".to_string(),
            ));
        }
    };

    let (setup, compiled_layouts) = match level {
        ProverLevel::Base => {
            let setup = compute_setup_for_machine_configuration::<
                IMStandardIsaConfigWithUnsignedMulDiv,
            >(&binary, &text);
            let compiled_layouts = get_unrolled_circuits_artifacts_for_machine_type::<
                IMStandardIsaConfigWithUnsignedMulDiv,
            >(&binary_u32);
            (setup, compiled_layouts)
        }
        ProverLevel::RecursionUnrolled => {
            let setup = compute_setup_for_machine_configuration::<
                IWithoutByteAccessIsaConfigWithDelegation,
            >(&binary, &text);
            let compiled_layouts = get_unrolled_circuits_artifacts_for_machine_type::<
                IWithoutByteAccessIsaConfigWithDelegation,
            >(&binary_u32);
            (setup, compiled_layouts)
        }
        ProverLevel::RecursionUnified => {
            return Err(HostError::Verification(
                "unified verification keys must be generated with compute_unified_vk".to_string(),
            ));
        }
    };

    Ok(UnrolledVk {
        app_bin_hash,
        setup,
        compiled_layouts,
    })
}

pub fn verify_proof(
    proof: &UnrolledProgramProof,
    vk: &UnifiedVk,
    expected_app_bin_hash: Option<[u8; 32]>,
) -> Result<()> {
    verify_app_bin_hash(expected_app_bin_hash, vk.app_bin_hash)?;

    verify_proof_in_unified_layer(proof, &vk.unified_setup, &vk.unified_layouts, false)
        .map_err(|_| HostError::Verification("proof verification failed".to_string()))?;
    Ok(())
}

pub fn verify_unrolled_proof(
    proof: &UnrolledProgramProof,
    vk: &UnrolledVk,
    level: ProverLevel,
    expected_app_bin_hash: Option<[u8; 32]>,
) -> Result<()> {
    verify_app_bin_hash(expected_app_bin_hash, vk.app_bin_hash)?;

    let is_base_layer = match level {
        ProverLevel::Base => true,
        ProverLevel::RecursionUnrolled => false,
        ProverLevel::RecursionUnified => {
            return Err(HostError::Verification(
                "recursion-unified proofs must be verified with unified verification keys"
                    .to_string(),
            ));
        }
    };

    verify_unrolled_layer_proof(proof, &vk.setup, &vk.compiled_layouts, is_base_layer)
        .map_err(|_| HostError::Verification("proof verification failed".to_string()))?;
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

fn hash_app_bin(path: &Path) -> Result<[u8; 32]> {
    let app_bin_bytes = fs::read(path)?;
    Ok(sha3::Keccak256::digest(&app_bin_bytes).into())
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
