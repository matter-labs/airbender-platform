//! Build and package guest artifacts into a `dist/` directory.

mod build;
mod config;
mod constants;
mod errors;
mod metadata;
mod resolver;
mod utils;

pub use airbender_core::host::manifest::{
    ArtifactEntry, BuildMetadata, Manifest, Profile, CODEC_VERSION_V0, MANIFEST_VERSION_V1,
};
pub use build::clean_reproducible_volumes;
pub use config::{build_dist, BuildConfig};
pub use constants::{DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
pub use errors::{BuildError, Result};
