use super::{
    base_path, receipt_from_real_proof, resolve_app_bin_path, ProveResult, Prover, ProverLevel,
};
use crate::error::{HostError, Result};
use crate::proof::{Proof, RealProof};
use crate::security::SecurityLevel;
use execution_utils::unrolled_gpu::UnrolledProver;
use gpu_prover::execution::prover::ExecutionProverConfiguration;
use riscv_transpiler::abstractions::non_determinism::QuasiUARTSource;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread::JoinHandle;

/// Low-level configuration for the GPU prover.
///
/// The default configuration is meant to be optimal in the vast majority of
/// cases, so overriding it is only recommended if you either have specific
/// constraints or you need to fine-tune the configuration for a specific and
/// unusual workload. Each field left unset keeps the prover's own default.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuProverConfig {
    worker_threads: Option<usize>,
    max_device_memory_bytes: Option<usize>,
    host_allocators_per_job: Option<usize>,
    host_allocators_per_device: Option<usize>,
}

impl GpuProverConfig {
    /// Number of worker threads for the prover's thread pool and replay workers.
    pub fn with_worker_threads(mut self, worker_threads: usize) -> Self {
        self.worker_threads = Some(worker_threads);
        self
    }

    /// Like [`Self::with_worker_threads`] but a no-op when `None`.
    pub fn maybe_worker_threads(mut self, worker_threads: Option<usize>) -> Self {
        if let Some(worker_threads) = worker_threads {
            self.worker_threads = Some(worker_threads);
        }
        self
    }

    /// Caps the GPU device allocator at `bytes`. By default the allocator grabs
    /// all free VRAM, which leaves no room for a co-resident prover in the same
    /// process (e.g. the SNARK wrapper). Capping the FRI prover leaves headroom
    /// for it. A cap at or above the free amount is a no-op.
    pub fn with_max_device_memory_bytes(mut self, bytes: usize) -> Self {
        self.max_device_memory_bytes = Some(bytes);
        self
    }

    /// Pinned host transfer buffers pre-allocated per concurrent job (64 MiB
    /// each). Lowering this below the prover default reclaims committed RAM at
    /// the cost of less host<->device pipelining.
    pub fn with_host_allocators_per_job(mut self, count: usize) -> Self {
        self.host_allocators_per_job = Some(count);
        self
    }

    /// Pinned host transfer buffers pre-allocated per GPU device (64 MiB each).
    /// See [`Self::with_host_allocators_per_job`].
    pub fn with_host_allocators_per_device(mut self, count: usize) -> Self {
        self.host_allocators_per_device = Some(count);
        self
    }
}

/// Builder for creating a configured cached GPU prover.
pub struct GpuProverBuilder {
    app_bin_path: PathBuf,
    security: SecurityLevel,
    level: ProverLevel,
    config: GpuProverConfig,
}

impl GpuProverBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            security: SecurityLevel::default(),
            level: ProverLevel::RecursionUnified,
            config: GpuProverConfig::default(),
        }
    }

    pub fn with_level(mut self, level: ProverLevel) -> Self {
        self.level = level;
        self
    }

    pub fn with_security(mut self, security: SecurityLevel) -> Self {
        self.security = security;
        self
    }

    /// Applies low-level configuration for the GPU prover. See
    /// [`GpuProverConfig`]; the default is optimal for the vast majority of
    /// workloads, so this is only needed for specific constraints or unusual
    /// workloads.
    pub fn with_config(mut self, config: GpuProverConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> Result<GpuProver> {
        GpuProver::new(&self.app_bin_path, self.security, self.level, self.config)
    }
}

/// GPU prover wrapper that owns and reuses a single `UnrolledProver` instance.
///
/// ## Poisoning
///
/// Actual proving happens on a separate thread, and in case the program cannot be
/// proven, the prover can panic. Prover panics are not unwind safe, so the thread
/// and the prover will be disposed of, making this prover object poisoned, e.g. not
/// usable for future proving attempts. Once poisoned, the prover will return an error
/// on all operations.
///
/// After poisioning, you can instantiate a new prover if required.
pub struct GpuProver {
    command_tx: mpsc::Sender<WorkerCommand>,
    worker_handle: Mutex<Option<JoinHandle<()>>>,
    poisoned: AtomicBool,
}

enum WorkerCommand {
    Prove {
        input_words: Vec<u32>,
        response_tx: mpsc::Sender<Result<ProveResult>>,
    },
    Shutdown,
}

impl GpuProver {
    fn new(
        app_bin_path: &Path,
        security: SecurityLevel,
        level: ProverLevel,
        config: GpuProverConfig,
    ) -> Result<Self> {
        if matches!(config.worker_threads, Some(0)) {
            return Err(HostError::Prover(
                "worker thread count must be greater than zero".to_string(),
            ));
        }

        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let (command_tx, worker_handle) = spawn_worker(app_bin_path, security, level, config)?;

        Ok(Self {
            command_tx,
            worker_handle: Mutex::new(Some(worker_handle)),
            poisoned: AtomicBool::new(false),
        })
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned.load(Ordering::SeqCst)
    }

    fn poisoned_error() -> HostError {
        HostError::Prover("GPU prover is poisoned due to a previous proving panic".to_string())
    }

    fn handle_worker_failure(&self, operation: &str) -> HostError {
        if self.poisoned.swap(true, Ordering::SeqCst) {
            return Self::poisoned_error();
        }

        match self.take_worker_panic_message() {
            Some(message) => HostError::Prover(format!(
                "GPU prover panicked while {operation}; prover is now poisoned: {message}"
            )),
            None => HostError::Prover(format!(
                "GPU prover worker failed while {operation}; prover is now poisoned"
            )),
        }
    }

