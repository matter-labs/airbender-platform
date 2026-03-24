use super::{resolve_cycles, ExecutionResult, FlamegraphConfig, Runner};
use crate::error::{HostError, Result};
use crate::receipt::Receipt;
#[cfg(feature = "transpiler")]
use riscv_transpiler::abstractions::non_determinism::QuasiUARTSource;
#[cfg(feature = "transpiler")]
use riscv_transpiler::common_constants::{
    rom::ROM_SECOND_WORD_BITS, INITIAL_TIMESTAMP, TIMESTAMP_STEP,
};
#[cfg(feature = "transpiler")]
use riscv_transpiler::cycle::CycleMarkerHooks;
#[cfg(feature = "transpiler")]
use riscv_transpiler::ir::{preprocess_bytecode, FullUnsignedMachineDecoderConfig};
#[cfg(target_arch = "x86_64")]
#[cfg(feature = "transpiler")]
use riscv_transpiler::jit::JittedCode;
#[cfg(feature = "transpiler")]
use riscv_transpiler::jit::RAM_SIZE;
#[cfg(feature = "transpiler")]
use riscv_transpiler::vm::{
    DelegationsCounters, FlamegraphConfig as VmFlamegraphConfig, RamWithRomRegion, SimpleTape,
    State, VmFlamegraphProfiler, VM,
};
use std::path::{Path, PathBuf};
#[cfg(feature = "transpiler")]
use std::io::Read;

/// Builder for creating a configured transpiler runner.
pub struct TranspilerRunnerBuilder {
    app_bin_path: PathBuf,
    cycles: Option<usize>,
    text_path: Option<PathBuf>,
    flamegraph: Option<FlamegraphConfig>,
    use_jit: bool,
}

impl TranspilerRunnerBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            cycles: None,
            text_path: None,
            flamegraph: None,
            use_jit: false,
        }
    }

    pub fn with_cycles(mut self, cycles: usize) -> Self {
        self.cycles = Some(cycles);
        self
    }

    pub fn maybe_cycles(self, cycles: Option<usize>) -> Self {
        match cycles {
            Some(v) => self.with_cycles(v),
            None => self,
        }
    }

    pub fn with_text_path(mut self, text_path: impl AsRef<Path>) -> Self {
        self.text_path = Some(text_path.as_ref().to_path_buf());
        self
    }

    pub fn maybe_text_path(self, text_path: Option<impl AsRef<Path>>) -> Self {
        match text_path {
            Some(v) => self.with_text_path(v),
            None => self,
        }
    }

    pub fn with_flamegraph(mut self, flamegraph: FlamegraphConfig) -> Self {
        self.flamegraph = Some(flamegraph);
        self
    }

    pub fn with_jit(mut self) -> Self {
        self.use_jit = true;
        self
    }

    pub fn build(self) -> Result<TranspilerRunner> {
        if self.use_jit && cfg!(not(target_arch = "x86_64")) {
            return Err(HostError::Transpiler(
                "JIT execution is only available on x86_64 targets".to_string(),
            ));
        }

        let app_bin_path = resolve_app_bin_path(&self.app_bin_path)?;
        let app_text_path = self
            .text_path
            .as_deref()
            .map(resolve_text_path)
            .unwrap_or_else(|| resolve_text_path(&derive_text_path(&app_bin_path)))?;
        let cycles = resolve_cycles(self.cycles)?;

        Ok(TranspilerRunner {
            app_bin_path,
            app_text_path,
            cycles,
            flamegraph: self.flamegraph,
            use_jit: self.use_jit,
        })
    }
}

/// Transpiler based execution runner.
pub struct TranspilerRunner {
    app_bin_path: PathBuf,
    app_text_path: PathBuf,
    cycles: usize,
    flamegraph: Option<FlamegraphConfig>,
    use_jit: bool,
}

impl Runner for TranspilerRunner {
    #[cfg(feature = "transpiler")]
    fn run(&self, input_words: &[u32]) -> Result<ExecutionResult> {
        if self.flamegraph.is_some() {
            return self.run_without_jit_with_flamegraph(input_words);
        }

        if self.use_jit {
            return self.run_with_jit(input_words);
        }

        self.run_without_jit(input_words)
    }

    #[cfg(not(feature = "transpiler"))]
    fn run(&self, _input_words: &[u32]) -> Result<ExecutionResult> {
        Err(HostError::Transpiler(
            "transpiler support is unavailable without the `transpiler` feature".to_string(),
        ))
    }
}

