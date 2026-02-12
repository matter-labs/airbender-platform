use super::{resolve_cycles, ExecutionResult, Runner};
use crate::error::{HostError, Result};
use crate::receipt::Receipt;
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use riscv_transpiler::common_constants::{INITIAL_TIMESTAMP, TIMESTAMP_STEP};
use riscv_transpiler::jit::JittedCode;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Builder for creating a configured transpiler runner.
pub struct TranspilerRunnerBuilder {
    app_bin_path: PathBuf,
    cycles: Option<usize>,
    text_path: Option<PathBuf>,
}

impl TranspilerRunnerBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            cycles: None,
            text_path: None,
        }
    }

    pub fn with_cycles(mut self, cycles: usize) -> Self {
        self.cycles = Some(cycles);
        self
    }

    pub fn with_text_path(mut self, text_path: impl AsRef<Path>) -> Self {
        self.text_path = Some(text_path.as_ref().to_path_buf());
        self
    }

    pub fn build(self) -> Result<TranspilerRunner> {
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
        })
    }
}

/// Transpiler JIT based execution runner.
pub struct TranspilerRunner {
    app_bin_path: PathBuf,
    app_text_path: PathBuf,
    cycles: usize,
}

impl Runner for TranspilerRunner {
    fn run(&self, input_words: &[u32]) -> Result<ExecutionResult> {
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
