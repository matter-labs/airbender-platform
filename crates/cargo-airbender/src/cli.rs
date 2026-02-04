use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cargo-airbender",
    bin_name = "cargo airbender",
    version,
    about = "Airbender cargo subcommand"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Cargo invokes subcommand binaries as `cargo-airbender airbender <args>`.
    /// We strip the synthetic `airbender` token so clap can parse the command list naturally.
    pub fn parse_for_cargo() -> Self {
        let mut args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("airbender") {
            args.remove(1);
        }
        <Self as Parser>::parse_from(args)
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build and package guest artifacts into dist/.
    Build(BuildArgs),
    /// Create a new guest project from the template.
    New(NewArgs),
    /// Run app.bin with the simulator.
    Run(RunArgs),
    /// Run app.bin with simulator profiling and emit flamegraph SVG.
    Flamegraph(FlamegraphArgs),
    /// Run app.bin via the transpiler JIT.
    RunTranspiler(RunTranspilerArgs),
    /// Generate a proof and write it as bincode.
    Prove(ProveArgs),
    /// Generate verification keys and write them as bincode.
    GenerateVk(GenerateVkArgs),
    /// Verify a proof against verification keys.
    VerifyProof(VerifyProofArgs),
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[arg(long, default_value = "app")]
    pub app_name: String,
    #[arg(long)]
    pub bin: Option<String>,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long)]
    pub dist: Option<PathBuf>,
    #[arg(long)]
    pub project: Option<PathBuf>,
    #[arg(long, value_enum, conflicts_with_all = ["debug", "release"])]
    pub profile: Option<BuildProfile>,
    #[arg(long, conflicts_with = "release")]
    pub debug: bool,
    #[arg(long, conflicts_with = "debug")]
    pub release: bool,
    #[arg(last = true, value_name = "CARGO_ARGS")]
    pub cargo_args: Vec<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum BuildProfile {
    Debug,
    Release,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, conflicts_with = "sdk_version")]
    pub sdk_path: Option<PathBuf>,
    #[arg(long, conflicts_with = "sdk_path")]
    pub sdk_version: Option<String>,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    pub app_bin: PathBuf,
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub cycles: Option<usize>,
}

#[derive(Args, Debug)]
pub struct FlamegraphArgs {
    pub app_bin: PathBuf,
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long, default_value = "flamegraph.svg")]
    pub output: PathBuf,
    #[arg(short, long)]
    pub cycles: Option<usize>,
    #[arg(long, default_value_t = 100)]
    pub sampling_rate: usize,
    #[arg(long)]
    pub inverse: bool,
    #[arg(long)]
    pub elf_path: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RunTranspilerArgs {
    pub app_bin: PathBuf,
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub cycles: Option<usize>,
    #[arg(long)]
    pub text_path: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ProveArgs {
    pub app_bin: PathBuf,
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long, value_enum, default_value_t = ProverBackendArg::Gpu)]
    pub backend: ProverBackendArg,
    #[arg(short, long)]
    pub threads: Option<usize>,
    #[arg(long)]
    pub cycles: Option<usize>,
    #[arg(long)]
    pub ram_bound: Option<usize>,
    #[arg(long, value_enum, default_value_t = ProverLevelArg::RecursionUnified)]
    pub level: ProverLevelArg,
}

#[derive(Args, Debug)]
pub struct GenerateVkArgs {
    pub app_bin: PathBuf,
    #[arg(short, long, default_value = "vk.bin")]
    pub output: PathBuf,
    #[arg(long, value_enum, default_value_t = ProverLevelArg::RecursionUnified)]
    pub level: ProverLevelArg,
}

#[derive(Args, Debug)]
pub struct VerifyProofArgs {
    pub proof: PathBuf,
    #[arg(long)]
    pub vk: PathBuf,
    #[arg(long, value_enum, default_value_t = ProverLevelArg::RecursionUnified)]
    pub level: ProverLevelArg,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ProverBackendArg {
    Cpu,
    Gpu,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProverLevelArg {
    Base,
    RecursionUnrolled,
    RecursionUnified,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_run_command() {
        let cli = Cli::parse_from(["cargo-airbender", "run", "app.bin", "--input", "input.hex"]);
        match cli.command {
            Commands::Run(args) => {
                assert_eq!(args.app_bin, PathBuf::from("app.bin"));
                assert_eq!(args.input, PathBuf::from("input.hex"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_build_trailing_cargo_args() {
        let cli = Cli::parse_from([
            "cargo-airbender",
            "build",
            "--",
            "--features",
            "gpu",
            "--color",
            "always",
        ]);
        match cli.command {
            Commands::Build(args) => {
                assert_eq!(args.app_name, "app");
                assert_eq!(
                    args.cargo_args,
                    vec!["--features", "gpu", "--color", "always"]
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_build_custom_app_name() {
        let cli = Cli::parse_from(["cargo-airbender", "build", "--app-name", "gpu-profile"]);
        match cli.command {
            Commands::Build(args) => {
                assert_eq!(args.app_name, "gpu-profile");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_build_rejects_top_level_cargo_flags() {
        let err = Cli::try_parse_from(["cargo-airbender", "build", "--features", "gpu"])
            .expect_err("parse should fail without -- forwarding separator");
        assert!(err.to_string().contains("--features"));
    }
}
