use anyhow::{Context, Result};

mod cli;
mod commands;
mod input;

fn main() -> Result<()> {
    init_tracing().context("while attempting to initialize tracing")?;
    let cli = cli::Cli::parse_for_cargo();
    commands::run(cli).context("while attempting to execute command")
}

fn init_tracing() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
        .map_err(|err| anyhow::anyhow!("failed to initialize tracing subscriber: {err}"))?;
    Ok(())
}