impl TranspilerRunner {
    #[cfg(target_arch = "x86_64")]
    #[cfg(feature = "transpiler")]
    fn run_with_jit(&self, input_words: &[u32]) -> Result<ExecutionResult> {
        let bin_words = read_u32_words(&self.app_bin_path)?;
        let text_words = read_u32_words(&self.app_text_path)?;
        let mut non_determinism_source = QuasiUARTSource::new_with_reads(input_words.to_vec());

        let cycles_bound = match u32::try_from(self.cycles) {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::warn!(
                    "cycles limit {} exceeds u32::MAX; running transpiler without a cycle bound",
                    self.cycles
                );
                None
            }
        };

        let (state, _memory) = JittedCode::run_alternative_simulator(
            &text_words,
            &mut non_determinism_source,
            &bin_words,
            cycles_bound,
        );
        let cycles_executed = ((state.timestamp - INITIAL_TIMESTAMP) / TIMESTAMP_STEP) as usize;

        Ok(ExecutionResult {
            receipt: Receipt::from_registers(state.registers),
            cycles_executed,
            reached_end: true,
            cycle_markers: None,
        })
    }

    #[cfg(not(target_arch = "x86_64"))]
    #[cfg(feature = "transpiler")]
    fn run_with_jit(&self, _input_words: &[u32]) -> Result<ExecutionResult> {
        Err(HostError::Transpiler(
            "JIT execution is only available on x86_64 targets".to_string(),
        ))
    }

    #[cfg(feature = "transpiler")]
    fn run_without_jit(&self, input_words: &[u32]) -> Result<ExecutionResult> {
        self.run_without_jit_internal(input_words, None)
    }

    #[cfg(feature = "transpiler")]
    fn run_without_jit_with_flamegraph(&self, input_words: &[u32]) -> Result<ExecutionResult> {
        let flamegraph = self
            .flamegraph
            .as_ref()
            .ok_or_else(|| HostError::Transpiler("flamegraph options are missing".to_string()))?;

        let symbols_path = flamegraph
            .elf_path
            .clone()
            .unwrap_or_else(|| derive_elf_path(&self.app_bin_path));
        let mut profiler_config = VmFlamegraphConfig::new(symbols_path, flamegraph.output.clone());
        profiler_config.frequency_recip = flamegraph.sampling_rate;
        profiler_config.reverse_graph = flamegraph.inverse;
        let mut profiler = VmFlamegraphProfiler::new(profiler_config).map_err(|err| {
            HostError::Transpiler(format!("failed to initialize flamegraph profiler: {err}"))
        })?;

        self.run_without_jit_internal(input_words, Some(&mut profiler))
    }

    #[cfg(feature = "transpiler")]
    fn run_without_jit_internal(
        &self,
        input_words: &[u32],
        profiler: Option<&mut VmFlamegraphProfiler>,
    ) -> Result<ExecutionResult> {
        let bin_words = read_u32_words(&self.app_bin_path)?;
        let text_words = read_u32_words(&self.app_text_path)?;
        let instructions = preprocess_bytecode::<FullUnsignedMachineDecoderConfig>(&text_words);
        let instruction_tape = SimpleTape::new(&instructions);
        let mut ram =
            RamWithRomRegion::<{ ROM_SECOND_WORD_BITS }>::from_rom_content(&bin_words, RAM_SIZE);
        let mut state = State::initial_with_counters(DelegationsCounters::default());
        let mut non_determinism_source = QuasiUARTSource::new_with_reads(input_words.to_vec());

        let (reached_end, cycle_markers) = CycleMarkerHooks::with(|| match profiler {
            Some(profiler) => {
                VM::<DelegationsCounters, CycleMarkerHooks>::run_basic_unrolled_with_flamegraph::<
                    _,
                    _,
                    _,
                >(
                    &mut state,
                    &mut ram,
                    &mut (),
                    &instruction_tape,
                    self.cycles,
                    &mut non_determinism_source,
                    profiler,
                )
                .map_err(|err| {
                    HostError::Transpiler(format!("failed to generate flamegraph: {err}"))
                })
            }
            None => Ok(
                VM::<DelegationsCounters, CycleMarkerHooks>::run_basic_unrolled::<_, _, _>(
                    &mut state,
                    &mut ram,
                    &mut (),
                    &instruction_tape,
                    self.cycles,
                    &mut non_determinism_source,
                ),
            ),
        });
        let reached_end = reached_end?;

        let cycles_executed = ((state.timestamp - INITIAL_TIMESTAMP) / TIMESTAMP_STEP) as usize;
        let registers = state.registers.map(|register| register.value);

        Ok(ExecutionResult {
            receipt: Receipt::from_registers(registers),
            cycles_executed,
            reached_end,
            cycle_markers: Some(cycle_markers.into()),
        })
    }
}

