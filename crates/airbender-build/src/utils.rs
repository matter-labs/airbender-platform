//! Internal helpers for command execution and hashing.

use crate::errors::{BuildError, Result};
use sha2::Digest;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;

/// Runs a command and maps non-success exit codes into [`BuildError`].
pub(crate) fn run_command(mut cmd: Command, name: &str) -> Result<()> {
    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(BuildError::ProcessFailed {
            cmd: name.to_string(),
            status,
        })
    }
}

/// Computes a lowercase hex SHA-256 digest for a file.
pub(crate) fn sha256_file_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let digest = sha2::Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to string cannot fail");
    }
    Ok(encoded)
}
