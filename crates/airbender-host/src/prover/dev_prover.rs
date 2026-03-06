use super::{resolve_app_bin_path, ProveResult, Prover};
use crate::error::Result;
use crate::proof::{hash_app_bin, hash_input_words, DevProof, Proof};
use crate::runner::{Runner, TranspilerRunner, TranspilerRunnerBuilder};
use std::path::{Path, PathBuf};

/// Builder for creating a configured development prover.
pub struct DevProverBuilder {
    app_bin_path: PathBuf,
    cycles: Option<usize>,
    text_path: Option<PathBuf>,
}

impl DevProverBuilder {
    pub fn new(app_bin_path: impl AsRef<Path>) -> Self {
        Self {
            app_bin_path: app_bin_path.as_ref().to_path_buf(),
            cycles: None,
            text_path: None,
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

    pub fn with_text_path(mut self, text_path: impl AsRef<Path>) -> Self {
        self.text_path = Some(text_path.as_ref().to_path_buf());
        self
    }

    pub fn maybe_text_path(self, text_path: Option<impl AsRef<Path>>) -> Self {
        match text_path {
            Some(v) => self.with_text_path(v),
            None => self,
        }
    }

    pub fn build(self) -> Result<DevProver> {
        DevProver::new(&self.app_bin_path, self.cycles, self.text_path.as_deref())
    }
}

/// Development prover that records transpiler execution metadata instead of generating a zk-proof.
pub struct DevProver {
    app_bin_hash: [u8; 32],
    runner: TranspilerRunner,
}

impl DevProver {
    fn new(app_bin_path: &Path, cycles: Option<usize>, text_path: Option<&Path>) -> Result<Self> {
        let app_bin_path = resolve_app_bin_path(app_bin_path)?;
        let app_bin_hash = hash_app_bin(&app_bin_path)?;

        let runner = TranspilerRunnerBuilder::new(&app_bin_path)
            .maybe_cycles(cycles)
            .maybe_text_path(text_path)
            .build()?;

        Ok(Self {
            app_bin_hash,
            runner,
        })
    }
}

impl Prover for DevProver {
    fn prove(&self, input_words: &[u32]) -> Result<ProveResult> {
        let execution = self.runner.run(input_words)?;
        let cycles = execution.cycles_executed as u64;
        let receipt = execution.receipt;

        let proof = Proof::Dev(DevProof {
            app_bin_hash: self.app_bin_hash,
            input_words_hash: hash_input_words(input_words),
            receipt: receipt.clone(),
            cycles,
        });

        Ok(ProveResult {
            proof,
            cycles,
            receipt,
        })
    }
}
