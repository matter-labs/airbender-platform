use crate::error::{CliError, Result};
use std::fs;
use std::path::Path;

pub fn parse_input_words(path: &Path) -> Result<Vec<u32>> {
    let raw = fs::read_to_string(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => CliError::with_source(
            format!("input file `{}` does not exist", path.display()),
            err,
        )
        .with_hint("provide an existing hex file with `--input <path>`"),
        _ => CliError::with_source(
            format!("failed to read input file `{}`", path.display()),
            err,
        ),
    })?;

    let mut hex: String = raw
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if let Some(stripped) = hex.strip_prefix("0x") {
        hex = stripped.to_string();
    }

    if hex.is_empty() {
        return Ok(Vec::new());
    }

    if !hex.len().is_multiple_of(8) {
        return Err(CliError::new(format!(
            "input hex length must be a multiple of 8 (got {})",
            hex.len()
        ))
        .with_hint("encode each u32 word as exactly 8 hexadecimal characters"));
    }

    let mut words = Vec::with_capacity(hex.len() / 8);
    for chunk in hex.as_bytes().chunks(8) {
        let chunk_str = std::str::from_utf8(chunk)
            .map_err(|err| CliError::with_source("failed to parse input as UTF-8", err))?;
        let word = u32::from_str_radix(chunk_str, 16).map_err(|err| {
            CliError::with_source(format!("failed to parse hex word `{chunk_str}`"), err)
                .with_hint("input must contain hexadecimal characters only")
        })?;
        words.push(word);
    }

    Ok(words)
}
