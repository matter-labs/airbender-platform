use super::profiles::ProverBackendProfile;
use crate::cli::NewAllocatorArg;
use crate::error::{CliError, Result};
use airbender_build::{DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
use serde::Serialize;
use std::fs;
use std::path::Path;
use tera::{Context, Tera};

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

#[derive(Clone, Copy)]
struct TemplateFile<'a> {
    relative_path: &'static str,
    source: &'a str,
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
    let template_data = context.into_template_data();
    let template_context = Context::from_serialize(&template_data)
        .map_err(|err| CliError::with_source("failed to build template context", err))?;
    let template_renderer = template_renderer(profile)?;

    for template in template_files(profile) {
        let destination_path = destination_root.join(template.relative_path);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                CliError::with_source(
                    format!("failed to create directory `{}`", parent.display()),
                    err,
                )
            })?;
        }

        let rendered = render_template(
            &template_renderer,
            template.relative_path,
            &template_context,
        )?;

        fs::write(&destination_path, rendered).map_err(|err| {
            CliError::with_source(
                format!("failed to write `{}`", destination_path.display()),
                err,
            )
        })?;
    }

    Ok(())
}

fn template_files(profile: ProverBackendProfile) -> [TemplateFile<'static>; 9] {
    [
        TemplateFile {
            relative_path: ".gitignore",
            source: GITIGNORE_TEMPLATE,
        },
        TemplateFile {
            relative_path: "README.md",
            source: ROOT_README_TEMPLATE,
        },
        TemplateFile {
            relative_path: "guest/Cargo.toml",
            source: GUEST_CARGO_TEMPLATE,
        },
        TemplateFile {
            relative_path: "guest/src/main.rs",
            source: GUEST_MAIN_TEMPLATE,
        },
        TemplateFile {
            relative_path: "guest/rust-toolchain.toml",
            source: GUEST_TOOLCHAIN_TEMPLATE,
        },
        TemplateFile {
            relative_path: "guest/.cargo/config.toml",
            source: GUEST_CARGO_CONFIG_TEMPLATE,
        },
        TemplateFile {
            relative_path: "host/Cargo.toml",
            source: HOST_CARGO_TEMPLATE,
        },
        TemplateFile {
            relative_path: "host/src/main.rs",
            source: profile.host_main_template,
        },
        TemplateFile {
            relative_path: "host/rust-toolchain.toml",
            source: HOST_TOOLCHAIN_TEMPLATE,
        },
    ]
}

fn template_renderer(profile: ProverBackendProfile) -> Result<Tera> {
    let mut tera = Tera::default();
    for template in template_files(profile) {
        tera.add_raw_template(template.relative_path, template.source)
            .map_err(|err| {
                CliError::with_source(
                    format!("failed to parse template `{}`", template.relative_path),
                    err,
                )
            })?;
    }
    Ok(tera)
}

fn render_template(
    template_renderer: &Tera,
    relative_path: &str,
    context: &Context,
) -> Result<String> {
    template_renderer
        .render(relative_path, context)
        .map_err(|err| {
            CliError::with_source(format!("failed to render template `{relative_path}`"), err)
        })
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
    use tera::Context;

    #[test]
    fn rejects_unknown_placeholder() {
        let mut context = Context::new();
        context.insert("project_name", "demo");

        let mut template_renderer = Tera::default();
        template_renderer
            .add_raw_template("dummy", "{{ project_name }} {{ unknown }}")
            .expect("add test template");

        let err = render_template(&template_renderer, "dummy", &context)
            .expect_err("must fail when an unknown placeholder remains");

        assert!(err
            .to_string()
            .contains("failed to render template `dummy`"));
        assert!(err.source_error().is_some());
    }

    #[test]
    fn renders_plain_rust_braces() {
        let mut template_renderer = Tera::default();
        template_renderer
            .add_raw_template(
                "host/src/main.rs",
                "fn main() {\n    println!(\"hello\");\n}\n",
            )
            .expect("add test template");
        let source = "fn main() {\n    println!(\"hello\");\n}\n";
        let rendered = render_template(&template_renderer, "host/src/main.rs", &Context::new())
            .expect("render template");

        assert_eq!(rendered, source);
    }
}
