//! Local host-based guest build using the installed toolchain.

use crate::errors::Result;
use crate::resolver::ResolvedBuildParams;
use crate::utils::run_command;
use airbender_core::manifest::Profile;
use std::{path::Path, process::Command};

/// Executes a guest build directly on the host using the local toolchain.
#[derive(Debug)]
pub(crate) struct LocalBuild<'a> {
    params: &'a ResolvedBuildParams,
}

impl<'a> LocalBuild<'a> {
    pub(crate) fn new(params: &'a ResolvedBuildParams) -> Self {
        Self { params }
    }

    /// Runs `cargo build` and `cargo objcopy` to produce `app.bin`, `app.elf`, and `app.text`.
    pub(crate) fn run(
        &self,
        profile: Profile,
        cargo_args: &Vec<String>,
        extra_config: Option<&str>,
    ) -> Result<()> {
        Self::run_cargo_build(
            profile,
            cargo_args,
            &self.params.project_dir,
            &self.params.bin_name,
            &self.params.target,
            extra_config,
        )?;
        Self::run_cargo_objcopy(
            profile,
            cargo_args,
            &self.params.project_dir,
            &self.params.bin_name,
            &self.params.target,
            extra_config,
            &["-O", "binary"],
            self.params.dist_app.bin(),
        )?;
        Self::run_cargo_objcopy(
            profile,
            cargo_args,
            &self.params.project_dir,
            &self.params.bin_name,
            &self.params.target,
            extra_config,
            &["-R", ".text"],
            self.params.dist_app.elf(),
        )?;
        Self::run_cargo_objcopy(
            profile,
            cargo_args,
            &self.params.project_dir,
            &self.params.bin_name,
            &self.params.target,
            extra_config,
            &["-O", "binary", "--only-section=.text"],
            self.params.dist_app.text(),
        )
    }

    /// Runs `cargo build` for the given binary and target.
    fn run_cargo_build(
        profile: Profile,
        cargo_args: &Vec<String>,
        project_dir: &Path,
        bin_name: &str,
        target: &str,
        extra_config: Option<&str>,
    ) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        if profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("--bin").arg(bin_name);
        cmd.arg("--target").arg(target);
        if let Some(cfg) = extra_config {
            cmd.arg("--config").arg(cfg);
        }
        cmd.args(cargo_args);
        cmd.current_dir(project_dir);

        run_command(cmd, "cargo build")
    }

    /// Runs `cargo objcopy` to generate one concrete output artifact.
    #[allow(clippy::too_many_arguments)]
    fn run_cargo_objcopy(
        profile: Profile,
        cargo_args: &Vec<String>,
        project_dir: &Path,
        bin_name: &str,
        target: &str,
        extra_config: Option<&str>,
        objcopy_args: &[&str],
        output: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.arg("objcopy");
        if profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("--bin").arg(bin_name);
        cmd.arg("--target").arg(target);
        if let Some(cfg) = extra_config {
            cmd.arg("--config").arg(cfg);
        }
        cmd.args(cargo_args);
        cmd.arg("--");
        cmd.args(objcopy_args);
        cmd.arg(output);
        cmd.current_dir(project_dir);

        run_command(cmd, "cargo objcopy")
    }
}
