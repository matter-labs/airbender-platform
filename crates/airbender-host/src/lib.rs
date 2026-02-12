//! Host-side APIs for executing, proving, and verifying Airbender programs.

mod error;
mod inputs;
mod program;
mod prover;
mod receipt;
mod runner;
mod vk;

pub use error::{HostError, Result};
pub use inputs::Inputs;
pub use program::Program;
pub use prover::{
    CpuProver, CpuProverBuilder, GpuProver, GpuProverBuilder, ProveResult, Prover, ProverLevel,
};
pub use receipt::Receipt;
pub use runner::{
    resolve_cycles, ExecutionResult, FlamegraphConfig, Runner, SimulatorRunner,
    SimulatorRunnerBuilder, TranspilerRunner, TranspilerRunnerBuilder, DEFAULT_CYCLES,
    MAX_CYCLES_ENV,
};
pub use vk::{
    compute_unified_vk, compute_unrolled_vk, verify_proof, verify_unrolled_proof, UnifiedVk,
    UnrolledVk,
};

pub use execution_utils::unrolled::UnrolledProgramProof;
