mod args;
mod deps;
mod profiles;
mod template;

use crate::cli::NewArgs;
use crate::error::{CliError, Result};
use crate::ui;
use args::resolve_new_args;
use deps::resolve_crate_dependency;
use profiles::prover_backend_profile;
use std::fs;
use std::path::Path;
use template::{write_templates, TemplateContext};

pub fn run(args: NewArgs) -> Result<()> {
    let args = resolve_new_args(args)?;
    let profile = prover_backend_profile(args.prover_backend);

    create_directory(&args.path, "destination")?;

    let guest_destination_dir = args.path.join("guest");
    create_directory(&guest_destination_dir, "guest")?;

    let host_destination_dir = args.path.join("host");
    create_directory(&host_destination_dir, "host")?;

    let sdk_dependency = resolve_crate_dependency(
        &guest_destination_dir,
        args.sdk_path.as_deref(),
        args.sdk_version.as_deref(),
        "airbender-sdk",
    )?;
    let host_dependency = resolve_crate_dependency(
        &host_destination_dir,
        args.sdk_path.as_deref(),
        args.sdk_version.as_deref(),
        "airbender-host",
    )?;

    let template_context = TemplateContext::new(
        &args.project_name,
        &sdk_dependency,
        &host_dependency,
        args.enable_std,
        args.allocator,
        profile.host_dependency_features,
        profile.readme_prover_backend_doc,
    );

    write_templates(&args.path, template_context, profile)?;

    ui::success(format!("created Airbender project `{}`", args.project_name));
    ui::field("path", args.path.display());
    ui::field("guest", guest_destination_dir.display());
    ui::field("host", host_destination_dir.display());
    ui::blank_line();
    ui::info("next steps");
    ui::command(format!("cd \"{}\"", args.path.display()));
    ui::command("cd guest && cargo airbender build");
    ui::command(profile.host_run_command);

    Ok(())
}

