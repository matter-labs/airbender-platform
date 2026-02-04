//! Host-side APIs for executing, proving, and verifying Airbender programs.

mod error;
mod inputs;
mod program;
mod prover;
mod receipt;
mod sim;
mod transpiler;
mod vk;

pub use error::{HostError, Result};
pub use inputs::Inputs;
pub use program::Program;
pub use prover::{
    prove, prove_with_options, ProveOptions, ProveResult, ProverBackend, ProverLevel,
};
pub use receipt::Receipt;
pub use sim::{
    resolve_cycles, run_simulator, run_simulator_with_flamegraph, ExecutionResult,
    FlamegraphConfig, DEFAULT_CYCLES, MAX_CYCLES_ENV,
};
pub use transpiler::run_transpiler;
pub use vk::{
    compute_unified_vk, compute_unrolled_vk, verify_proof, verify_unrolled_proof, UnifiedVk,
    UnrolledVk,
};

pub use execution_utils::unrolled::UnrolledProgramProof;
