//! The oracle as seen by the querier: `airbender_oracle::MemoryOracle`, with the error type of
//! this crate's own oracles.

use airbender_oracle::{MemoryOracle, OracleError};

/// An error of the hint protocol: the oracle failed, or its response is malformed. The error type
/// of this crate's oracles (the recorder, the oracle side of the hints and the test oracles); a
/// querier with an oracle of its own gets the protocol's errors in its oracle's error type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HintError(pub &'static str);

impl OracleError for HintError {
    #[track_caller]
    fn new(message: &'static str) -> Self {
        Self(message)
    }
}

/// No oracle: the type of `None` where an `Option<&mut O>` is expected, for computations without
/// hints. It has no values, so none of its methods can be called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoHints {}

impl MemoryOracle for NoHints {
    type Error = HintError;

    unsafe fn send_query(&mut self, _query_id: u32, _input_word: usize) -> Result<(), HintError> {
        match *self {}
    }

    fn read_word(&mut self) -> Result<u32, HintError> {
        match *self {}
    }

    unsafe fn write_words(&mut self, _dst: *mut u32, _num_words: usize) -> Result<(), HintError> {
        match *self {}
    }

    fn finish_query(&mut self) -> Result<(), HintError> {
        match *self {}
    }
}