fn resolve_app_bin_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Err(HostError::Transpiler(format!(
            "binary not found: {}",
            path.display()
        )));
    }

    path.canonicalize().map_err(|err| {
        HostError::Transpiler(format!(
            "failed to canonicalize binary path {}: {err}",
            path.display()
        ))
    })
}

fn resolve_text_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Err(HostError::Transpiler(format!(
            "text file not found: {}",
            path.display()
        )));
    }

    path.canonicalize().map_err(|err| {
        HostError::Transpiler(format!(
            "failed to canonicalize text path {}: {err}",
            path.display()
        ))
    })
}

fn derive_text_path(bin_path: &Path) -> PathBuf {
    let mut text_path = bin_path.to_path_buf();
    text_path.set_extension("text");
    text_path
}

fn derive_elf_path(bin_path: &Path) -> PathBuf {
    let mut elf_path = bin_path.to_path_buf();
    elf_path.set_extension("elf");
    elf_path
}

#[cfg(feature = "transpiler")]
fn read_u32_words(path: &Path) -> Result<Vec<u32>> {
    let mut file = std::fs::File::open(path).map_err(|err| {
        HostError::Transpiler(format!("failed to open {}: {err}", path.display()))
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|err| {
        HostError::Transpiler(format!("failed to read {}: {err}", path.display()))
    })?;

    if bytes.len() % 4 != 0 {
        return Err(HostError::Transpiler(format!(
            "file length is not a multiple of 4: {}",
            path.display()
        )));
    }

    let mut words = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.as_chunks::<4>().0 {
        words.push(u32::from_le_bytes(*chunk));
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::TranspilerRunnerBuilder;
    use crate::runner::Runner;
    use std::path::Path;

    const MARKER_OPCODE: u32 = 0x7ff01073; // csrrw x0, 2047, x0
    const ADDI_OPCODE: u32 = 0x00100093; // addi x1, x0, 1
    const LOOP_OPCODE: u32 = 0x0000006f; // jal x0, 0

    // TODO: Evaluate how low-level do we want tests to be
    #[test]
    fn collects_cycle_markers_for_interpreter_runs() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let bin_path = dir.path().join("app.bin");
        let text_path = dir.path().join("app.text");
        let program = [MARKER_OPCODE, ADDI_OPCODE, MARKER_OPCODE, LOOP_OPCODE];
        write_program(&bin_path, &program);
        write_program(&text_path, &program);

        let runner = TranspilerRunnerBuilder::new(&bin_path)
            .with_text_path(&text_path)
            .with_cycles(program.len())
            .build()
            .expect("build runner");
        let execution = runner.run(&[]).expect("run program");
        let markers = execution.cycle_markers.expect("cycle markers");

        assert!(execution.reached_end);
        assert_eq!(execution.receipt.registers[1], 1);
        assert_eq!(markers.markers.len(), 2);
        assert!(markers.delegation_counter.is_empty());
        let diff = markers.markers[1].diff(&markers.markers[0]);
        assert_eq!(diff.cycles, 1);
        assert!(diff.delegations.is_empty());
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn jit_runs_do_not_collect_cycle_markers() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let bin_path = dir.path().join("app.bin");
        let text_path = dir.path().join("app.text");
        let program = [ADDI_OPCODE, LOOP_OPCODE];
        write_program(&bin_path, &program);
        write_program(&text_path, &program);

        let runner = TranspilerRunnerBuilder::new(&bin_path)
            .with_text_path(&text_path)
            .with_cycles(program.len())
            .with_jit()
            .build()
            .expect("build runner");
        let execution = runner.run(&[]).expect("run program");

        assert_eq!(execution.receipt.registers[1], 1);
        assert!(execution.cycle_markers.is_none());
    }

    fn write_program(path: &Path, program: &[u32]) {
        let bytes: Vec<u8> = program.iter().flat_map(|word| word.to_le_bytes()).collect();
        std::fs::write(path, bytes).expect("write test program");
    }
}
