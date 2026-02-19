//! Build and package guest artifacts into a `dist/` directory.

mod config;
mod constants;
mod errors;
mod utils;

pub use airbender_core::host::manifest::{
    ArtifactEntry, BuildMetadata, Manifest, Profile, CODEC_VERSION_V0, MANIFEST_VERSION_V1,
};
pub use config::{build_dist, BuildConfig, DistArtifacts};
pub use constants::{DEFAULT_GUEST_TARGET, DEFAULT_GUEST_TOOLCHAIN};
pub use errors::{BuildError, Result};