fn create_directory(path: &Path, description: &str) -> Result<()> {
    fs::create_dir_all(path).map_err(|err| {
        CliError::with_source(
            format!(
                "failed to create {description} directory `{}`",
                path.display()
            ),
            err,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{NewAllocatorArg, NewProverBackendArg};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const ALL_SCAFFOLD_FILES: &[&str] = &[
        ".gitignore",
        "README.md",
        "guest/.cargo/config.toml",
        "guest/Cargo.toml",
        "guest/rust-toolchain.toml",
        "guest/src/main.rs",
        "host/Cargo.toml",
        "host/rust-toolchain.toml",
        "host/src/main.rs",
    ];

    #[test]
    fn defaults_to_sdk_git_repository() {
        let dependency =
            deps::resolve_crate_dependency(Path::new("."), None, None, "airbender-sdk")
                .expect("resolve default SDK dependency");
        assert_eq!(
            dependency,
            "git = \"https://github.com/matter-labs/airbender-platform\", branch = \"main\""
        );
    }

    #[test]
    fn prefers_explicit_sdk_version() {
        let dependency =
            deps::resolve_crate_dependency(Path::new("."), None, Some("0.1.0"), "airbender-sdk")
                .expect("resolve version SDK dependency");
        assert_eq!(dependency, "version = \"0.1.0\"");
    }

    #[test]
    fn rejects_empty_sdk_version() {
        let err = deps::resolve_crate_dependency(Path::new("."), None, Some(""), "airbender-sdk")
            .expect_err("empty version should fail");
        assert!(err.to_string().contains("--sdk-version"));
    }

    #[test]
    fn resolves_dependency_from_workspace_root() {
        let root = test_workspace_dir("sdk-workspace-root");
        let destination = root.join("destination").join("guest");
        let sdk_workspace = root.join("sdk-workspace");
        let sdk = sdk_workspace.join("crates").join("airbender-sdk");
        let host = sdk_workspace.join("crates").join("airbender-host");

        fs::create_dir_all(&destination).expect("create destination dir");
        fs::create_dir_all(&sdk).expect("create sdk dir");
        fs::create_dir_all(&host).expect("create host dir");
        fs::write(
            sdk.join("Cargo.toml"),
            "[package]\nname = \"airbender-sdk\"\n",
        )
        .expect("write sdk Cargo.toml");
        fs::write(
            host.join("Cargo.toml"),
            "[package]\nname = \"airbender-host\"\n",
        )
        .expect("write host Cargo.toml");

        let dependency = deps::resolve_crate_dependency(
            &destination,
            Some(sdk_workspace.as_path()),
            None,
            "airbender-sdk",
        )
        .expect("resolve path SDK dependency");
        assert_eq!(
            dependency,
            "path = \"../../sdk-workspace/crates/airbender-sdk\""
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn resolves_host_from_sibling_sdk_path() {
        let root = test_workspace_dir("sdk-sibling-host");
        let destination = root.join("destination").join("host");
        let crates_dir = root.join("sdk-workspace").join("crates");
        let sdk = crates_dir.join("airbender-sdk");
        let host = crates_dir.join("airbender-host");

        fs::create_dir_all(&destination).expect("create destination dir");
        fs::create_dir_all(&sdk).expect("create sdk dir");
        fs::create_dir_all(&host).expect("create host dir");
        fs::write(
            sdk.join("Cargo.toml"),
            "[package]\nname = \"airbender-sdk\"\n",
        )
        .expect("write sdk Cargo.toml");
        fs::write(
            host.join("Cargo.toml"),
            "[package]\nname = \"airbender-host\"\n",
        )
        .expect("write host Cargo.toml");

        let dependency = deps::resolve_crate_dependency(
            &destination,
            Some(sdk.as_path()),
            None,
            "airbender-host",
        )
        .expect("resolve host dependency from sibling path");
        assert_eq!(
            dependency,
            "path = \"../../sdk-workspace/crates/airbender-host\""
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn new_scaffolds_host_and_guest() {
        let root = test_workspace_dir("scaffold-host-guest");
        let destination = root.join("hello-airbender");

        run(NewArgs {
            path: Some(destination.clone()),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Talc,
            prover_backend: NewProverBackendArg::Dev,
            yes: true,
            sdk_path: None,
            sdk_version: Some("0.1.0".to_string()),
        })
        .expect("create scaffold");

        assert_rendered_files_snapshot(
            "new_scaffolds_host_and_guest",
            &destination,
            ALL_SCAFFOLD_FILES,
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn new_enable_std_updates_guest_template() {
        let root = test_workspace_dir("scaffold-enable-std");
        let destination = root.join("hello-airbender");

        run(NewArgs {
            path: Some(destination.clone()),
            name: Some("hello-airbender".to_string()),
            enable_std: true,
            allocator: NewAllocatorArg::Talc,
            prover_backend: NewProverBackendArg::Dev,
            yes: true,
            sdk_path: None,
            sdk_version: Some("0.1.0".to_string()),
        })
        .expect("create std scaffold");

        assert_rendered_files_snapshot(
            "new_enable_std_updates_guest_template",
            &destination,
            &["guest/Cargo.toml", "guest/src/main.rs"],
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn new_bump_allocator_disables_sdk_default_features() {
        let root = test_workspace_dir("scaffold-bump-allocator");
        let destination = root.join("hello-airbender");

        run(NewArgs {
            path: Some(destination.clone()),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Bump,
            prover_backend: NewProverBackendArg::Dev,
            yes: true,
            sdk_path: None,
            sdk_version: Some("0.1.0".to_string()),
        })
        .expect("create bump allocator scaffold");

        assert_rendered_files_snapshot(
            "new_bump_allocator_disables_sdk_default_features",
            &destination,
            &["guest/Cargo.toml"],
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn new_custom_allocator_adds_allocator_hook() {
        let root = test_workspace_dir("scaffold-custom-allocator");
        let destination = root.join("hello-airbender");

        run(NewArgs {
            path: Some(destination.clone()),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Custom,
            prover_backend: NewProverBackendArg::Dev,
            yes: true,
            sdk_path: None,
            sdk_version: Some("0.1.0".to_string()),
        })
        .expect("create custom allocator scaffold");

        assert_rendered_files_snapshot(
            "new_custom_allocator_adds_allocator_hook",
            &destination,
            &["guest/Cargo.toml", "guest/src/main.rs"],
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn new_gpu_backend_generates_real_prover_setup() {
        let root = test_workspace_dir("scaffold-gpu-prover");
        let destination = root.join("hello-airbender");

        run(NewArgs {
            path: Some(destination.clone()),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Talc,
            prover_backend: NewProverBackendArg::Gpu,
            yes: true,
            sdk_path: None,
            sdk_version: Some("0.1.0".to_string()),
        })
        .expect("create gpu scaffold");

        assert_rendered_files_snapshot(
            "new_gpu_backend_generates_real_prover_setup",
            &destination,
            &["README.md", "host/Cargo.toml", "host/src/main.rs"],
        );

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    fn assert_rendered_files_snapshot(snapshot_name: &str, root: &Path, relative_paths: &[&str]) {
        let mut rendered = String::new();

        for relative_path in relative_paths {
            let contents = fs::read_to_string(root.join(relative_path))
                .unwrap_or_else(|err| panic!("read rendered file `{relative_path}`: {err}"));
            rendered.push_str(&format!("=== {relative_path} ===\n"));
            rendered.push_str(&contents);
            if !contents.ends_with('\n') {
                rendered.push('\n');
            }
        }

        insta::assert_snapshot!(snapshot_name, rendered);
    }

    fn test_workspace_dir(suffix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tmp")
            .join(format!(
                "cargo-airbender-new-tests-{suffix}-{timestamp}-{}",
                std::process::id()
            ))
    }
}
