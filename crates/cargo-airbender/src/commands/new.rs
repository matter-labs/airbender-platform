use crate::cli::{NewAllocatorArg, NewArgs};
use airbender_build::DEFAULT_GUEST_TOOLCHAIN;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_SDK_GIT_REPOSITORY: &str = "https://github.com/popzxc/airbender-platform";
const DEFAULT_SDK_GIT_BRANCH: &str = "main";

const TEMPLATE_FILES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../templates/README.md.template"),
    ),
    (
        "guest/Cargo.toml",
        include_str!("../../templates/guest/Cargo.toml.template"),
    ),
    (
        "guest/src/main.rs",
        include_str!("../../templates/guest/src/main.rs.template"),
    ),
    (
        "guest/rust-toolchain.toml",
        include_str!("../../templates/guest/rust-toolchain.toml.template"),
    ),
    (
        "guest/.cargo/config.toml",
        include_str!("../../templates/guest/.cargo/config.toml.template"),
    ),
    (
        "host/Cargo.toml",
        include_str!("../../templates/host/Cargo.toml.template"),
    ),
    (
        "host/src/main.rs",
        include_str!("../../templates/host/src/main.rs.template"),
    ),
    (
        "host/rust-toolchain.toml",
        include_str!("../../templates/host/rust-toolchain.toml.template"),
    ),
];

