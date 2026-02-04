use crate::cli::{BuildArgs, BuildProfile};
use airbender_build::{build_dist, BuildConfig, Profile};
use anyhow::{Context, Result};

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
        None => std::env::current_dir().context("while attempting to resolve current directory")?,
    };

    let mut config = BuildConfig::new(project_dir);
    config.app_name = app_name;
    config.bin_name = bin;
    config.target = target;
    config.dist_dir = dist;
    config.profile = resolve_profile(profile, debug, release);
    config.cargo_args = cargo_args;

    let artifacts = build_dist(&config)
        .map_err(|err| anyhow::anyhow!("while attempting to build guest artifacts: {err}"))?;
    tracing::info!("dist: {}", artifacts.dist_dir.display());
    tracing::info!("app.bin: {}", artifacts.app_bin.display());
    tracing::info!("app.elf: {}", artifacts.app_elf.display());
    tracing::info!("app.text: {}", artifacts.app_text.display());
    tracing::info!("manifest: {}", artifacts.manifest.display());
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
