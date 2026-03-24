use crate::cli::{GenerateVkArgs, ProverLevelArg, VerifyProofArgs};
use crate::error::{CliError, Result};
use crate::ui;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

pub fn generate(args: GenerateVkArgs) -> Result<()> {
    ensure_gpu_vk_support()?;

    let vk = match args.level {
        ProverLevelArg::RecursionUnified => {
            let vk = airbender_host::compute_unified_vk(&args.app_bin).map_err(|err| {
                CliError::with_source(
                    format!(
                        "failed to compute unified verification keys for `{}`",
                        args.app_bin.display()
                    ),
                    err,
                )
            })?;
            airbender_host::VerificationKey::RealUnified(
                airbender_host::RealUnifiedVerificationKey { vk },
            )
        }
        ProverLevelArg::Base | ProverLevelArg::RecursionUnrolled => {
            let level = as_host_level(args.level);
            let vk = airbender_host::compute_unrolled_vk(&args.app_bin, level).map_err(|err| {
                CliError::with_source(
                    format!(
                        "failed to compute unrolled verification keys for `{}`",
                        args.app_bin.display()
                    ),
                    err,
                )
            })?;
            airbender_host::VerificationKey::RealUnrolled(
                airbender_host::RealUnrolledVerificationKey { level, vk },
            )
        }
    };

    write_bincode(&args.output, &vk)?;

    ui::success("verification keys generated");
    ui::field("level", level_name(args.level));
    ui::field("output", args.output.display());

    Ok(())
}

fn ensure_gpu_vk_support() -> Result<()> {
    #[cfg(any(feature = "gpu-prover", feature = "docs-only"))]
    {
        Ok(())
    }

    #[cfg(not(any(feature = "gpu-prover", feature = "docs-only")))]
    {
        Err(CliError::new(
            "verification key generation requires GPU support in `cargo-airbender`",
        )
        .with_hint(
            "rebuild `cargo-airbender` with default features or pass `--features gpu-prover` to use `generate-vk`",
        ))
    }
}

pub fn verify(args: VerifyProofArgs) -> Result<()> {
    let expected_output_words = parse_expected_output_words(args.expected_output.as_deref())?;

    let proof: airbender_host::Proof = read_bincode(&args.proof).map_err(|err| {
        CliError::with_source(
            format!("failed to decode proof from `{}`", args.proof.display()),
            err,
        )
    })?;

    let vk: airbender_host::VerificationKey = read_bincode(&args.vk).map_err(|err| {
        CliError::with_source(
            format!(
                "failed to decode verification key file `{}`",
                args.vk.display()
            ),
            err,
        )
    })?;

    let level = match &proof {
        airbender_host::Proof::Dev(_) => {
            return Err(CliError::new(
                "detected a dev proof; `cargo airbender verify-proof` supports only real proofs",
            )
            .with_hint(
                "verify dev proofs through `airbender-host` with `Program::dev_verifier()`",
            ));
        }
        airbender_host::Proof::Real(proof) => {
            let expected_output_commit = expected_output_words
                .as_ref()
                .map(|words| words as &dyn airbender_host::Commit);

            airbender_host::verify_real_proof_with_vk(proof, &vk, expected_output_commit)
                .map_err(|err| CliError::with_source("proof verification failed", err))?;
            proof.level()
        }
    };

    if expected_output_words.is_none() {
        tracing::warn!("public outputs were not provided; only proof/VK validity was checked");
    }

    ui::success("proof verified");
    ui::field("level", host_level_name(level));
    if let Some(words) = expected_output_words {
        ui::field("expected_output", format_output_words(&words));
    }

    Ok(())
}

fn parse_expected_output_words(raw: Option<&str>) -> Result<Option<[u32; 8]>> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CliError::new("`--expected-output` cannot be empty")
            .with_hint("provide comma-separated u32 words, for example `--expected-output 42`"));
    }

    let parts: Vec<&str> = trimmed.split(',').collect();
    if parts.len() > 8 {
        return Err(CliError::new(format!(
            "`--expected-output` accepts at most 8 words (got {})",
            parts.len()
        ))
        .with_hint(
            "provide up to 8 comma-separated values for x10..x17; missing words are zero-padded",
        ));
    }

    let mut words = [0u32; 8];
    for (index, token) in parts.into_iter().enumerate() {
        let token = token.trim();
        if token.is_empty() {
            return Err(CliError::new(format!(
                "found an empty word at position {} in `--expected-output`",
                index + 1
            ))
            .with_hint("use comma-separated values like `42,0,0`"));
        }
        words[index] = parse_output_word(token, index + 1)?;
    }

    Ok(Some(words))
}

