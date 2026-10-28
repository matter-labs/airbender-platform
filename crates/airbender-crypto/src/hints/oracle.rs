//! The oracle as seen by the querier.

/// An error of the hint protocol: the oracle failed, or its response is malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HintError(pub &'static str);

/// Querier side of the memory-based oracle protocol that hints are served over.
///
/// A query is the query ID and an input word, typically the address of a request struct; the
/// response is raw `u32` words with no framing, received through [`read_word`](Self::read_word)
/// and [`write_words`](Self::write_words).
pub trait HintOracle {
    /// Starts a query. The oracle reads whatever the input word refers to during this call only.
    ///
    /// # Safety
    ///
    /// `input_word` must be the address of a live request with the layout that `query_id` defines
    /// (e.g. a `GenericFieldOpsHint<usize>` for [`FIELD_OPS_ADVISE_QUERY_ID`]), whose operand
    /// pointers reference live values of the types its op expects, for the duration of the call:
    /// an oracle in this process reads them as values of those types.
    ///
    /// [`FIELD_OPS_ADVISE_QUERY_ID`]: super::FIELD_OPS_ADVISE_QUERY_ID
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError>;

    /// Reads one response word.
    fn read_word(&mut self) -> Result<u32, HintError>;

    /// Writes the next `num_words` response words to `dst`, in order.
    ///
    /// # Safety
    ///
    /// `dst` must be non-null (even if `num_words` is 0), aligned for `u32` and valid for writes
    /// of `num_words` `u32`s.
    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError>;

    /// Ends the query. Oracles that can should check here that the response was consumed in full.
    fn finish_query(&mut self) -> Result<(), HintError> {
        Ok(())
    }
}

impl<O: HintOracle + ?Sized> HintOracle for &mut O {
    #[inline(always)]
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError> {
        // SAFETY: guaranteed by the caller
        unsafe { (**self).send_query(query_id, input_word) }
    }

    #[inline(always)]
    fn read_word(&mut self) -> Result<u32, HintError> {
        (**self).read_word()
    }

    #[inline(always)]
    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError> {
        // SAFETY: guaranteed by the caller
        unsafe { (**self).write_words(dst, num_words) }
    }

    #[inline(always)]
    fn finish_query(&mut self) -> Result<(), HintError> {
        (**self).finish_query()
    }
}

/// No oracle: the type of `None` where an `Option<&mut O>` is expected, for computations without
/// hints. It has no values, so none of its methods can be called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoHints {}

impl HintOracle for NoHints {
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
