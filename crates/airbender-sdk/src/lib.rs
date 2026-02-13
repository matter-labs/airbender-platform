#![no_std]

//! Public SDK re-exports for Airbender guest programs.

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod codec {
    pub use airbender_codec::*;
}

#[cfg(feature = "crypto")]
pub mod crypto {
    pub use airbender_crypto::*;
}

pub mod guest {
    pub use airbender_guest::*;
}

pub mod rt {
    pub use airbender_rt::*;
}

pub use airbender_macros::main;
