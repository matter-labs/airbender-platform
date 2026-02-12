use crate::cli::{BuildArgs, BuildProfile};
use crate::error::{CliError, Result};
use crate::ui;
use airbender_build::{build_dist, BuildConfig, Profile};

pub fn run(args: BuildArgs) -> Result<()> {
    let BuildArgs {
        app_name,
        bin,
        target,
        dist,
        project,
        profile,
        debug,
        release,
        cargo_args,
    } = args;

    let project_dir = match project {
        Some(path) => path,
        None => std::env::current_dir().map_err(|err| {
            CliError::with_source("failed to resolve current working directory", err)
        })?,
    };

    let manifest_path = project_dir.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Err(CliError::new(format!(
            "guest project `{}` does not contain a Cargo.toml",
            project_dir.display()
        ))
        .with_hint("use --project <path-to-guest-crate>"));
    }

    let mut config = BuildConfig::new(project_dir);
    config.app_name = app_name;
    config.bin_name = bin;
    config.target = target;
    config.dist_dir = dist;
    config.profile = resolve_profile(profile, debug, release);
    config.cargo_args = cargo_args;

    let artifacts = build_dist(&config).map_err(|err| {
        CliError::with_source("failed to build guest artifacts", err)
            .with_hint("set `RUST_LOG=info` if you need backend diagnostic logs")
    })?;

    ui::success("built guest artifacts");
    ui::field("dist", artifacts.dist_dir.display());
    ui::field("app.bin", artifacts.app_bin.display());
    ui::field("app.elf", artifacts.app_elf.display());
    ui::field("app.text", artifacts.app_text.display());
    ui::field("manifest", artifacts.manifest.display());
    ui::blank_line();
    ui::info("next step");
    ui::command(format!(
        "cargo airbender run \"{}\" --input <input.hex>",
        artifacts.app_bin.display()
    ));

    Ok(())
}

fn resolve_profile(profile: Option<BuildProfile>, debug: bool, release: bool) -> Profile {
    if debug {
        return Profile::Debug;
    }
    if release {
        return Profile::Release;
    }
    match profile {
        Some(BuildProfile::Debug) => Profile::Debug,
        Some(BuildProfile::Release) => Profile::Release,
        None => Profile::Release,
    }
}
