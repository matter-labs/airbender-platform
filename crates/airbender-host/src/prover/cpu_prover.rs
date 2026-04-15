//! CPU-based prover for Airbender programs.
//!
//! # Warning: High Memory Requirement
//!
//! CPU proving typically requires **96 GB or more of RAM** to complete successfully.
//! Running this prover on machines with insufficient memory will cause the process to crash.
//!
//! # When to use the CPU prover
//!
//! Using CPU proving is **very rarely a good idea**. It is primarily a reference
//! implementation and is most useful for debugging circuit constraints. In almost all
//! cases you want either the **dev prover** (for rapid iteration without real proofs)
//! or the **GPU prover** (for production-grade performance). There should be a very
//! specific reason to use the CPU prover before choosing it over the alternatives.

use super::{
    receipt_from_real_proof, resolve_app_bin_path, resolve_text_path, resolve_worker_threads,
    ProveResult, Prover, DEFAULT_CPU_CYCLE_BOUND, DEFAULT_RAM_BOUND_BYTES,
};
use crate::error::{HostError, Result};
use crate::proof::{Proof, RealProof};
use crate::runner::{Runner, TranspilerRunnerBuilder};
use execution_utils::setups;
use execution_utils::unrolled;
use riscv_transpiler::abstractions::non_determinism::QuasiUARTSource;
use riscv_transpiler::common_constants::rom::ROM_BYTE_SIZE;
use riscv_transpiler::cycle::IMStandardIsaConfigWithUnsignedMulDiv;
use std::path::{Path, PathBuf};

/// Minimum system RAM (in GB) required to run CPU proving without crashing.
const MIN_RAM_GB: u64 = 96;

/// Minimum system RAM in bytes derived from [`MIN_RAM_GB`].
const MIN_RAM_BYTES: u64 = MIN_RAM_GB * 1024 * 1024 * 1024;

/// Environment variable that, when set to `true`, skips the system RAM check.
const MEM_OVERRIDE_ENV: &str = "AIRBENDER_PLATFORM_CPU_PROVER_MEM_OVERRIDE";

fn check_system_ram() -> Result<()> {
    if std::env::var(MEM_OVERRIDE_ENV).as_deref() == Ok("true") {
        return Ok(());
    }

    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total_ram = sys.total_memory();

    if total_ram < MIN_RAM_BYTES {
        let detected_gb = total_ram / (1024 * 1024 * 1024);
        return Err(HostError::Prover(format!(
            "System is expected to have at least {MIN_RAM_GB} GB of ram, but this system only has \
             {detected_gb} GB. On machines with not enough RAM, process might crash. If you want \
             to run it anyway, set `{MEM_OVERRIDE_ENV}=true` and run again"
        )));
    }

    Ok(())
}

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

    pub fn maybe_worker_threads(self, worker_threads: Option<usize>) -> Self {
        match worker_threads {
            Some(v) => self.with_worker_threads(v),
            None => self,
        }
    }

    pub fn with_cycles(mut self, cycles: usize) -> Self {
        self.cycles = Some(cycles);
        self
    }

    pub fn maybe_cycles(self, cycles: Option<usize>) -> Self {
        match cycles {
            Some(v) => self.with_cycles(v),
            None => self,
        }
    }

    pub fn with_ram_bound(mut self, ram_bound: usize) -> Self {
        self.ram_bound = Some(ram_bound);
        self
    }

    pub fn maybe_ram_bound(self, ram_bound: Option<usize>) -> Self {
        match ram_bound {
            Some(v) => self.with_ram_bound(v),
            None => self,
        }
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
        check_system_ram()?;

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
    fn prove(&self, input_words: &[u32]) -> Result<ProveResult> {
        let cycles_bound = match self.cycles {
            Some(value) => value,
            None => {
                let cycle_estimator = TranspilerRunnerBuilder::new(&self.app_bin_path)
                    .with_cycles(DEFAULT_CPU_CYCLE_BOUND)
                    .with_text_path(&self.app_text_path)
                    .build()?;
                let outcome = cycle_estimator.run(input_words)?;
                if !outcome.reached_end {
                    return Err(HostError::Prover(format!(
                        "automatic cycle estimation did not reach program end after {} cycles; provide explicit cycles to prove a bounded run",
                        outcome.cycles_executed
                    )));
                }
                outcome.cycles_executed
            }
        };
        if cycles_bound == 0 {
            return Err(HostError::Prover(
                "cycles bound must be greater than zero".to_string(),
            ));
        }

        let oracle = QuasiUARTSource::new_with_reads(input_words.to_vec());
        let inner_proof = unrolled::prove_unrolled_for_machine_configuration_into_program_proof::<
            IMStandardIsaConfigWithUnsignedMulDiv,
        >(
            &self.binary_u32,
            &self.text_u32,
            cycles_bound,
            oracle,
            self.ram_bound,
            &self.worker,
        );
        let receipt = receipt_from_real_proof(&inner_proof);
        let proof = Proof::Real(RealProof::new(super::ProverLevel::Base, inner_proof));

        Ok(ProveResult {
            proof,
            cycles: cycles_bound as u64,
            receipt,
        })
    }
}
