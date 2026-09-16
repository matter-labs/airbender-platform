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
    ensure_supported_security, program_source, real_prove_result, ProveResult, Prover, ProverLevel,
    DEFAULT_CPU_CYCLE_BOUND, DEFAULT_RAM_BOUND_BYTES,
};
use crate::error::{HostError, Result};
use crate::security::SecurityLevel;
use prover_pipeline::{CpuConfig, GpuConfig, ProgramProver, ProgramProverConfig, ProverBackend};
use riscv_transpiler::common_constants::rom::ROM_BYTE_SIZE;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
    security: SecurityLevel,
    level: ProverLevel,
}

impl CpuProverBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            worker_threads: None,
            cycles: None,
            ram_bound: None,
            security: SecurityLevel::default(),
            level: ProverLevel::Base,
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

    /// Upper bound on the cycles a single proof may replay (default `1 << 31`).
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

    pub fn with_security(mut self, security: SecurityLevel) -> Self {
        self.security = security;
        self
    }

    /// Proof layer to produce. Defaults to [`ProverLevel::Base`]; recursion
    /// levels are supported but very slow on the CPU backend.
    pub fn with_level(mut self, level: ProverLevel) -> Self {
        self.level = level;
        self
    }

    pub fn build(self) -> Result<CpuProver> {
        CpuProver::new(
            &self.app_bin_path,
            self.worker_threads,
            self.cycles,
            self.ram_bound,
            self.security,
            self.level,
        )
    }
}

/// CPU prover wrapper that owns a `prover_pipeline::ProgramProver` on the CPU
/// backend and reuses its worker pool across proofs.
pub struct CpuProver {
    security: SecurityLevel,
    level: ProverLevel,
    prover: Mutex<ProgramProver>,
    next_batch_id: AtomicU64,
}

impl CpuProver {
    fn new(
        app_bin_path: &Path,
        worker_threads: Option<usize>,
        cycles: Option<usize>,
        ram_bound: Option<usize>,
        security: SecurityLevel,
        level: ProverLevel,
    ) -> Result<Self> {
        check_system_ram()?;
        ensure_supported_security(security)?;

        if matches!(worker_threads, Some(0)) {
            return Err(HostError::Prover(
                "worker thread count must be greater than zero".to_string(),
            ));
        }

        let cycles_bound = cycles.unwrap_or(DEFAULT_CPU_CYCLE_BOUND);
        if cycles_bound == 0 {
            return Err(HostError::Prover(
                "cycles bound must be greater than zero".to_string(),
            ));
        }

        let ram_bound = ram_bound.unwrap_or(DEFAULT_RAM_BOUND_BYTES);
        if ram_bound < ROM_BYTE_SIZE {
            return Err(HostError::Prover(format!(
                "ram bound must be at least {} bytes",
                ROM_BYTE_SIZE
            )));
        }

        let source = program_source(app_bin_path)?;
        let config = ProgramProverConfig {
            target: level.as_proof_target(),
            backend: ProverBackend::Cpu,
            cpu: CpuConfig {
                cycles_bound,
                ram_bound,
                worker_threads,
            },
            gpu: GpuConfig::default(),
        };
        let prover = ProgramProver::new(source, config).map_err(HostError::Prover)?;

        Ok(Self {
            security,
            level,
            prover: Mutex::new(prover),
            next_batch_id: AtomicU64::new(0),
        })
    }
}

impl Prover for CpuProver {
    fn prove(&self, input_words: &[u32]) -> Result<ProveResult> {
        let batch_id = self.next_batch_id.fetch_add(1, Ordering::SeqCst);
        let mut prover = self
            .prover
            .lock()
            .map_err(|_| HostError::Prover("CPU prover mutex is poisoned".to_string()))?;
        let artifact = prover
            .prove_words(batch_id, input_words.to_vec())
            .map_err(HostError::Prover)?;
        Ok(real_prove_result(artifact, self.security, self.level))
    }
}
