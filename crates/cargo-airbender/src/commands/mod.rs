mod build;
mod new;
mod prove;
mod run;
mod vk;

use crate::cli::{Cli, Commands};
use crate::error::Result;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Build(args) => build::run(args),
        Commands::New(args) => new::run(args),
        Commands::Run(args) => run::run(args),
        Commands::Flamegraph(args) => run::flamegraph(args),
        Commands::RunTranspiler(args) => run::run_transpiler(args),
        Commands::Prove(args) => prove::run(args),
        Commands::GenerateVk(args) => vk::generate(args),
        Commands::VerifyProof(args) => vk::verify(args),
    }
}
