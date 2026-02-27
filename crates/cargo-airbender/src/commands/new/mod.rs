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
    use airbender_build::DEFAULT_GUEST_TOOLCHAIN;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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

        let root_readme =
            fs::read_to_string(destination.join("README.md")).expect("read project root README");
        let root_gitignore = fs::read_to_string(destination.join(".gitignore"))
            .expect("read project root .gitignore");
        let guest_cargo =
            fs::read_to_string(destination.join("guest/Cargo.toml")).expect("read guest Cargo");
        let guest_main =
            fs::read_to_string(destination.join("guest/src/main.rs")).expect("read guest main");
        let guest_toolchain = fs::read_to_string(destination.join("guest/rust-toolchain.toml"))
            .expect("read guest rust-toolchain");
        let guest_cargo_config = fs::read_to_string(destination.join("guest/.cargo/config.toml"))
            .expect("read guest cargo config");
        let host_cargo =
            fs::read_to_string(destination.join("host/Cargo.toml")).expect("read host Cargo");
        let host_main =
            fs::read_to_string(destination.join("host/src/main.rs")).expect("read host main");
        let host_toolchain = fs::read_to_string(destination.join("host/rust-toolchain.toml"))
            .expect("read host rust-toolchain");

        assert!(root_readme.contains("Default prover backend: `dev`"));
        assert!(root_readme.contains("mock proof envelope"));
        assert!(root_gitignore.contains("target/"));
        assert!(guest_cargo.contains("name = \"hello-airbender-guest\""));
        assert!(guest_cargo.contains("airbender-sdk"));
        assert!(guest_main.contains("#![no_std]"));
        assert!(guest_toolchain.contains(&format!("channel = \"{}\"", DEFAULT_GUEST_TOOLCHAIN)));
        assert!(guest_toolchain
            .contains("components = [\"clippy\", \"rust-src\", \"llvm-tools-preview\"]"));
        assert!(guest_cargo_config.contains("target = \"riscv32im-risc0-zkvm-elf\""));
        assert!(guest_cargo_config
            .contains("build-std = [\"alloc\", \"core\", \"panic_abort\", \"compiler_builtins\", \"std\", \"proc_macro\"]"));
        assert!(host_cargo.contains("name = \"hello-airbender-host\""));
        assert!(host_cargo.contains("airbender-host"));
        assert!(host_cargo.contains("default-features = false"));
        assert!(host_cargo.contains("[profile.dev.package.keccak_special5]"));
        assert!(host_cargo.contains("[profile.release.package.setups]"));
        assert!(host_main.contains("Program::load"));
        assert!(host_main.contains("program.dev_prover()"));
        assert!(host_main.contains("program.dev_verifier()"));
        assert!(host_toolchain.contains(&format!("channel = \"{}\"", DEFAULT_GUEST_TOOLCHAIN)));
        assert!(!host_toolchain.contains("components"));

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

        let guest_cargo =
            fs::read_to_string(destination.join("guest/Cargo.toml")).expect("read guest Cargo");
        let guest_main =
            fs::read_to_string(destination.join("guest/src/main.rs")).expect("read guest main");

        assert!(guest_cargo.contains("features = [\"std\"]"));
        assert!(!guest_main.contains("#![no_std]"));

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

        let guest_cargo =
            fs::read_to_string(destination.join("guest/Cargo.toml")).expect("read guest Cargo");

        assert!(guest_cargo.contains("default-features = false"));
        assert!(guest_cargo.contains("features = [\"allocator-bump\"]"));

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

        let guest_cargo =
            fs::read_to_string(destination.join("guest/Cargo.toml")).expect("read guest Cargo");
        let guest_main =
            fs::read_to_string(destination.join("guest/src/main.rs")).expect("read guest main");

        assert!(guest_cargo.contains("default-features = false"));
        assert!(guest_cargo.contains("features = [\"allocator-custom\"]"));
        assert!(guest_main
            .contains("#[airbender::main(allocator_init = crate::custom_allocator::init)]"));
        assert!(guest_main.contains("mod custom_allocator"));

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

        let root_readme =
            fs::read_to_string(destination.join("README.md")).expect("read project root README");
        let host_cargo =
            fs::read_to_string(destination.join("host/Cargo.toml")).expect("read host Cargo");
        let host_main =
            fs::read_to_string(destination.join("host/src/main.rs")).expect("read host main");

        assert!(root_readme.contains("Default prover backend: `gpu`"));
        assert!(root_readme.contains("CUDA-capable NVIDIA GPU"));
        assert!(root_readme.contains("ZKSYNC_USE_CUDA_STUBS=true"));
        assert!(!host_cargo.contains("default-features = false"));
        assert!(host_main.contains(".gpu_prover()"));
        assert!(host_main.contains("program.real_verifier(ProverLevel::RecursionUnified)"));
        assert!(host_main.contains("VerificationRequest::real(&expected_output)"));

        fs::remove_dir_all(&root).expect("remove test directories");
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
