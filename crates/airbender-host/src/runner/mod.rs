use crate::error::{HostError, Result};
use crate::receipt::Receipt;
use std::env::VarError;

mod simulator_runner;
mod transpiler_runner;

pub use self::simulator_runner::{FlamegraphConfig, SimulatorRunner, SimulatorRunnerBuilder};
pub use self::transpiler_runner::{TranspilerRunner, TranspilerRunnerBuilder};

pub const DEFAULT_CYCLES: usize = 90_000_000_000;
pub const MAX_CYCLES_ENV: &str = "AIRBENDER_MAX_CYCLES";

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

/// Resolve the cycle budget from an explicit override or environment.
pub fn resolve_cycles(explicit_cycles: Option<usize>) -> Result<usize> {
    if let Some(cycles) = explicit_cycles {
        if cycles == 0 {
            return Err(HostError::Runner(
                "cycle budget must be greater than zero".to_string(),
            ));
        }
        return Ok(cycles);
    }

    match std::env::var(MAX_CYCLES_ENV) {
        Ok(value) => parse_cycles(&value).map_err(|reason| {
            HostError::Runner(format!(
                "invalid {MAX_CYCLES_ENV} value `{value}`: {reason}"
            ))
        }),
        Err(VarError::NotPresent) => Ok(DEFAULT_CYCLES),
        Err(err) => Err(HostError::Runner(format!(
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
