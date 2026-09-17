use crate::cycle_marker::CycleMarker;
use crate::error::{HostError, Result};
use crate::receipt::Receipt;
use std::path::PathBuf;

mod transpiler_runner;

pub use self::transpiler_runner::{TranspilerRunner, TranspilerRunnerBuilder};

/// Flamegraph collection options for execution runners.
#[derive(Clone, Debug)]
pub struct FlamegraphConfig {
    pub output: PathBuf,
    pub sampling_rate: usize,
    pub inverse: bool,
    pub elf_path: Option<PathBuf>,
}

pub const DEFAULT_CYCLES: usize = 90_000_000_000;

/// Host runner interface.
pub trait Runner {
    fn run(&self, input_words: &[u32]) -> Result<ExecutionResult>;
}

/// Execution outcome for transpiler based runners.
#[derive(Debug)]
pub struct ExecutionResult {
    pub receipt: Receipt,
    pub cycles_executed: usize,
    pub reached_end: bool,
    /// Program counter at the end of the run. A program that stops in the
    /// canonical exit sequence (see [`find_exit_pc`]) succeeded; a program
    /// parked in another self-loop (an error handler) has `reached_end` set
    /// too but a different `final_pc`.
    pub final_pc: u32,
    pub cycle_markers: Option<CycleMarker>,
}

/// Locate the canonical exit sequence in a program image and return the PC
/// of its final self-loop, i.e. the `final_pc` of a successful run.
///
/// Returns `None` when the image contains no (or more than one) exit sequence.
pub fn find_exit_pc(bin_words: &[u32]) -> Option<u32> {
    let sequence = riscv_common::EXIT_SEQUENCE;
    let mut found = None;
    for (start, window) in bin_words.windows(sequence.len()).enumerate() {
        if window == sequence {
            if found.is_some() {
                return None;
            }
            found = Some(((start + sequence.len() - 1) * 4) as u32);
        }
    }
    found
}

/// Resolve the cycle budget from an explicit override or default.
pub fn resolve_cycles(explicit_cycles: Option<usize>) -> Result<usize> {
    let cycles = explicit_cycles.unwrap_or(DEFAULT_CYCLES);
    if cycles == 0 {
        return Err(HostError::Runner(
            "cycle budget must be greater than zero".to_string(),
        ));
    }
    Ok(cycles)
}

#[cfg(test)]
mod tests {
    use super::{resolve_cycles, DEFAULT_CYCLES};

    #[test]
    fn resolve_cycles_uses_explicit_value() {
        assert_eq!(resolve_cycles(Some(100)).expect("cycles"), 100);
    }

    #[test]
    fn resolve_cycles_uses_default_when_unspecified() {
        assert_eq!(resolve_cycles(None).expect("cycles"), DEFAULT_CYCLES);
    }

    #[test]
    fn resolve_cycles_rejects_zero() {
        let err = resolve_cycles(Some(0)).expect_err("error");
        assert_eq!(
            err.to_string(),
            "runner error: cycle budget must be greater than zero"
        );
    }
}
