use crate::error::{HostError, Result};
use crate::receipt::Receipt;
use crate::sim::ExecutionResult;
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use riscv_transpiler::common_constants::{INITIAL_TIMESTAMP, TIMESTAMP_STEP};
use riscv_transpiler::jit::JittedCode;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Run the transpiler JIT and return an execution result compatible with simulator output.
pub fn run_transpiler(
    bin_path: &Path,
    input_words: &[u32],
    cycles: usize,
    text_path: Option<&Path>,
) -> Result<ExecutionResult> {
    if !bin_path.exists() {
        return Err(HostError::Transpiler(format!(
            "binary not found: {}",
            bin_path.display()
        )));
    }

    let text_path = text_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| derive_text_path(bin_path));
    if !text_path.exists() {
        return Err(HostError::Transpiler(format!(
            "text file not found: {}",
            text_path.display()
        )));
    }

    let bin_words = read_u32_words(bin_path)?;
    let text_words = read_u32_words(&text_path)?;
    let mut non_determinism_source = QuasiUARTSource::new_with_reads(input_words.to_vec());

    let cycles_bound = match u32::try_from(cycles) {
        Ok(value) => Some(value),
        Err(_) => {
            tracing::warn!(
                "cycles limit {} exceeds u32::MAX; running transpiler without a cycle bound",
                cycles
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
