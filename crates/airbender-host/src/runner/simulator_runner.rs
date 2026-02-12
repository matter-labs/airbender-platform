use super::{resolve_cycles, ExecutionResult, Runner};
use crate::error::{HostError, Result};
use crate::receipt::Receipt;
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use risc_v_simulator::cycle::IMStandardIsaConfig;
use risc_v_simulator::runner::CUSTOM_ENTRY_POINT;
use risc_v_simulator::setup::BaselineWithND;
use risc_v_simulator::sim::{
    BinarySource, DiagnosticsConfig, ProfilerConfig, Simulator, SimulatorConfig,
};
use std::path::{Path, PathBuf};

/// Flamegraph collection options for simulator execution.
#[derive(Clone, Debug)]
pub struct FlamegraphConfig {
    pub output: PathBuf,
    pub sampling_rate: usize,
    pub inverse: bool,
    pub elf_path: Option<PathBuf>,
}

/// Builder for creating a configured simulator runner.
pub struct SimulatorRunnerBuilder {
    app_bin_path: PathBuf,
    cycles: Option<usize>,
    flamegraph: Option<FlamegraphConfig>,
}

impl SimulatorRunnerBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            cycles: None,
            flamegraph: None,
        }
    }

    pub fn with_cycles(mut self, cycles: usize) -> Self {
        self.cycles = Some(cycles);
        self
    }

    pub fn with_flamegraph(mut self, flamegraph: FlamegraphConfig) -> Self {
        self.flamegraph = Some(flamegraph);
        self
    }

    pub fn build(self) -> Result<SimulatorRunner> {
        let app_bin_path = resolve_app_bin_path(&self.app_bin_path)?;
        let cycles = resolve_cycles(self.cycles)?;

        if let Some(flamegraph) = self.flamegraph.as_ref() {
            profiler_diagnostics(&app_bin_path, flamegraph)?;
        }

        Ok(SimulatorRunner {
            app_bin_path,
            cycles,
            flamegraph: self.flamegraph,
        })
    }
}

/// Simulator-based execution runner.
pub struct SimulatorRunner {
    app_bin_path: PathBuf,
    cycles: usize,
    flamegraph: Option<FlamegraphConfig>,
}

impl Runner for SimulatorRunner {
    fn run(&self, input_words: &[u32]) -> Result<ExecutionResult> {
        let diagnostics = self
            .flamegraph
            .as_ref()
            .map(|flamegraph| profiler_diagnostics(&self.app_bin_path, flamegraph))
            .transpose()?;
        run_simulator_with_diagnostics(&self.app_bin_path, input_words, self.cycles, diagnostics)
    }
}

fn run_simulator_with_diagnostics(
    bin_path: &Path,
    input_words: &[u32],
    cycles: usize,
    diagnostics: Option<DiagnosticsConfig>,
) -> Result<ExecutionResult> {
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

fn resolve_app_bin_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Err(HostError::Simulator(format!(
            "binary not found: {}",
            path.display()
        )));
    }

    path.canonicalize().map_err(|err| {
        HostError::Simulator(format!(
            "failed to canonicalize binary path {}: {err}",
            path.display()
        ))
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
