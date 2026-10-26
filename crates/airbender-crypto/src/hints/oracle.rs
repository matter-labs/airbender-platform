// Ported from zksync-os e855bca2:zk_ee/src/oracle/memory_io/oracle.rs
//! The oracle as seen by the querier.

/// An error of the hint protocol: the oracle failed, or its response is malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HintError(pub &'static str);

/// Querier side of the memory-based oracle protocol, which hints ("advice") are served over.
///
/// A query is the query ID and an input word, typically the address of a request struct; the
/// response is a sequence of raw `u32` words, with no framing added by the protocol (a hint may
/// start with length words of its own, as the ModExp reduction does), received through
/// [`read_word`](Self::read_word) and [`write_words`](Self::write_words).
pub trait HintOracle {
    /// Starts a query: sends the query ID and the input word. The oracle reads whatever the input word
    /// refers to during this call; the memory may change as soon as it returns.
    ///
    /// # Safety
    ///
    /// `input_word` must be the address of a live request with the layout that `query_id` defines
    /// (e.g. a `GenericFieldOpsHint<usize>` for [`FIELD_OPS_ADVISE_QUERY_ID`]), and the operand
    /// pointers in it must reference live values of the types that the op of the request expects,
    /// for the duration of the call. Implementations may dereference them: an oracle that answers
    /// in this process reads them as values of those types.
    ///
    /// [`FIELD_OPS_ADVISE_QUERY_ID`]: super::FIELD_OPS_ADVISE_QUERY_ID
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), HintError>;

    /// Reads one response word: a short value, or the claimed length of a dynamically sized response.
    fn read_word(&mut self) -> Result<u32, HintError>;

    /// Writes the next `num_words` response words to `dst`, in order. This is the memcpy-like primitive:
    /// "write the next `u32` to the next aligned address", repeated.
    ///
    /// # Safety
    ///
    /// `dst` must be non-null (even if `num_words` is 0), aligned for `u32` and valid for writes
    /// of `num_words` `u32`s.
    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), HintError>;

    /// Ends the query. Implementations that can (in forward mode) should check here that the response
    /// was consumed in full.
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

/// No oracle: the type of `None` where an `Option<&mut O>` of an oracle is expected, for the
/// computations that run without hints. It has no values, so none of its methods can be called.
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
