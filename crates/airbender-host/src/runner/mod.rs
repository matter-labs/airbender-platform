use crate::error::{HostError, Result};
use crate::receipt::Receipt;

mod simulator_runner;
mod transpiler_runner;

pub use self::simulator_runner::{FlamegraphConfig, SimulatorRunner, SimulatorRunnerBuilder};
pub use self::transpiler_runner::{TranspilerRunner, TranspilerRunnerBuilder};

pub const DEFAULT_CYCLES: usize = 90_000_000_000;

/// Host runner interface.
pub trait Runner {
    fn run(&self, input_words: &[u32]) -> Result<ExecutionResult>;
}

/// Execution outcome for simulator/transpiler based runners.
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub receipt: Receipt,
    pub cycles_executed: usize,
    pub reached_end: bool,
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
