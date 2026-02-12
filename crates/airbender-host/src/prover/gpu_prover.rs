use super::{
    base_path, ensure_prover_program_match, receipt_from_proof, resolve_app_bin_path, ProveResult,
    Prover, ProverLevel,
};
use crate::error::{HostError, Result};
use execution_utils::unrolled_gpu::UnrolledProver;
use gpu_prover::execution::prover::ExecutionProverConfiguration;
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use std::path::{Path, PathBuf};

/// Builder for creating a configured cached GPU prover.
pub struct GpuProverBuilder {
    app_bin_path: PathBuf,
    worker_threads: Option<usize>,
    level: ProverLevel,
}

impl GpuProverBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            worker_threads: None,
            level: ProverLevel::RecursionUnified,
        }
    }

    pub fn with_worker_threads(mut self, worker_threads: usize) -> Self {
        self.worker_threads = Some(worker_threads);
        self
    }

    pub fn with_level(mut self, level: ProverLevel) -> Self {
        self.level = level;
        self
    }

    pub fn build(self) -> Result<GpuProver> {
        GpuProver::new(&self.app_bin_path, self.worker_threads, self.level)
    }
}

/// GPU prover wrapper that owns and reuses a single `UnrolledProver` instance.
pub struct GpuProver {
    app_bin_path: PathBuf,
    prover: UnrolledProver,
}

impl GpuProver {
    fn new(app_bin_path: &Path, worker_threads: Option<usize>, level: ProverLevel) -> Result<Self> {
        if matches!(worker_threads, Some(0)) {
            return Err(HostError::Prover(
                "worker thread count must be greater than zero".to_string(),
            ));
        }

        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let prover =
            create_unrolled_prover(&app_bin_path, worker_threads, level.as_unrolled_level())?;

        Ok(Self {
            app_bin_path,
            prover,
        })
    }
}

impl Prover for GpuProver {
    fn prove(&self, app_bin_path: &Path, input_words: &[u32]) -> Result<ProveResult> {
        ensure_prover_program_match(&self.app_bin_path, app_bin_path)?;

        let oracle = QuasiUARTSource::new_with_reads(input_words.to_vec());
        let (proof, cycles) = self.prover.prove(0, oracle);
        let receipt = receipt_from_proof(&proof);

        Ok(ProveResult {
            proof,
            cycles,
            receipt,
        })
    }
}

fn create_unrolled_prover(
    app_bin_path: &Path,
    worker_threads: Option<usize>,
    level: execution_utils::unrolled_gpu::UnrolledProverLevel,
) -> Result<UnrolledProver> {
    let base_path = base_path(app_bin_path)?;
    let mut configuration = ExecutionProverConfiguration::default();
    if let Some(threads) = worker_threads {
        configuration.max_thread_pool_threads = Some(threads);
        configuration.replay_worker_threads_count = threads;
    }
    Ok(UnrolledProver::new(&base_path, configuration, level))
}
