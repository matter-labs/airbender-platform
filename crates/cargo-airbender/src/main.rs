mod cli;
mod commands;
mod error;
mod input;
mod ui;

use crate::error::{CliError, Result};

fn main() {
    let cli = cli::Cli::parse_for_cargo();

    if let Err(err) = init_tracing() {
        ui::render_error(&err);
        std::process::exit(1);
    }

    if let Err(err) = commands::run(cli) {
        ui::render_error(&err);
        std::process::exit(1);
    }
}

fn init_tracing() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .try_init()
        .map_err(|err| {
            CliError::with_source(
                "failed to initialize tracing subscriber",
                anyhow::Error::from_boxed(err),
            )
            .with_hint("set RUST_LOG=<level> (for example `RUST_LOG=info`) to inspect backend logs")
        })?;

    Ok(())
}