    fn take_worker_panic_message(&self) -> Option<String> {
        let mut handle_slot = match self.worker_handle.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        let handle = handle_slot.take()?;

        match handle.join() {
            Ok(()) => None,
            Err(payload) => Some(panic_payload_to_string(payload)),
        }
    }
}

impl Prover for GpuProver {
    fn prove(&self, input_words: &[u32]) -> Result<ProveResult> {
        if self.is_poisoned() {
            return Err(Self::poisoned_error());
        }

        let (response_tx, response_rx) = mpsc::channel();
        self.command_tx
            .send(WorkerCommand::Prove {
                input_words: input_words.to_vec(),
                response_tx,
            })
            .map_err(|_| self.handle_worker_failure("submitting a prove request"))?;

        response_rx
            .recv()
            .map_err(|_| self.handle_worker_failure("receiving a prove response"))?
    }
}

impl Drop for GpuProver {
    fn drop(&mut self) {
        let _ = self.command_tx.send(WorkerCommand::Shutdown);

        let handle_slot = match self.worker_handle.get_mut() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(handle) = handle_slot.take() {
            let _ = handle.join();
        }
    }
}

fn spawn_worker(
    app_bin_path: PathBuf,
    security: SecurityLevel,
    level: ProverLevel,
    config: GpuProverConfig,
) -> Result<(mpsc::Sender<WorkerCommand>, JoinHandle<()>)> {
    let (command_tx, command_rx) = mpsc::channel();
    let (init_tx, init_rx) = mpsc::channel();

    let worker_handle = std::thread::Builder::new()
        .name("airbender-gpu-prover".to_string())
        .spawn(move || gpu_worker_loop(command_rx, init_tx, app_bin_path, security, level, config))
        .map_err(|err| {
            HostError::Prover(format!("failed to spawn GPU prover worker thread: {err}"))
        })?;

    match init_rx.recv() {
        Ok(Ok(())) => Ok((command_tx, worker_handle)),
        Ok(Err(err)) => {
            let _ = worker_handle.join();
            Err(err)
        }
        Err(_) => {
            let reason = match worker_handle.join() {
                Ok(()) => "GPU prover worker exited during initialization".to_string(),
                Err(payload) => format!(
                    "GPU prover worker panicked during initialization: {}",
                    panic_payload_to_string(payload)
                ),
            };
            Err(HostError::Prover(reason))
        }
    }
}

fn gpu_worker_loop(
    command_rx: mpsc::Receiver<WorkerCommand>,
    init_tx: mpsc::Sender<Result<()>>,
    app_bin_path: PathBuf,
    security: SecurityLevel,
    level: ProverLevel,
    config: GpuProverConfig,
) {
    // Keep all prover state inside this dedicated thread so a panic does not unwind
    // through host-call boundaries or require `AssertUnwindSafe`.
    let prover =
        match create_unrolled_prover(&app_bin_path, security, level.as_unrolled_level(), config) {
            Ok(prover) => prover,
            Err(err) => {
                let _ = init_tx.send(Err(err));
                return;
            }
        };

    if init_tx.send(Ok(())).is_err() {
        return;
    }

    let mut next_batch_id_base: u64 = 0;

    while let Ok(command) = command_rx.recv() {
        match command {
            WorkerCommand::Prove {
                input_words,
                response_tx,
            } => {
                let oracle = QuasiUARTSource::new_with_reads(input_words);
                let batch_id_base = next_batch_id_base;
                next_batch_id_base += 1;
                let (inner_proof, cycles) = prover.prove(batch_id_base, oracle);
                let receipt = receipt_from_real_proof(&inner_proof);
                let proof = Proof::Real(RealProof::new(security, level, inner_proof));
                let result = Ok(ProveResult {
                    proof,
                    cycles,
                    receipt,
                });
                let _ = response_tx.send(result);
            }
            WorkerCommand::Shutdown => break,
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn Any + Send + 'static>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }

    "unknown panic payload".to_string()
}

fn create_unrolled_prover(
    app_bin_path: &Path,
    security: SecurityLevel,
    level: execution_utils::unrolled_gpu::UnrolledProverLevel,
    config: GpuProverConfig,
) -> Result<UnrolledProver> {
    let base_path = base_path(app_bin_path)?;
    // Map the host-level GpuProverConfig onto the prover's own configuration,
    // leaving any field the caller did not set at the prover default.
    let mut configuration = ExecutionProverConfiguration::default();
    if let Some(threads) = config.worker_threads {
        configuration.max_thread_pool_threads = Some(threads);
        configuration.replay_worker_threads_count = threads;
    }
    if let Some(count) = config.host_allocators_per_job {
        configuration.host_allocators_per_job_count = count;
    }
    if let Some(count) = config.host_allocators_per_device {
        configuration.host_allocators_per_device_count = count;
    }
    if let Some(bytes) = config.max_device_memory_bytes {
        // The device allocator works in fixed-size blocks; translate the byte cap
        // into a block count. A cap below one block is rejected rather than
        // silently rounded to zero (which would mean "use all free memory").
        let block_log = configuration.prover_context_config.allocator_block_log_size;
        let blocks = bytes >> block_log;
        if blocks == 0 {
            return Err(HostError::Prover(format!(
                "max device memory cap of {bytes} bytes is smaller than one allocator block ({} bytes)",
                1usize << block_log,
            )));
        }
        configuration
            .prover_context_config
            .max_device_allocation_blocks_count = Some(blocks);
    }
    Ok(UnrolledProver::new(
        security.into(),
        &base_path,
        configuration,
        level,
    ))
}
