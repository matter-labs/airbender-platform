use crate::cli::{ProveArgs, ProverBackendArg, ProverLevelArg};
use crate::input;
use airbender_host::Prover;
use anyhow::{Context, Result};

pub fn run(args: ProveArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let level = as_host_level(args.level);

    tracing::info!("starting proof generation");
    let prove_result = match args.backend {
        ProverBackendArg::Gpu => {
            let mut builder =
                airbender_host::GpuProverBuilder::new(&args.app_bin).with_level(level);
            if let Some(threads) = args.threads {
                builder = builder.with_worker_threads(threads);
            }
            let prover = builder.build().with_context(|| {
                format!(
                    "while attempting to initialize GPU prover for {}",
                    args.app_bin.display()
                )
            })?;
            prover.prove(&input_words)
        }
        ProverBackendArg::Cpu => {
            if level != airbender_host::ProverLevel::Base {
                return Err(anyhow::anyhow!(
                    "CPU backend currently supports only --level base"
                ));
            }

            let mut builder = airbender_host::CpuProverBuilder::new(&args.app_bin);
            if let Some(threads) = args.threads {
                builder = builder.with_worker_threads(threads);
            }
            if let Some(cycles) = args.cycles {
                builder = builder.with_cycles(cycles);
            }
            if let Some(ram_bound) = args.ram_bound {
                builder = builder.with_ram_bound(ram_bound);
            }

            let prover = builder.build().with_context(|| {
                format!(
                    "while attempting to initialize CPU prover for {}",
                    args.app_bin.display()
                )
            })?;
            prover.prove(&input_words)
        }
    }
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

fn as_host_level(level: ProverLevelArg) -> airbender_host::ProverLevel {
    match level {
        ProverLevelArg::Base => airbender_host::ProverLevel::Base,
        ProverLevelArg::RecursionUnrolled => airbender_host::ProverLevel::RecursionUnrolled,
        ProverLevelArg::RecursionUnified => airbender_host::ProverLevel::RecursionUnified,
    }
}
