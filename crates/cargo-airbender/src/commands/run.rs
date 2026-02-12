use crate::cli::{FlamegraphArgs, RunArgs, RunTranspilerArgs};
use crate::input;
use airbender_host::Runner;
use anyhow::{Context, Result};

// Keep parity with the legacy airbender-cli defaults for simulator-oriented commands.
const DEFAULT_CYCLE_LIMIT: usize = 90_000_000_000;

pub fn run(args: RunArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let cycle_limit = args.cycles.unwrap_or(DEFAULT_CYCLE_LIMIT);
    let runner = airbender_host::SimulatorRunnerBuilder::new(&args.app_bin)
        .with_cycles(cycle_limit)
        .build()
        .with_context(|| {
            format!(
                "while attempting to initialize simulator runner for {}",
                args.app_bin.display()
            )
        })?;

    tracing::info!("running simulator");
    let outcome = runner.run(&input_words).with_context(|| {
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

    let runner = airbender_host::SimulatorRunnerBuilder::new(&args.app_bin)
        .with_cycles(cycle_limit)
        .with_flamegraph(flamegraph)
        .build()
        .with_context(|| {
            format!(
                "while attempting to initialize simulator runner for {}",
                args.app_bin.display()
            )
        })?;

    tracing::info!("running simulator with profiler");
    let outcome = runner.run(&input_words).with_context(|| {
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
    let mut builder =
        airbender_host::TranspilerRunnerBuilder::new(&args.app_bin).with_cycles(cycle_limit);
    if let Some(text_path) = args.text_path.as_ref() {
        builder = builder.with_text_path(text_path);
    }
    let runner = builder.build().with_context(|| {
        format!(
            "while attempting to initialize transpiler runner for {}",
            args.app_bin.display()
        )
    })?;

    tracing::info!("running transpiler JIT");
    let outcome = runner.run(&input_words).with_context(|| {
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
