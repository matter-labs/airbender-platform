use crate::cli::{ProveArgs, ProverBackendArg, ProverLevelArg};
use crate::input;
use anyhow::{Context, Result};

pub fn run(args: ProveArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let options = airbender_host::ProveOptions {
        backend: as_host_backend(args.backend),
        worker_threads: args.threads,
        cycles: args.cycles,
        ram_bound: args.ram_bound,
        level: as_host_level(args.level),
    };

    tracing::info!("starting proof generation");
    let prove_result = airbender_host::prove_with_options(&args.app_bin, &input_words, &options)
        .with_context(|| {
            format!(
                "while attempting to generate proof for {}",
                args.app_bin.display()
            )
        })?;
    tracing::info!("proof generated: cycles={}", prove_result.cycles);
    tracing::info!("{}", prove_result.proof.debug_info());

    let encoded = bincode::serde::encode_to_vec(&prove_result.proof, bincode::config::standard())
        .map_err(|err| anyhow::anyhow!("while attempting to encode proof: {err}"))?;
    std::fs::write(&args.output, encoded).with_context(|| {
        format!(
            "while attempting to write proof to {}",
            args.output.display()
        )
    })?;
    tracing::info!("proof written to {}", args.output.display());
    Ok(())
}

fn as_host_backend(backend: ProverBackendArg) -> airbender_host::ProverBackend {
    match backend {
        ProverBackendArg::Cpu => airbender_host::ProverBackend::Cpu,
        ProverBackendArg::Gpu => airbender_host::ProverBackend::Gpu,
    }
}

fn as_host_level(level: ProverLevelArg) -> airbender_host::ProverLevel {
    match level {
        ProverLevelArg::Base => airbender_host::ProverLevel::Base,
        ProverLevelArg::RecursionUnrolled => airbender_host::ProverLevel::RecursionUnrolled,
        ProverLevelArg::RecursionUnified => airbender_host::ProverLevel::RecursionUnified,
    }
}
