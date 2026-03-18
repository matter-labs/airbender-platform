//! Error types surfaced by guest artifact building.

use airbender_core::host::manifest::ManifestError;
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

    /// Docker CLI was not found on PATH.
    #[error("docker not found: install Docker and ensure it is on PATH")]
    DockerNotFound,

    /// Docker daemon is not running or not reachable.
    #[error("docker daemon is not running: start Docker Desktop or the docker service")]
    DockerNotRunning,

    /// `docker build` failed while building the reproducible build image.
    #[error("failed to build Docker image for reproducible build")]
    DockerBuildFailed,

    /// Guest project is missing a Cargo.lock or it was not generated with the container toolchain.
    #[error(
        "Cargo.lock not found in `{project}` or not generated with the container toolchain ({toolchain})\n\
         fix: cargo +{toolchain} generate-lockfile --manifest-path {project}/Cargo.toml"
    )]
    LockfileNotReady {
        project: String,
        toolchain: &'static str,
    },
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