pub fn run(args: NewArgs) -> Result<()> {
    let project_name = args.name.or_else(|| {
        args.path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
    });
    let project_name = project_name.context("while attempting to determine project name")?;

    ensure_empty_dir(&args.path)?;
    fs::create_dir_all(&args.path).with_context(|| {
        format!(
            "while attempting to create destination directory {}",
            args.path.display()
        )
    })?;

    let guest_destination_dir = args.path.join("guest");
    fs::create_dir_all(&guest_destination_dir).with_context(|| {
        format!(
            "while attempting to create destination directory {}",
            guest_destination_dir.display()
        )
    })?;
    let host_destination_dir = args.path.join("host");
    fs::create_dir_all(&host_destination_dir).with_context(|| {
        format!(
            "while attempting to create destination directory {}",
            host_destination_dir.display()
        )
    })?;

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

    let guest_attributes = if args.enable_std {
        "#![no_main]"
    } else {
        "#![no_std]\n#![no_main]"
    };
    let sdk_default_features = match args.allocator {
        NewAllocatorArg::Talc => "",
        NewAllocatorArg::Bump | NewAllocatorArg::Custom => ", default-features = false",
    };

    let mut sdk_feature_flags = Vec::new();
    if args.enable_std {
        sdk_feature_flags.push("std");
    }
    match args.allocator {
        NewAllocatorArg::Talc => {}
        NewAllocatorArg::Bump => sdk_feature_flags.push("allocator-bump"),
        NewAllocatorArg::Custom => sdk_feature_flags.push("allocator-custom"),
    }
    let sdk_features = if sdk_feature_flags.is_empty() {
        String::new()
    } else {
        let rendered = sdk_feature_flags
            .into_iter()
            .map(|flag| format!("\"{flag}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(", features = [{rendered}]")
    };

    let (main_attr_args, custom_allocator_module) = match args.allocator {
        NewAllocatorArg::Custom => (
            "(allocator_init = crate::custom_allocator::init)",
            "mod custom_allocator {\n    use core::alloc::{GlobalAlloc, Layout};\n    use core::cell::UnsafeCell;\n    use core::ptr::null_mut;\n\n    struct CustomBumpAllocator {\n        state: UnsafeCell<State>,\n    }\n\n    struct State {\n        start: usize,\n        end: usize,\n        current: usize,\n        initialized: bool,\n    }\n\n    unsafe impl Sync for CustomBumpAllocator {}\n\n    impl CustomBumpAllocator {\n        const fn uninit() -> Self {\n            Self {\n                state: UnsafeCell::new(State {\n                    start: 0,\n                    end: 0,\n                    current: 0,\n                    initialized: false,\n                }),\n            }\n        }\n\n        unsafe fn init(&self, start: *mut usize, end: *mut usize) {\n            let state = &mut *self.state.get();\n            state.start = start as usize;\n            state.end = end as usize;\n            state.current = state.start;\n            state.initialized = true;\n        }\n\n        unsafe fn alloc_inner(&self, layout: Layout) -> *mut u8 {\n            let state = &mut *self.state.get();\n            if !state.initialized {\n                return null_mut();\n            }\n\n            let aligned = (state.current + layout.align() - 1) & !(layout.align() - 1);\n            let next = aligned.saturating_add(layout.size());\n            if next > state.end {\n                return null_mut();\n            }\n\n            state.current = next;\n            aligned as *mut u8\n        }\n    }\n\n    unsafe impl GlobalAlloc for CustomBumpAllocator {\n        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {\n            self.alloc_inner(layout)\n        }\n\n        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}\n    }\n\n    #[global_allocator]\n    static GLOBAL_ALLOCATOR: CustomBumpAllocator = CustomBumpAllocator::uninit();\n\n    pub fn init(start: *mut usize, end: *mut usize) {\n        unsafe { GLOBAL_ALLOCATOR.init(start, end) };\n    }\n}\n",
        ),
        NewAllocatorArg::Talc | NewAllocatorArg::Bump => ("", ""),
    };

    let replacements = [
        ("__AIRBENDER_PROJECT_NAME__", project_name.as_str()),
        ("__AIRBENDER_SDK_DEP__", sdk_dependency.as_str()),
        ("__AIRBENDER_SDK_DEFAULT_FEATURES__", sdk_default_features),
        ("__AIRBENDER_HOST_DEP__", host_dependency.as_str()),
        ("__AIRBENDER_GUEST_ATTRIBUTES__", guest_attributes),
        ("__AIRBENDER_SDK_FEATURES__", sdk_features.as_str()),
        ("__AIRBENDER_MAIN_ATTR_ARGS__", main_attr_args),
        (
            "__AIRBENDER_CUSTOM_ALLOCATOR_MODULE__",
            custom_allocator_module,
        ),
        (
            "__AIRBENDER_RUST_TOOLCHAIN_CHANNEL__",
            DEFAULT_GUEST_TOOLCHAIN,
        ),
    ];

    write_template(&args.path, &replacements)?;
    tracing::info!("created project at {}", args.path.display());
    Ok(())
}

fn resolve_crate_dependency(
    destination_crate_dir: &Path,
    sdk_path: Option<&Path>,
    sdk_version: Option<&str>,
    crate_name: &str,
) -> Result<String> {
    if let Some(version) = sdk_version {
        if version.is_empty() {
            bail!("--sdk-version cannot be empty");
        }
        return Ok(format!("version = \"{version}\""));
    }

    if let Some(sdk_path) = sdk_path {
        if !sdk_path.exists() {
            bail!("failed to locate SDK path at {}", sdk_path.display());
        }

        let crate_path = resolve_dependency_crate_path(sdk_path, crate_name)?;
        let sdk_relative = relative_path(destination_crate_dir, &crate_path)?;
        return Ok(format!("path = \"{}\"", sdk_relative.to_string_lossy()));
    }

    Ok(format!(
        "git = \"{DEFAULT_SDK_GIT_REPOSITORY}\", branch = \"{DEFAULT_SDK_GIT_BRANCH}\""
    ))
}

fn resolve_dependency_crate_path(sdk_path: &Path, crate_name: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    if sdk_path
        .file_name()
        .map(|name| name == crate_name)
        .unwrap_or(false)
    {
        candidates.push(sdk_path.to_path_buf());
    }
    candidates.push(sdk_path.join(crate_name));
    candidates.push(sdk_path.join("crates").join(crate_name));
    if let Some(parent) = sdk_path.parent() {
        candidates.push(parent.join(crate_name));
    }

    for candidate in candidates {
        if candidate.join("Cargo.toml").exists() {
            return candidate.canonicalize().with_context(|| {
                format!("while attempting to canonicalize {}", candidate.display())
            });
        }
    }

    bail!(
        "failed to locate {crate_name} crate under {}",
        sdk_path.display()
    )
}

fn ensure_empty_dir(path: &Path) -> Result<()> {
    if path.exists()
        && path
            .read_dir()
            .with_context(|| format!("while attempting to list {}", path.display()))?
            .next()
            .is_some()
    {
        bail!("destination directory is not empty: {}", path.display());
    }
    Ok(())
}

fn write_template(destination_root: &Path, replacements: &[(&str, &str)]) -> Result<()> {
    for (relative_path, source) in TEMPLATE_FILES {
        let destination_path = destination_root.join(relative_path);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("while attempting to create {}", parent.display()))?;
        }

        let mut content = source.to_string();
        for (from, to) in replacements {
            content = content.replace(from, to);
        }
        fs::write(&destination_path, content)
            .with_context(|| format!("while attempting to write {}", destination_path.display()))?;
    }
    Ok(())
}

fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let from = from
        .canonicalize()
        .with_context(|| format!("while attempting to canonicalize {}", from.display()))?;
    let to = to
        .canonicalize()
        .with_context(|| format!("while attempting to canonicalize {}", to.display()))?;

    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let mut common_len = 0usize;
    while common_len < from_components.len()
        && common_len < to_components.len()
        && from_components[common_len] == to_components[common_len]
    {
        common_len += 1;
    }

    let mut result = PathBuf::new();
    for _ in common_len..from_components.len() {
        result.push("..");
    }
    for component in &to_components[common_len..] {
        result.push(component.as_os_str());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn defaults_to_sdk_git_repository() {
        let dependency = resolve_crate_dependency(Path::new("."), None, None, "airbender-sdk")
            .expect("resolve default SDK dependency");
        assert_eq!(
            dependency,
            "git = \"https://github.com/popzxc/airbender-platform\", branch = \"main\""
        );
    }

    #[test]
    fn prefers_explicit_sdk_version() {
        let dependency =
            resolve_crate_dependency(Path::new("."), None, Some("0.1.0"), "airbender-sdk")
                .expect("resolve version SDK dependency");
        assert_eq!(dependency, "version = \"0.1.0\"");
    }

    #[test]
    fn rejects_empty_sdk_version() {
        let err = resolve_crate_dependency(Path::new("."), None, Some(""), "airbender-sdk")
            .expect_err("empty version should fail");
        assert!(err.to_string().contains("--sdk-version cannot be empty"));
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

        let dependency = resolve_crate_dependency(
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

        let dependency =
            resolve_crate_dependency(&destination, Some(sdk.as_path()), None, "airbender-host")
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
            path: destination.clone(),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Talc,
            sdk_path: None,
            sdk_version: Some("0.1.0".to_string()),
        })
        .expect("create scaffold");

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
        assert!(host_main.contains("Program::load"));
        assert!(host_toolchain.contains(&format!("channel = \"{}\"", DEFAULT_GUEST_TOOLCHAIN)));
        assert!(!host_toolchain.contains("components"));

        fs::remove_dir_all(&root).expect("remove test directories");
    }

    #[test]
    fn new_enable_std_updates_guest_template() {
        let root = test_workspace_dir("scaffold-enable-std");
        let destination = root.join("hello-airbender");

        run(NewArgs {
            path: destination.clone(),
            name: Some("hello-airbender".to_string()),
            enable_std: true,
            allocator: NewAllocatorArg::Talc,
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
            path: destination.clone(),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Bump,
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
            path: destination.clone(),
            name: Some("hello-airbender".to_string()),
            enable_std: false,
            allocator: NewAllocatorArg::Custom,
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