fn parse_output_word(token: &str, position: usize) -> Result<u32> {
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        if hex.is_empty() {
            return Err(CliError::new(format!(
                "failed to parse output word at position {position}: `{token}`"
            ))
            .with_hint("hex output words must use `0x` followed by one or more hex digits"));
        }

        return u32::from_str_radix(hex, 16)
            .map_err(|err| {
                CliError::with_source(
                    format!("failed to parse output word at position {position}: `{token}`"),
                    err,
                )
            })
            .map_err(|err| err.with_hint("use decimal or 0x-prefixed hexadecimal u32 words"));
    }

    token
        .parse::<u32>()
        .map_err(|err| {
            CliError::with_source(
                format!("failed to parse output word at position {position}: `{token}`"),
                err,
            )
        })
        .map_err(|err| err.with_hint("use decimal or 0x-prefixed hexadecimal u32 words"))
}

fn format_output_words(words: &[u32; 8]) -> String {
    words
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn as_host_level(level: ProverLevelArg) -> airbender_host::ProverLevel {
    match level {
        ProverLevelArg::Base => airbender_host::ProverLevel::Base,
        ProverLevelArg::RecursionUnrolled => airbender_host::ProverLevel::RecursionUnrolled,
        ProverLevelArg::RecursionUnified => airbender_host::ProverLevel::RecursionUnified,
    }
}

fn level_name(level: ProverLevelArg) -> &'static str {
    match level {
        ProverLevelArg::Base => "base",
        ProverLevelArg::RecursionUnrolled => "recursion-unrolled",
        ProverLevelArg::RecursionUnified => "recursion-unified",
    }
}

fn host_level_name(level: airbender_host::ProverLevel) -> &'static str {
    match level {
        airbender_host::ProverLevel::Base => "base",
        airbender_host::ProverLevel::RecursionUnrolled => "recursion-unrolled",
        airbender_host::ProverLevel::RecursionUnified => "recursion-unified",
    }
}

fn read_bincode<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path).map_err(|err| {
        CliError::with_source(format!("failed to read `{}`", path.display()), err)
    })?;
    let (decoded, read_len): (T, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|err| CliError::with_source("failed to decode bincode payload", err))?;

    if read_len != bytes.len() {
        tracing::warn!(
            "bincode decoded {} bytes but file is {} bytes",
            read_len,
            bytes.len()
        );
    }
    Ok(decoded)
}

fn write_bincode<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = bincode::serde::encode_to_vec(value, bincode::config::standard())
        .map_err(|err| CliError::with_source("failed to encode bincode payload", err))?;
    std::fs::write(path, encoded).map_err(|err| {
        CliError::with_source(format!("failed to write `{}`", path.display()), err)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(any(feature = "gpu-prover", feature = "docs-only")))]
    use crate::cli::GenerateVkArgs;
    #[cfg(not(any(feature = "gpu-prover", feature = "docs-only")))]
    use std::path::PathBuf;

    #[cfg(not(any(feature = "gpu-prover", feature = "docs-only")))]
    #[test]
    fn generate_vk_requires_gpu_support() {
        let err = generate(GenerateVkArgs {
            app_bin: PathBuf::from("app.bin"),
            output: PathBuf::from("vk.bin"),
            level: ProverLevelArg::Base,
        })
        .expect_err("generate-vk must require gpu-prover support");

        assert!(
            err.to_string().contains("requires GPU support"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_expected_output_none() {
        let parsed = parse_expected_output_words(None).expect("parse should succeed");
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_expected_output_pads_trailing_words() {
        let parsed = parse_expected_output_words(Some("42")).expect("parse should succeed");
        assert_eq!(parsed, Some([42, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn parse_expected_output_supports_hex_and_spaces() {
        let parsed =
            parse_expected_output_words(Some("0x2a, 0X01, 7")).expect("parse should succeed");
        assert_eq!(parsed, Some([42, 1, 7, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn parse_expected_output_rejects_too_many_words() {
        let err =
            parse_expected_output_words(Some("1,2,3,4,5,6,7,8,9")).expect_err("parse should fail");
        assert!(err.to_string().contains("at most 8 words"));
    }

    #[test]
    fn parse_expected_output_rejects_empty_word() {
        let err = parse_expected_output_words(Some("1,,3")).expect_err("parse should fail");
        assert!(err.to_string().contains("empty word"));
    }

    #[test]
    fn parse_expected_output_rejects_invalid_word() {
        let err = parse_expected_output_words(Some("1,nope")).expect_err("parse should fail");
        assert!(err.to_string().contains("failed to parse output word"));
    }
}
