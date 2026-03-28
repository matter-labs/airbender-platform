use super::profiles::ProverBackendProfile;
use crate::cli::NewAllocatorArg;
use crate::error::{CliError, Result};
use airbender_build::{DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
use serde::Serialize;
use std::fs;
use std::path::Path;
use tinytemplate::TinyTemplate;

const GITIGNORE_TEMPLATE: &str = include_str!("../../../templates/.gitignore.template");
const ROOT_README_TEMPLATE: &str = include_str!("../../../templates/README.md.template");
const GUEST_CARGO_TEMPLATE: &str = include_str!("../../../templates/guest/Cargo.toml.template");
const GUEST_MAIN_TEMPLATE: &str = include_str!("../../../templates/guest/src/main.rs.template");
const GUEST_TOOLCHAIN_TEMPLATE: &str =
    include_str!("../../../templates/guest/rust-toolchain.toml.template");
const GUEST_CARGO_CONFIG_TEMPLATE: &str =
    include_str!("../../../templates/guest/.cargo/config.toml.template");
const HOST_CARGO_TEMPLATE: &str = include_str!("../../../templates/host/Cargo.toml.template");
const HOST_TOOLCHAIN_TEMPLATE: &str =
    include_str!("../../../templates/host/rust-toolchain.toml.template");
const CUSTOM_ALLOCATOR_MODULE_TEMPLATE: &str =
    include_str!("../../../templates/snippets/custom_allocator_module.rs.template");

const TEMPLATES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE_TEMPLATE),
    ("README.md", ROOT_README_TEMPLATE),
    ("guest/Cargo.toml", GUEST_CARGO_TEMPLATE),
    ("guest/src/main.rs", GUEST_MAIN_TEMPLATE),
    ("guest/rust-toolchain.toml", GUEST_TOOLCHAIN_TEMPLATE),
    ("guest/.cargo/config.toml", GUEST_CARGO_CONFIG_TEMPLATE),
    ("host/Cargo.toml", HOST_CARGO_TEMPLATE),
    ("host/rust-toolchain.toml", HOST_TOOLCHAIN_TEMPLATE),
];

#[derive(Serialize)]
struct TemplateData {
    project_name: String,
    sdk_dep: String,
    sdk_default_features: String,
    sdk_features: String,
    host_dep: String,
    host_dep_features: String,
    prover_backend_doc: String,
    guest_attributes: String,
    main_attr_args: String,
    custom_allocator_module: String,
    rust_toolchain_channel: String,
    guest_target: String,
}

pub(super) struct TemplateContext<'a> {
    project_name: &'a str,
    sdk_dependency: &'a str,
    host_dependency: &'a str,
    enable_std: bool,
    allocator: NewAllocatorArg,
    host_dependency_features: &'a str,
    readme_prover_backend_doc: &'a str,
}

impl<'a> TemplateContext<'a> {
    pub(super) fn new(
        project_name: &'a str,
        sdk_dependency: &'a str,
        host_dependency: &'a str,
        enable_std: bool,
        allocator: NewAllocatorArg,
        host_dependency_features: &'a str,
        readme_prover_backend_doc: &'a str,
    ) -> Self {
        Self {
            project_name,
            sdk_dependency,
            host_dependency,
            enable_std,
            allocator,
            host_dependency_features,
            readme_prover_backend_doc,
        }
    }

    fn into_template_data(self) -> TemplateData {
        TemplateData {
            project_name: self.project_name.to_string(),
            sdk_dep: self.sdk_dependency.to_string(),
            sdk_default_features: sdk_default_features(self.allocator).to_string(),
            sdk_features: sdk_features(self.enable_std, self.allocator),
            host_dep: self.host_dependency.to_string(),
            host_dep_features: self.host_dependency_features.to_string(),
            prover_backend_doc: self.readme_prover_backend_doc.to_string(),
            guest_attributes: guest_attributes(self.enable_std).to_string(),
            main_attr_args: main_attr_args(self.allocator).to_string(),
            custom_allocator_module: custom_allocator_module(self.allocator).to_string(),
            rust_toolchain_channel: DEFAULT_GUEST_TOOLCHAIN.to_string(),
            guest_target: DEFAULT_GUEST_TARGET.to_string(),
        }
    }
}

