use crate::cli::{FlamegraphArgs, RunArgs, RunTranspilerArgs};
use crate::error::{CliError, Result};
use crate::input;
use crate::ui;
use airbender_host::Runner;

pub fn run(args: RunArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let mut builder = airbender_host::SimulatorRunnerBuilder::new(&args.app_bin);
    if let Some(cycle_limit) = args.cycles {
        builder = builder.with_cycles(cycle_limit);
    }

    let runner = builder.build().map_err(|err| {
        CliError::with_source(
            format!(
                "failed to initialize simulator runner for `{}`",
                args.app_bin.display()
            ),
            err,
        )
    })?;

    let outcome = runner.run(&input_words).map_err(|err| {
        CliError::with_source(
            format!(
                "simulator execution failed for `{}`",
                args.app_bin.display()
            ),
            err,
        )
    })?;

    report_execution_outcome("simulator", &outcome);

    Ok(())
}

pub fn flamegraph(args: FlamegraphArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let flamegraph_output = args.output.clone();
    let flamegraph = airbender_host::FlamegraphConfig {
        output: args.output,
        sampling_rate: args.sampling_rate,
        inverse: args.inverse,
        elf_path: args.elf_path,
    };
    let mut builder =
        airbender_host::SimulatorRunnerBuilder::new(&args.app_bin).with_flamegraph(flamegraph);
    if let Some(cycle_limit) = args.cycles {
        builder = builder.with_cycles(cycle_limit);
    }

    let runner = builder.build().map_err(|err| {
        CliError::with_source(
            format!(
                "failed to initialize simulator runner for `{}`",
                args.app_bin.display()
            ),
            err,
        )
    })?;

    let outcome = runner.run(&input_words).map_err(|err| {
        CliError::with_source(
            format!(
                "failed to generate flamegraph for `{}`",
                args.app_bin.display()
            ),
            err,
        )
    })?;

    report_execution_outcome("simulator", &outcome);
    ui::field("flamegraph", flamegraph_output.display());

    Ok(())
}

pub fn run_transpiler(args: RunTranspilerArgs) -> Result<()> {
    let input_words = input::parse_input_words(&args.input)?;
    let mut builder = airbender_host::TranspilerRunnerBuilder::new(&args.app_bin);
    if let Some(cycle_limit) = args.cycles {
        builder = builder.with_cycles(cycle_limit);
    }
    if let Some(text_path) = args.text_path.as_ref() {
        builder = builder.with_text_path(text_path);
    }
    if args.jit {
        builder = builder.with_jit();
    }
    let runner = builder.build().map_err(|err| {
        CliError::with_source(
            format!(
                "failed to initialize transpiler runner for `{}`",
                args.app_bin.display()
            ),
            err,
        )
    })?;

    let outcome = runner.run(&input_words).map_err(|err| {
        CliError::with_source(
            format!(
                "transpiler execution failed for `{}`",
                args.app_bin.display()
            ),
            err,
        )
    })?;

    report_execution_outcome("transpiler", &outcome);

    Ok(())
}

fn report_execution_outcome(mode: &str, outcome: &airbender_host::ExecutionResult) {
    ui::success(format!("{mode} execution finished"));
    ui::field("cycles", outcome.cycles_executed);
    ui::field("reached_end", outcome.reached_end);
    ui::field("outputs", format_output_registers(&outcome.receipt.output));
}

fn format_output_registers(output: &[u32]) -> String {
    if output.is_empty() {
        return "<none>".to_string();
    }

    let mut registers = String::new();
    for (offset, value) in output.iter().enumerate() {
        use std::fmt::Write;
        let _ = write!(registers, "x{}={} ", offset + 10, value);
    }

    registers.trim_end().to_string()
}
