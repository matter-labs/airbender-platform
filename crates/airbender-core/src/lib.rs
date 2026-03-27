#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "host"), no_std)]

extern crate alloc;

pub mod guest;
pub mod wire;

#[cfg(feature = "host")]
pub mod manifest;

#[cfg(feature = "host")]
pub mod host {
    pub use crate::manifest;
}
