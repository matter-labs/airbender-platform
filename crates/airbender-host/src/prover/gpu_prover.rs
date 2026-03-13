use super::{
    base_path, receipt_from_real_proof, resolve_app_bin_path, ProveResult, Prover, ProverLevel,
};
use crate::error::{HostError, Result};
use crate::proof::{Proof, RealProof};
use risc_v_simulator::abstractions::non_determinism::QuasiUARTSource;
use execution_utils::unrolled_gpu::UnrolledProver;
use gpu_prover::execution::prover::ExecutionProverConfiguration;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread::JoinHandle;

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

    pub fn maybe_worker_threads(self, worker_threads: Option<usize>) -> Self {
        match worker_threads {
            Some(v) => self.with_worker_threads(v),
            None => self,
        }
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
    fn new(app_bin_path: &Path, worker_threads: Option<usize>, level: ProverLevel) -> Result<Self> {
        if matches!(worker_threads, Some(0)) {
            return Err(HostError::Prover(
                "worker thread count must be greater than zero".to_string(),
            ));
        }

        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let (command_tx, worker_handle) = spawn_worker(app_bin_path, worker_threads, level)?;

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
    worker_threads: Option<usize>,
    level: ProverLevel,
) -> Result<(mpsc::Sender<WorkerCommand>, JoinHandle<()>)> {
    let (command_tx, command_rx) = mpsc::channel();
    let (init_tx, init_rx) = mpsc::channel();

    let worker_handle = std::thread::Builder::new()
        .name("airbender-gpu-prover".to_string())
        .spawn(move || gpu_worker_loop(command_rx, init_tx, app_bin_path, worker_threads, level))
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
    worker_threads: Option<usize>,
    level: ProverLevel,
) {
    // Keep all prover state inside this dedicated thread so a panic does not unwind
    // through host-call boundaries or require `AssertUnwindSafe`.
    let prover =
        match create_unrolled_prover(&app_bin_path, worker_threads, level.as_unrolled_level()) {
            Ok(prover) => prover,
            Err(err) => {
                let _ = init_tx.send(Err(err));
                return;
            }
        };

    if init_tx.send(Ok(())).is_err() {
        return;
    }

    while let Ok(command) = command_rx.recv() {
        match command {
            WorkerCommand::Prove {
                input_words,
                response_tx,
            } => {
                let oracle = QuasiUARTSource::new_with_reads(input_words);
                // TODO: we use `batch 0` for all the jobs, which can cause issues when generating multiple proofs in parallel.
                let (inner_proof, cycles) = prover.prove(0, oracle);
                let receipt = receipt_from_real_proof(&inner_proof);
                let proof = Proof::Real(RealProof::new(level, inner_proof));
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
