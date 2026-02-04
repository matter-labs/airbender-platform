use crate::cli::{GenerateVkArgs, ProverLevelArg, VerifyProofArgs};
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

pub fn generate(args: GenerateVkArgs) -> Result<()> {
    match args.level {
        ProverLevelArg::RecursionUnified => {
            tracing::info!("computing unified recursion VKs");
            let vk = airbender_host::compute_unified_vk(&args.app_bin).with_context(|| {
                format!(
                    "while attempting to compute unified VKs for {}",
                    args.app_bin.display()
                )
            })?;
            write_bincode(&args.output, &vk)?;
        }
        ProverLevelArg::Base | ProverLevelArg::RecursionUnrolled => {
            tracing::info!("computing unrolled VKs");
            let vk = airbender_host::compute_unrolled_vk(&args.app_bin, as_host_level(args.level))
                .with_context(|| {
                    format!(
                        "while attempting to compute unrolled VKs for {}",
                        args.app_bin.display()
                    )
                })?;
            write_bincode(&args.output, &vk)?;
        }
    }

    tracing::info!("VKs written to {}", args.output.display());
    Ok(())
}

pub fn verify(args: VerifyProofArgs) -> Result<()> {
    let proof: airbender_host::UnrolledProgramProof =
        read_bincode(&args.proof).context("while attempting to decode proof")?;

    tracing::info!("verifying proof");
    match args.level {
        ProverLevelArg::RecursionUnified => {
            let vk: airbender_host::UnifiedVk =
                read_bincode(&args.vk).context("while attempting to decode unified VK file")?;
            airbender_host::verify_proof(&proof, &vk, None)
                .context("while attempting to verify proof")?;
        }
        ProverLevelArg::Base | ProverLevelArg::RecursionUnrolled => {
            let vk: airbender_host::UnrolledVk =
                read_bincode(&args.vk).context("while attempting to decode unrolled VK file")?;
            airbender_host::verify_unrolled_proof(&proof, &vk, as_host_level(args.level), None)
                .context("while attempting to verify proof")?;
        }
    }

    tracing::info!("proof verified successfully");
    Ok(())
}

fn as_host_level(level: ProverLevelArg) -> airbender_host::ProverLevel {
    match level {
        ProverLevelArg::Base => airbender_host::ProverLevel::Base,
        ProverLevelArg::RecursionUnrolled => airbender_host::ProverLevel::RecursionUnrolled,
        ProverLevelArg::RecursionUnified => airbender_host::ProverLevel::RecursionUnified,
    }
}

fn read_bincode<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("while attempting to read {}", path.display()))?;
    let (decoded, read_len): (T, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|err| anyhow::anyhow!("while attempting to decode bincode: {err}"))?;
    if read_len != bytes.len() {
        tracing::warn!(
            "bincode decoded {} bytes but file is {} bytes",
            read_len,
            bytes.len()
        );
    }
    Ok(decoded)
}

fn write_bincode<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = bincode::serde::encode_to_vec(value, bincode::config::standard())
        .map_err(|err| anyhow::anyhow!("while attempting to encode bincode: {err}"))?;
    std::fs::write(path, encoded)
        .with_context(|| format!("while attempting to write {}", path.display()))?;
    Ok(())
}
