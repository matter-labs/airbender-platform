//! The oracle as seen by the querier.

use super::error::OracleError;

/// Querier side of the memory-based oracle protocol (see the [crate documentation](crate)): the
/// transport primitives. A query is the query ID and an input word, typically the address of a
/// request; the response is raw `u32` words with no framing, received through
/// [`read_word`](Self::read_word) and [`write_words`](Self::write_words). Typed encodings are
/// built on top of these by the querier (e.g. the hints of `airbender-crypto`).
///
/// The default primitives reject the protocol: they are for oracles that never serve it, like
/// test doubles.
pub trait MemoryOracle: Sized {
    /// The errors of the protocol, in the querier's error type
    type Error: OracleError;

    /// Starts a query: sends the query ID and the input word. The oracle reads whatever the input
    /// word refers to during this call; the memory may change as soon as it returns.
    ///
    /// # Safety
    ///
    /// The input word must be what `query_id` defines: a value, or the address of a live input
    /// with the layout that `query_id` defines, whose own pointers (e.g. the operand of a request)
    /// reference live values of the types the query expects, for the duration of the call. An
    /// oracle in this process reads them through the address, as values of those types.
    unsafe fn send_query(&mut self, _query_id: u32, _input_word: usize) -> Result<(), Self::Error> {
        Err(Self::Error::new(
            "oracle does not serve the memory-based protocol",
        ))
    }

    /// Reads one response word.
    fn read_word(&mut self) -> Result<u32, Self::Error> {
        Err(Self::Error::new(
            "oracle does not serve the memory-based protocol",
        ))
    }

    /// Writes the next `num_words` response words to `dst`, in order. This is the memcpy-like
    /// primitive: "write the next `u32` to the next aligned address", repeated.
    ///
    /// # Safety
    ///
    /// `dst` must be non-null (even if `num_words` is 0), aligned for `u32` and valid for writes
    /// of `num_words` `u32`s.
    unsafe fn write_words(&mut self, _dst: *mut u32, _num_words: usize) -> Result<(), Self::Error> {
        Err(Self::Error::new(
            "oracle does not serve the memory-based protocol",
        ))
    }

    /// Ends the query. Implementations that can (in forward mode) should check here that the
    /// response was consumed in full.
    fn finish_query(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<O: MemoryOracle> MemoryOracle for &mut O {
    type Error = O::Error;

    #[inline(always)]
    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), Self::Error> {
        // SAFETY: guaranteed by the caller
        unsafe { (**self).send_query(query_id, input_word) }
    }

    #[inline(always)]
    fn read_word(&mut self) -> Result<u32, Self::Error> {
        (**self).read_word()
    }

    #[inline(always)]
    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), Self::Error> {
        // SAFETY: guaranteed by the caller
        unsafe { (**self).write_words(dst, num_words) }
    }

    #[inline(always)]
    fn finish_query(&mut self) -> Result<(), Self::Error> {
        (**self).finish_query()
    }
}
