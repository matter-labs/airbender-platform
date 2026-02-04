//! Build and package guest artifacts into a `dist/` directory.

mod config;
mod constants;
mod errors;
mod utils;

pub use airbender_core::manifest::{Manifest, Profile, MANIFEST_FORMAT_VERSION};
pub use config::{build_dist, BuildConfig, DistArtifacts};
pub use errors::{BuildError, Result};
