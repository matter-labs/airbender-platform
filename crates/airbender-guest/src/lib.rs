#![no_std]

//! Guest-side APIs for reading inputs and committing outputs.

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod commit;
pub mod input;
pub mod transport;

pub use commit::{commit, commit_extended, exit_error, Commit, CommitExtended};
pub use input::{read, read_with, GuestError};
pub use transport::{CsrTransport, MockTransport, Transport};
