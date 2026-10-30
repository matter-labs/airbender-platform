//! The errors of the protocol, in the querier's own error type.

/// An error of the protocol: the oracle failed, or its response is malformed. The querier picks
/// the type ([`MemoryOracle::Error`](crate::MemoryOracle::Error)), so that the protocol's errors
/// are its own; the protocol makes them with [`new`](Self::new).
pub trait OracleError: Sized + core::fmt::Debug {
    /// The error with `message`. The caller's location is available through
    /// [`core::panic::Location::caller`].
    #[track_caller]
    fn new(message: &'static str) -> Self;
}
