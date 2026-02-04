use crate::error::{HostError, Result};
use crate::receipt::Receipt;
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use risc_v_simulator::cycle::IMStandardIsaConfig;
use risc_v_simulator::runner::CUSTOM_ENTRY_POINT;
use risc_v_simulator::setup::BaselineWithND;
use risc_v_simulator::sim::{
    BinarySource, DiagnosticsConfig, ProfilerConfig, Simulator, SimulatorConfig,
};
use std::env::VarError;
use std::path::{Path, PathBuf};

pub const DEFAULT_CYCLES: usize = 100_000_000;
pub const MAX_CYCLES_ENV: &str = "AIRBENDER_MAX_CYCLES";

/// Flamegraph collection options for simulator execution.
#[derive(Clone, Debug)]
pub struct FlamegraphConfig {
    pub output: PathBuf,
    pub sampling_rate: usize,
    pub inverse: bool,
    pub elf_path: Option<PathBuf>,
}

/// Simulator execution outcome.
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub receipt: Receipt,
    pub cycles_executed: usize,
    pub reached_end: bool,
}

/// Resolve the simulator cycle budget from an explicit override or environment.
pub fn resolve_cycles(explicit_cycles: Option<usize>) -> Result<usize> {
    if let Some(cycles) = explicit_cycles {
        if cycles == 0 {
            return Err(HostError::Simulator(
                "cycle budget must be greater than zero".to_string(),
            ));
        }
        return Ok(cycles);
    }

    match std::env::var(MAX_CYCLES_ENV) {
        Ok(value) => parse_cycles(&value).map_err(|reason| {
            HostError::Simulator(format!(
                "invalid {MAX_CYCLES_ENV} value `{value}`: {reason}"
            ))
        }),
        Err(VarError::NotPresent) => Ok(DEFAULT_CYCLES),
        Err(err) => Err(HostError::Simulator(format!(
            "failed to read {MAX_CYCLES_ENV}: {err}"
        ))),
    }
}

fn parse_cycles(raw: &str) -> std::result::Result<usize, &'static str> {
    let cycles = raw
        .trim()
        .parse::<usize>()
        .map_err(|_| "expected an integer")?;
    if cycles == 0 {
        return Err("must be greater than zero");
    }
    Ok(cycles)
}

pub fn run_simulator(
    bin_path: &Path,
    input_words: &[u32],
    cycles: usize,
) -> Result<ExecutionResult> {
    run_simulator_with_diagnostics(bin_path, input_words, cycles, None)
}

/// Run the simulator and emit a flamegraph profile.
pub fn run_simulator_with_flamegraph(
    bin_path: &Path,
    input_words: &[u32],
    cycles: usize,
    flamegraph: &FlamegraphConfig,
) -> Result<ExecutionResult> {
    let diagnostics = profiler_diagnostics(bin_path, flamegraph)?;
    run_simulator_with_diagnostics(bin_path, input_words, cycles, Some(diagnostics))
}

fn run_simulator_with_diagnostics(
    bin_path: &Path,
    input_words: &[u32],
    cycles: usize,
    diagnostics: Option<DiagnosticsConfig>,
) -> Result<ExecutionResult> {
    if !bin_path.exists() {
        return Err(HostError::Simulator(format!(
            "binary not found: {}",
            bin_path.display()
        )));
    }
    let config = SimulatorConfig::new(
        BinarySource::Path(bin_path.to_path_buf()),
        CUSTOM_ENTRY_POINT,
        cycles,
        diagnostics,
    );
    let non_determinism_source = QuasiUARTSource::new_with_reads(input_words.to_vec());
    let setup = BaselineWithND::<_, IMStandardIsaConfig>::new(non_determinism_source);
    let mut sim = Simulator::<_, IMStandardIsaConfig>::new(config, setup);
    let mut last_cycle = 0usize;
    let result = sim.run(|_, _| {}, |_, cycle| last_cycle = cycle);
    let cycles_executed = if result.reached_end {
        last_cycle.saturating_add(1)
    } else {
        cycles
    };

    Ok(ExecutionResult {
        receipt: Receipt::from_registers(result.state.registers),
        cycles_executed,
        reached_end: result.reached_end,
    })
}

fn profiler_diagnostics(
    bin_path: &Path,
    flamegraph: &FlamegraphConfig,
) -> Result<DiagnosticsConfig> {
    if flamegraph.sampling_rate == 0 {
        return Err(HostError::Simulator(
            "sampling rate must be greater than zero".to_string(),
        ));
    }

    let symbols_path = flamegraph
        .elf_path
        .clone()
        .unwrap_or_else(|| derive_elf_path(bin_path));
    if !symbols_path.exists() {
        return Err(HostError::Simulator(format!(
            "ELF file not found: {}",
            symbols_path.display()
        )));
    }

    let mut diagnostics = DiagnosticsConfig::new(symbols_path);
    let mut profiler = ProfilerConfig::new(flamegraph.output.clone());
    profiler.frequency_recip = flamegraph.sampling_rate;
    profiler.reverse_graph = flamegraph.inverse;
    diagnostics.profiler_config = Some(profiler);
    Ok(diagnostics)
}

fn derive_elf_path(bin_path: &Path) -> PathBuf {
    let mut elf_path = bin_path.to_path_buf();
    elf_path.set_extension("elf");
    elf_path
}

#[cfg(test)]
mod tests {
    use super::parse_cycles;

    #[test]
    fn parse_cycles_accepts_positive_integer() {
        assert_eq!(parse_cycles("100").expect("cycles"), 100);
    }

    #[test]
    fn parse_cycles_rejects_zero() {
        let err = parse_cycles("0").expect_err("error");
        assert_eq!(err, "must be greater than zero");
    }

    #[test]
    fn parse_cycles_rejects_non_numeric() {
        let err = parse_cycles("abc").expect_err("error");
        assert_eq!(err, "expected an integer");
    }
}
