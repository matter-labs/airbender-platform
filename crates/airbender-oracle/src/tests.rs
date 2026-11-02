//! The transport through [`WordChannelOracle`], against an in-process mock of the oracle side
//! that reads the inputs through the addresses exposed by the querier, the way the host reads the
//! guest memory.

extern crate std;

use super::query_ids::GENERIC_SUBSPACE_MASK;
use super::*;
use core::panic::Location;
use std::boxed::Box;
use std::collections::VecDeque;
use std::vec;
use std::vec::Vec;

const TEST_QUERY_ID: u32 = GENERIC_SUBSPACE_MASK | 0x42;

/// The error of the test querier: the message, and where the protocol made it
#[derive(Debug)]
struct TestError {
    message: &'static str,
    location: &'static Location<'static>,
}

impl OracleError for TestError {
    #[track_caller]
    fn new(message: &'static str) -> Self {
        Self {
            message,
            location: Location::caller(),
        }
    }
}

/// Oracle side of the protocol, in process: receives the two words of a query, hands them to a
/// handler that reads the input through the exposed addresses, and serves the response one word
/// at a time.
struct MockHost {
    handler: Box<dyn FnMut(u32, usize) -> Vec<u32>>,
    query_id: Option<u32>,
    response: VecDeque<u32>,
    /// `(query ID, input word)` of every query received
    queries: Vec<(u32, usize)>,
}

impl WordChannel for MockHost {
    fn write_word(&mut self, word: usize) {
        match self.query_id.take() {
            None => {
                assert!(
                    self.response.is_empty(),
                    "previous response not consumed in full"
                );
                self.query_id = Some(u32::try_from(word).unwrap());
            }
            Some(query_id) => {
                self.queries.push((query_id, word));
                self.response = (self.handler)(query_id, word).into();
            }
        }
    }

    fn read_word(&mut self) -> usize {
        self.response
            .pop_front()
            .expect("read past the end of the response") as usize
    }
}

fn mock_oracle(
    handler: impl FnMut(u32, usize) -> Vec<u32> + 'static,
) -> WordChannelOracle<MockHost, TestError> {
    WordChannelOracle::new(MockHost {
        handler: Box::new(handler),
        query_id: None,
        response: VecDeque::new(),
        queries: Vec::new(),
    })
}

#[test]
fn a_query_is_two_words_and_the_response_is_read_in_place() {
    let input = [1u32, 2, 3, 4];
    let mut oracle = mock_oracle(move |_, address| {
        // SAFETY: the querier passes the address of a live `[u32; 4]`, exposed while it sends
        let read = unsafe { core::ptr::with_exposed_provenance::<[u32; 4]>(address).read() };
        assert_eq!(read, input);
        vec![7, 5, 6, 7, 8]
    });
    // SAFETY: the input word is the address of `input`, live and exposed for the call
    unsafe {
        oracle.send_query(
            TEST_QUERY_ID,
            core::ptr::from_ref(&input).expose_provenance(),
        )
    }
    .unwrap();
    assert_eq!(oracle.read_word().unwrap(), 7);
    let mut dst = [0u32; 4];
    // SAFETY: the destination holds 4 words
    unsafe { oracle.write_words(dst.as_mut_ptr(), 4) }.unwrap();
    assert_eq!(dst, [5, 6, 7, 8]);
    oracle.finish_query().unwrap();
    assert_eq!(
        oracle.channel().queries,
        [(TEST_QUERY_ID, core::ptr::from_ref(&input).addr())]
    );
    assert!(oracle.channel().response.is_empty());
}

#[test]
fn writes_of_any_length_are_unrolled_in_order() {
    for len in 0..10u32 {
        let mut oracle = mock_oracle(move |_, _| (0..len).collect());
        // SAFETY: a query without input
        unsafe { oracle.send_query(TEST_QUERY_ID, 0) }.unwrap();
        let mut dst = vec![u32::MAX; len as usize + 1];
        // SAFETY: the destination holds `len + 1` words
        unsafe { oracle.write_words(dst.as_mut_ptr(), len as usize) }.unwrap();
        assert_eq!(dst[..len as usize], (0..len).collect::<Vec<_>>()[..]);
        assert_eq!(dst[len as usize], u32::MAX);
    }
}

/// A channel whose words do not fit into `u32`
struct WideChannel;

impl WordChannel for WideChannel {
    fn write_word(&mut self, _word: usize) {}

    fn read_word(&mut self) -> usize {
        usize::MAX
    }
}

#[test]
fn errors_are_the_queriers_located_where_the_protocol_makes_them() {
    struct Unserved;
    impl MemoryOracle for Unserved {
        type Error = TestError;
    }
    // SAFETY: a query without input
    let error = unsafe { Unserved.send_query(TEST_QUERY_ID, 0) }.unwrap_err();
    assert_eq!(
        error.message,
        "oracle does not serve the memory-based protocol"
    );
    assert!(error.location.file().ends_with("oracle.rs"));
    assert!(Unserved.read_word().is_err());
    assert!(Unserved.finish_query().is_ok());

    // a word that does not fit into `u32`, possible on 64-bit hosts only
    let mut wide = WordChannelOracle::<_, TestError>::new(WideChannel);
    let error = wide.read_word().unwrap_err();
    assert_eq!(error.message, "oracle word does not fit into u32");
    assert!(error.location.file().ends_with("word_channel.rs"));
}