pub(super) fn write_templates(
    destination_root: &Path,
    context: TemplateContext<'_>,
    profile: ProverBackendProfile,
) -> Result<()> {
    let data = context.into_template_data();

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);

    for (name, source) in TEMPLATES {
        tt.add_template(name, source).map_err(|err| {
            CliError::with_source(format!("failed to parse template `{name}`"), err)
        })?;
    }

    for (name, _) in TEMPLATES {
        let destination_path = destination_root.join(name);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                CliError::with_source(
                    format!("failed to create directory `{}`", parent.display()),
                    err,
                )
            })?;
        }

        let rendered = tt.render(name, &data).map_err(|err| {
            CliError::with_source(format!("failed to render template `{name}`"), err)
        })?;

        fs::write(&destination_path, rendered).map_err(|err| {
            CliError::with_source(
                format!("failed to write `{}`", destination_path.display()),
                err,
            )
        })?;
    }

    // The host main template is profile-specific and has no placeholders,
    // so it is written directly without rendering.
    let host_main_path = destination_root.join("host/src/main.rs");
    if let Some(parent) = host_main_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::with_source(
                format!("failed to create directory `{}`", parent.display()),
                err,
            )
        })?;
    }
    fs::write(&host_main_path, profile.host_main_template).map_err(|err| {
        CliError::with_source(
            format!("failed to write `{}`", host_main_path.display()),
            err,
        )
    })?;

    Ok(())
}

fn guest_attributes(enable_std: bool) -> &'static str {
    if enable_std {
        "#![no_main]"
    } else {
        "#![no_std]\n#![no_main]"
    }
}

fn sdk_default_features(allocator: NewAllocatorArg) -> &'static str {
    match allocator {
        NewAllocatorArg::Talc => "",
        NewAllocatorArg::Bump | NewAllocatorArg::Custom => ", default-features = false",
    }
}

fn sdk_features(enable_std: bool, allocator: NewAllocatorArg) -> String {
    let mut sdk_feature_flags = Vec::new();
    if enable_std {
        sdk_feature_flags.push("std");
    }
    match allocator {
        NewAllocatorArg::Talc => {}
        NewAllocatorArg::Bump => sdk_feature_flags.push("allocator-bump"),
        NewAllocatorArg::Custom => sdk_feature_flags.push("allocator-custom"),
    }

    if sdk_feature_flags.is_empty() {
        return String::new();
    }

    let rendered = sdk_feature_flags
        .iter()
        .map(|flag| format!("\"{flag}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(", features = [{rendered}]")
}

fn main_attr_args(allocator: NewAllocatorArg) -> &'static str {
    match allocator {
        NewAllocatorArg::Custom => "(allocator_init = crate::custom_allocator::init)",
        NewAllocatorArg::Talc | NewAllocatorArg::Bump => "",
    }
}

fn custom_allocator_module(allocator: NewAllocatorArg) -> &'static str {
    match allocator {
        NewAllocatorArg::Custom => CUSTOM_ALLOCATOR_MODULE_TEMPLATE,
        NewAllocatorArg::Talc | NewAllocatorArg::Bump => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_placeholder() {
        let mut tt = TinyTemplate::new();
        tt.set_default_formatter(&tinytemplate::format_unescaped);
        tt.add_template("test", "{project_name} {unknown}").unwrap();

        let data = TemplateData {
            project_name: "demo".to_string(),
            sdk_dep: String::new(),
            sdk_default_features: String::new(),
            sdk_features: String::new(),
            host_dep: String::new(),
            host_dep_features: String::new(),
            prover_backend_doc: String::new(),
            guest_attributes: String::new(),
            main_attr_args: String::new(),
            custom_allocator_module: String::new(),
            rust_toolchain_channel: String::new(),
            guest_target: String::new(),
        };

        let err = tt
            .render("test", &data)
            .expect_err("must fail on unknown placeholder");
        let msg = err.to_string();
        assert!(
            msg.contains("unknown"),
            "error should mention the unknown field, got: {msg}"
        );
    }
}
