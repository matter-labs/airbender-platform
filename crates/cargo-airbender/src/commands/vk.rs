use crate::cli::{GenerateVkArgs, ProverLevelArg, VerifyProofArgs};
use crate::error::{CliError, Result};
use crate::ui;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

pub fn generate(args: GenerateVkArgs) -> Result<()> {
    match args.level {
        ProverLevelArg::RecursionUnified => {
            let vk = airbender_host::compute_unified_vk(&args.app_bin).map_err(|err| {
                CliError::with_source(
                    format!(
                        "failed to compute unified verification keys for `{}`",
                        args.app_bin.display()
                    ),
                    err,
                )
            })?;
            write_bincode(&args.output, &vk)?;
        }
        ProverLevelArg::Base | ProverLevelArg::RecursionUnrolled => {
            let vk = airbender_host::compute_unrolled_vk(&args.app_bin, as_host_level(args.level))
                .map_err(|err| {
                    CliError::with_source(
                        format!(
                            "failed to compute unrolled verification keys for `{}`",
                            args.app_bin.display()
                        ),
                        err,
                    )
                })?;
            write_bincode(&args.output, &vk)?;
        }
    }

    ui::success("verification keys generated");
    ui::field("level", level_name(args.level));
    ui::field("output", args.output.display());

    Ok(())
}

pub fn verify(args: VerifyProofArgs) -> Result<()> {
    let proof: airbender_host::UnrolledProgramProof = read_bincode(&args.proof).map_err(|err| {
        CliError::with_source(
            format!("failed to decode proof from `{}`", args.proof.display()),
            err,
        )
    })?;

    match args.level {
        ProverLevelArg::RecursionUnified => {
            let vk: airbender_host::UnifiedVk = read_bincode(&args.vk).map_err(|err| {
                CliError::with_source(
                    format!("failed to decode unified VK file `{}`", args.vk.display()),
                    err,
                )
            })?;
            airbender_host::verify_proof(&proof, &vk, None)
                .map_err(|err| CliError::with_source("proof verification failed", err))?;
        }
        ProverLevelArg::Base | ProverLevelArg::RecursionUnrolled => {
            let vk: airbender_host::UnrolledVk = read_bincode(&args.vk).map_err(|err| {
                CliError::with_source(
                    format!("failed to decode unrolled VK file `{}`", args.vk.display()),
                    err,
                )
            })?;
            airbender_host::verify_unrolled_proof(&proof, &vk, as_host_level(args.level), None)
                .map_err(|err| CliError::with_source("proof verification failed", err))?;
        }
    }

    ui::success("proof verified");
    ui::field("level", level_name(args.level));

    Ok(())
}

fn as_host_level(level: ProverLevelArg) -> airbender_host::ProverLevel {
    match level {
        ProverLevelArg::Base => airbender_host::ProverLevel::Base,
        ProverLevelArg::RecursionUnrolled => airbender_host::ProverLevel::RecursionUnrolled,
        ProverLevelArg::RecursionUnified => airbender_host::ProverLevel::RecursionUnified,
    }
}

fn level_name(level: ProverLevelArg) -> &'static str {
    match level {
        ProverLevelArg::Base => "base",
        ProverLevelArg::RecursionUnrolled => "recursion-unrolled",
        ProverLevelArg::RecursionUnified => "recursion-unified",
    }
}

fn read_bincode<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path).map_err(|err| {
        CliError::with_source(format!("failed to read `{}`", path.display()), err)
    })?;
    let (decoded, read_len): (T, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|err| CliError::with_source("failed to decode bincode payload", err))?;

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
        .map_err(|err| CliError::with_source("failed to encode bincode payload", err))?;
    std::fs::write(path, encoded).map_err(|err| {
        CliError::with_source(format!("failed to write `{}`", path.display()), err)
    })?;

    Ok(())
}
