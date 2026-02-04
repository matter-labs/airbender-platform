use crate::cli::{FlamegraphArgs, RunArgs, RunTranspilerArgs};
use crate::input;
use anyhow::{Context, Result};

// Keep parity with the legacy airbender-cli defaults for simulator-oriented commands.
const DEFAULT_CYCLE_LIMIT: usize = 90_000_000_000;

pub fn run(args: RunArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let cycle_limit = args.cycles.unwrap_or(DEFAULT_CYCLE_LIMIT);
    tracing::info!("running simulator");
    let outcome = airbender_host::run_simulator(&args.app_bin, &input_words, cycle_limit)
        .with_context(|| {
            format!(
                "while attempting to run simulator for {}",
                args.app_bin.display()
            )
        })?;
    report_execution_outcome(&outcome);
    Ok(())
}

pub fn flamegraph(args: FlamegraphArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let cycle_limit = args.cycles.unwrap_or(DEFAULT_CYCLE_LIMIT);
    let flamegraph = airbender_host::FlamegraphConfig {
        output: args.output,
        sampling_rate: args.sampling_rate,
        inverse: args.inverse,
        elf_path: args.elf_path,
    };

    tracing::info!("running simulator with profiler");
    let outcome = airbender_host::run_simulator_with_flamegraph(
        &args.app_bin,
        &input_words,
        cycle_limit,
        &flamegraph,
    )
    .with_context(|| {
        format!(
            "while attempting to generate flamegraph for {}",
            args.app_bin.display()
        )
    })?;
    report_execution_outcome(&outcome);
    Ok(())
}

pub fn run_transpiler(args: RunTranspilerArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let cycle_limit = args.cycles.unwrap_or(DEFAULT_CYCLE_LIMIT);
    tracing::info!("running transpiler JIT");
    let outcome = airbender_host::run_transpiler(
        &args.app_bin,
        &input_words,
        cycle_limit,
        args.text_path.as_deref(),
    )
    .with_context(|| {
        format!(
            "while attempting to run transpiler for {}",
            args.app_bin.display()
        )
    })?;
    report_execution_outcome(&outcome);
    Ok(())
}

fn report_execution_outcome(outcome: &airbender_host::ExecutionResult) {
    tracing::info!(
        "execution finished: cycles_executed={}, reached_end={}",
        outcome.cycles_executed,
        outcome.reached_end
    );

    let mut registers = String::new();
    for (offset, value) in outcome.receipt.output.iter().enumerate() {
        use std::fmt::Write;
        let _ = write!(registers, "x{}={} ", offset + 10, value);
    }
    tracing::info!("output values: {}", registers.trim_end());
}
