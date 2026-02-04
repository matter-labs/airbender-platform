//! Error types surfaced by guest artifact building.

use airbender_core::manifest::ManifestError;
use std::process::ExitStatus;

/// Unified error type for build and packaging operations.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Wraps filesystem and process-spawn I/O errors.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Reports external command failures with their final exit status.
    #[error("command `{cmd}` failed with status {status}")]
    ProcessFailed { cmd: String, status: ExitStatus },

    /// Signals that required metadata was not available.
    #[error("missing field: {0}")]
    MissingField(&'static str),

    /// Signals invalid user inputs or metadata content.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

impl From<ManifestError> for BuildError {
    fn from(err: ManifestError) -> Self {
        match err {
            ManifestError::Io(err) => Self::Io(err),
            _ => Self::InvalidConfig(err.to_string()),
        }
    }
}

/// Convenience result alias for crate APIs.
pub type Result<T> = std::result::Result<T, BuildError>;
