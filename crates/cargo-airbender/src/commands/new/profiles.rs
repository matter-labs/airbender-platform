use crate::cli::NewProverBackendArg;

const HOST_MAIN_DEV_TEMPLATE: &str =
    include_str!("../../../templates/host/src/main.dev.rs.template");
const HOST_MAIN_GPU_TEMPLATE: &str =
    include_str!("../../../templates/host/src/main.gpu.rs.template");
const README_BACKEND_DEV_DOC: &str =
    include_str!("../../../templates/snippets/prover_backend.dev.md.template");
const README_BACKEND_GPU_DOC: &str =
    include_str!("../../../templates/snippets/prover_backend.gpu.md.template");

#[derive(Clone, Copy)]
pub(super) struct ProverBackendProfile {
    pub(super) host_dependency_features: &'static str,
    pub(super) host_main_template: &'static str,
    pub(super) readme_prover_backend_doc: &'static str,
    pub(super) host_run_command: &'static str,
}

pub(super) fn prover_backend_profile(backend: NewProverBackendArg) -> ProverBackendProfile {
    match backend {
        NewProverBackendArg::Dev => ProverBackendProfile {
            host_dependency_features: ", default-features = false",
            host_main_template: HOST_MAIN_DEV_TEMPLATE,
            readme_prover_backend_doc: README_BACKEND_DEV_DOC,
            host_run_command: "cd ../host && cargo run --release",
        },
        NewProverBackendArg::Gpu => ProverBackendProfile {
            host_dependency_features: "",
            host_main_template: HOST_MAIN_GPU_TEMPLATE,
            readme_prover_backend_doc: README_BACKEND_GPU_DOC,
            host_run_command: "cd ../host && ZKSYNC_USE_CUDA_STUBS=true cargo run --release",
        },
    }
}
