#![no_std]

//! Guest-side APIs for reading inputs and committing outputs.

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod commit;
pub mod cycle;
pub mod input;
pub mod transport;

pub use commit::{commit, exit_error, Commit};
pub use cycle::{marker as cycle_marker, record_cycles};
pub use input::{read, read_with, GuestError};
pub use transport::{CsrTransport, MockTransport, Transport};
