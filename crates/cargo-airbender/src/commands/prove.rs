use crate::cli::{ProveArgs, ProverBackendArg, ProverLevelArg};
use crate::error::{CliError, Result};
use crate::input;
use crate::ui;
use airbender_host::Prover;

pub fn run(args: ProveArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let level = as_host_level(args.level);

    let prove_result = match args.backend {
        ProverBackendArg::Gpu => {
            let mut builder =
                airbender_host::GpuProverBuilder::new(&args.app_bin).with_level(level);
            if let Some(threads) = args.threads {
                builder = builder.with_worker_threads(threads);
            }
            let prover = builder.build().map_err(|err| {
                CliError::with_source(
                    format!(
                        "failed to initialize GPU prover for `{}`",
                        args.app_bin.display()
                    ),
                    err,
                )
            })?;

            prover.prove(&input_words)
        }
        ProverBackendArg::Cpu => {
            if level != airbender_host::ProverLevel::Base {
                return Err(
                    CliError::new("CPU backend currently supports only `--level base`")
                        .with_hint("use `--backend gpu` for recursion levels"),
                );
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

            let prover = builder.build().map_err(|err| {
                CliError::with_source(
                    format!(
                        "failed to initialize CPU prover for `{}`",
                        args.app_bin.display()
                    ),
                    err,
                )
            })?;

            prover.prove(&input_words)
        }
    }
    .map_err(|err| {
        CliError::with_source(
            format!("failed to generate proof for `{}`", args.app_bin.display()),
            err,
        )
        .with_hint("set `RUST_LOG=info` to inspect prover backend logs")
    })?;

    tracing::info!("{}", prove_result.proof.debug_info());

    let encoded = bincode::serde::encode_to_vec(&prove_result.proof, bincode::config::standard())
        .map_err(|err| CliError::with_source("failed to encode proof", err))?;
    std::fs::write(&args.output, encoded).map_err(|err| {
        CliError::with_source(
            format!("failed to write proof to `{}`", args.output.display()),
            err,
        )
    })?;

    ui::success("proof generated");
    ui::field("backend", backend_name(args.backend));
    ui::field("level", level_name(args.level));
    ui::field("cycles", prove_result.cycles);
    ui::field("output", args.output.display());

    Ok(())
}

fn backend_name(backend: ProverBackendArg) -> &'static str {
    match backend {
        ProverBackendArg::Cpu => "cpu",
        ProverBackendArg::Gpu => "gpu",
    }
}

fn level_name(level: ProverLevelArg) -> &'static str {
    match level {
        ProverLevelArg::Base => "base",
        ProverLevelArg::RecursionUnrolled => "recursion-unrolled",
        ProverLevelArg::RecursionUnified => "recursion-unified",
    }
}

fn as_host_level(level: ProverLevelArg) -> airbender_host::ProverLevel {
    match level {
        ProverLevelArg::Base => airbender_host::ProverLevel::Base,
        ProverLevelArg::RecursionUnrolled => airbender_host::ProverLevel::RecursionUnrolled,
        ProverLevelArg::RecursionUnified => airbender_host::ProverLevel::RecursionUnified,
    }
}
