//! The oracle of Airbender guests: the query IDs, and the querier side of the memory-based
//! protocol.
//!
//! On the proving target the oracle is the non-determinism CSR, a channel of words. In the
//! memory-based protocol a query is exactly two words, the query ID (see [`query_ids`]) and one
//! *input word*: `0` for a query without input, a short value itself, or the address of the input
//! (e.g. a request struct) in the memory of the querier, which the oracle reads while it receives
//! the word. The response is exactly the words the querier reads, without a length prefix, each
//! written by the querier straight to its destination ([`MemoryOracle::write_words`]).
//!
//! [`MemoryOracle`] is the querier's side of that transport, with the querier's error type;
//! [`WordChannelOracle`] implements it over a word channel, and on the guest [`CsrOracle`] over the
//! non-determinism CSR. The encodings of values on top of it are the querier's (ZKsync OS's
//! `zk_ee::oracle::memory_io`, the hints of `airbender-crypto`).
//!
//! # Security
//!
//! Everything the oracle sends is untrusted: the querier validates the encoding of a response,
//! and checks that the value is correct, before it uses it.

#![no_std]

#[cfg(target_arch = "riscv32")]
mod csr;
mod error;
mod oracle;
pub mod query_ids;
mod word_channel;

#[cfg(test)]
mod tests;

#[cfg(target_arch = "riscv32")]
pub use self::csr::{CsrChannel, CsrOracle};
pub use self::error::OracleError;
pub use self::oracle::MemoryOracle;
pub use self::word_channel::{WordChannel, WordChannelOracle};
