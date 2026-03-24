//! Host-side APIs for executing, proving, and verifying Airbender programs.

mod cycle_marker;
mod error;
mod inputs;
mod program;
mod proof;
mod prover;
mod receipt;
mod runner;
mod verifier;
mod vk;

pub use airbender_core::guest::Commit;
pub use cycle_marker::{CycleMarker, Mark};
pub use error::{HostError, Result};
pub use inputs::Inputs;
pub use program::Program;
pub use proof::{DevProof, Proof, RealProof};
pub use prover::{
    CpuProver, CpuProverBuilder, DevProver, DevProverBuilder, ProveResult, Prover, ProverLevel,
};
#[cfg(feature = "gpu-prover")]
pub use prover::{GpuProver, GpuProverBuilder};
pub use receipt::Receipt;
pub use runner::{
    resolve_cycles, ExecutionResult, FlamegraphConfig, Runner, TranspilerRunner,
    TranspilerRunnerBuilder, DEFAULT_CYCLES,
};
pub use verifier::{
    verify_real_proof_with_vk, DevVerificationKey, DevVerifier, DevVerifierBuilder,
    RealUnifiedVerificationKey, RealUnrolledVerificationKey, RealVerifier, RealVerifierBuilder,
    VerificationKey, VerificationRequest, Verifier,
};
pub use vk::{
    compute_unified_vk, compute_unrolled_vk, verify_proof, verify_unrolled_proof, UnifiedVk,
    UnrolledVk,
};

/// Raw Airbender re-exports without stability guarantees.
///
/// These items are not recommended for normal use. They are exposed for rare
/// cases, for example when a project depends on both `airbender-host` and
/// direct Airbender crates at the same time.
pub mod raw {
    pub use execution_utils::unrolled::UnrolledProgramProof;
}
