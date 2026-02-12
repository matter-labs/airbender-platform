use super::{
    ensure_prover_program_match, receipt_from_proof, resolve_app_bin_path, resolve_text_path,
    resolve_worker_threads, ProveResult, Prover, DEFAULT_CPU_CYCLE_BOUND, DEFAULT_RAM_BOUND_BYTES,
};
use crate::error::{HostError, Result};
use execution_utils::setups;
use execution_utils::unrolled;
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use risc_v_simulator::cycle::IMStandardIsaConfigWithUnsignedMulDiv;
use riscv_transpiler::common_constants::rom::ROM_BYTE_SIZE;
use std::path::{Path, PathBuf};

/// Builder for creating a configured cached CPU prover.
pub struct CpuProverBuilder {
    app_bin_path: PathBuf,
    worker_threads: Option<usize>,
    cycles: Option<usize>,
    ram_bound: Option<usize>,
}

impl CpuProverBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            worker_threads: None,
            cycles: None,
            ram_bound: None,
        }
    }

    pub fn with_worker_threads(mut self, worker_threads: usize) -> Self {
        self.worker_threads = Some(worker_threads);
        self
    }

    pub fn with_cycles(mut self, cycles: usize) -> Self {
        self.cycles = Some(cycles);
        self
    }

    pub fn with_ram_bound(mut self, ram_bound: usize) -> Self {
        self.ram_bound = Some(ram_bound);
        self
    }

    pub fn build(self) -> Result<CpuProver> {
        CpuProver::new(
            &self.app_bin_path,
            self.worker_threads,
            self.cycles,
            self.ram_bound,
        )
    }
}

/// CPU prover wrapper that caches padded artifacts and worker threads.
pub struct CpuProver {
    app_bin_path: PathBuf,
    app_text_path: PathBuf,
    binary_u32: Vec<u32>,
    text_u32: Vec<u32>,
    cycles: Option<usize>,
    ram_bound: usize,
    worker: execution_utils::prover_examples::prover::worker::Worker,
}

impl CpuProver {
    fn new(
        app_bin_path: &Path,
        worker_threads: Option<usize>,
        cycles: Option<usize>,
        ram_bound: Option<usize>,
    ) -> Result<Self> {
        if matches!(worker_threads, Some(0)) {
            return Err(HostError::Prover(
                "worker thread count must be greater than zero".to_string(),
            ));
        }

        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let app_text_path = resolve_text_path(&app_bin_path)?;
        let (_, binary_u32) = setups::read_and_pad_binary(&app_bin_path);
        let (_, text_u32) = setups::read_and_pad_binary(&app_text_path);

        let ram_bound = ram_bound.unwrap_or(DEFAULT_RAM_BOUND_BYTES);
        if ram_bound < ROM_BYTE_SIZE {
            return Err(HostError::Prover(format!(
                "ram bound must be at least {} bytes",
                ROM_BYTE_SIZE
            )));
        }

        let threads = resolve_worker_threads(worker_threads);
        let worker =
            execution_utils::prover_examples::prover::worker::Worker::new_with_num_threads(threads);

        Ok(Self {
            app_bin_path,
            app_text_path,
            binary_u32,
            text_u32,
            cycles,
            ram_bound,
            worker,
        })
    }
}

impl Prover for CpuProver {
    fn prove(&self, app_bin_path: &Path, input_words: &[u32]) -> Result<ProveResult> {
        ensure_prover_program_match(&self.app_bin_path, app_bin_path)?;

        let cycles_bound = match self.cycles {
            Some(value) => value,
            None => {
                let outcome = crate::transpiler::run_transpiler(
                    &self.app_bin_path,
                    input_words,
                    DEFAULT_CPU_CYCLE_BOUND,
                    Some(&self.app_text_path),
                )?;
                outcome.cycles_executed
            }
        };
        if cycles_bound == 0 {
            return Err(HostError::Prover(
                "cycles bound must be greater than zero".to_string(),
            ));
        }

        let oracle = QuasiUARTSource::new_with_reads(input_words.to_vec());
        let proof = unrolled::prove_unrolled_for_machine_configuration_into_program_proof::<
            IMStandardIsaConfigWithUnsignedMulDiv,
        >(
            &self.binary_u32,
            &self.text_u32,
            cycles_bound,
            oracle,
            self.ram_bound,
            &self.worker,
        );
        let receipt = receipt_from_proof(&proof);

        Ok(ProveResult {
            proof,
            cycles: cycles_bound as u64,
            receipt,
        })
    }
}
