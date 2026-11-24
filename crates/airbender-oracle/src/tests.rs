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

/// The packed transport of the 64-bit guests
#[cfg(target_pointer_width = "64")]
mod packed {
    use super::*;

    /// A channel of 64-bit words: the tape of a packed guest
    struct Tape {
        words: VecDeque<u64>,
        written: Vec<usize>,
    }

    impl WordChannel for Tape {
        fn write_word(&mut self, word: usize) {
            self.written.push(word);
        }

        fn read_word(&mut self) -> usize {
            self.words
                .pop_front()
                .expect("read past the end of the tape") as usize
        }
    }

    fn packed(words: impl IntoIterator<Item = u64>) -> PackedWordChannelOracle<Tape, TestError> {
        PackedWordChannelOracle::new(Tape {
            words: words.into_iter().collect(),
            written: Vec::new(),
        })
    }

    /// One read of the querier
    #[derive(Clone, Copy, Debug)]
    enum Read {
        Word,
        U64,
        /// `write_words` of this many words, to a destination this many words into a buffer
        /// aligned for `u64`
        Words(usize, usize),
    }

    /// The channel words of a response that the querier reads with `reads`: the packing rule,
    /// written out
    fn pack(response: &[u32], reads: &[Read]) -> Vec<u64> {
        let mut response = response.iter().copied().map(u64::from);
        let mut next = || response.next().unwrap();
        let mut tape = Vec::new();
        for read in reads {
            match *read {
                Read::Word => tape.push(next()),
                Read::U64 => tape.push(next() | next() << 32),
                Read::Words(len, _) => {
                    for _ in 0..len / 2 {
                        tape.push(next() | next() << 32);
                    }
                    if len % 2 == 1 {
                        tape.push(next());
                    }
                }
            }
        }
        assert!(response.next().is_none());
        tape
    }

    /// The words that `oracle` delivers to `reads`, as the response words they are
    fn run<O: MemoryOracle<Error = TestError>>(oracle: &mut O, reads: &[Read]) -> Vec<u32> {
        let mut words = Vec::new();
        for read in reads {
            match *read {
                Read::Word => words.push(oracle.read_word().unwrap()),
                Read::U64 => {
                    let value = oracle.read_u64().unwrap();
                    words.extend([value as u32, (value >> 32) as u32]);
                }
                Read::Words(len, offset) => {
                    let mut buffer = vec![u64::MAX; (len + offset).div_ceil(2) + 1];
                    let base = buffer.as_mut_ptr().cast::<u32>();
                    // SAFETY: `offset + len` words are within the buffer
                    unsafe {
                        oracle.write_words(base.add(offset), len).unwrap();
                        words.extend_from_slice(core::slice::from_raw_parts(base.add(offset), len));
                        // nothing around the destination is written
                        let all = core::slice::from_raw_parts(base, 2 * buffer.len());
                        assert!(all[..offset].iter().all(|word| *word == u32::MAX));
                        assert!(all[offset + len..].iter().all(|word| *word == u32::MAX));
                    }
                }
            }
        }
        words
    }

    fn response(len: usize) -> Vec<u32> {
        (0..len as u32)
            .map(|i| 0x9e37_79b9u32.wrapping_mul(i + 1) | 1 << 31)
            .collect()
    }

    fn response_len(reads: &[Read]) -> usize {
        reads
            .iter()
            .map(|read| match *read {
                Read::Word => 1,
                Read::U64 => 2,
                Read::Words(len, _) => len,
            })
            .sum()
    }

    #[test]
    fn a_query_is_the_same_two_words() {
        let mut oracle = packed([]);
        let input = 7u64;
        let address = core::ptr::from_ref(&input).expose_provenance();
        // SAFETY: the mock reads nothing through the input word
        unsafe { oracle.send_query(TEST_QUERY_ID, address) }.unwrap();
        assert_eq!(oracle.channel().written, [TEST_QUERY_ID as usize, address]);
        assert!(oracle.finish_query().is_ok());
    }

    #[test]
    fn reads_take_their_words_from_one_channel_word() {
        // the unused half of a channel word is ignored
        let mut oracle = packed([0xffff_ffff_0000_0005, 0x1111_2222_3333_4444]);
        assert_eq!(oracle.read_word().unwrap(), 5);
        assert_eq!(oracle.read_u64().unwrap(), 0x1111_2222_3333_4444);
        assert!(oracle.channel().words.is_empty());

        let mut oracle = packed([0x0000_0002_0000_0001, 0xdead_beef_0000_0003]);
        let mut dst = [u32::MAX; 4];
        // SAFETY: the destination holds 4 words
        unsafe { oracle.write_words(dst.as_mut_ptr(), 3) }.unwrap();
        assert_eq!(dst, [1, 2, 3, u32::MAX]);
        assert!(oracle.channel().words.is_empty());
    }

    /// Every sequence of reads delivers the response words that the unpacked transport does,
    /// from the channel words of the packing rule, all of them
    #[test]
    fn the_packed_transport_delivers_the_response_of_the_unpacked_one() {
        let mut sequences: Vec<Vec<Read>> = Vec::new();
        for len in 0..10 {
            for offset in 0..2 {
                sequences.push(vec![Read::Words(len, offset)]);
                sequences.push(vec![Read::Word, Read::Words(len, offset), Read::U64]);
            }
        }
        sequences.push(vec![
            Read::Word,
            Read::Word,
            Read::U64,
            Read::U64,
            Read::Word,
        ]);
        sequences.push(vec![
            Read::Words(3, 1),
            Read::Words(5, 0),
            Read::U64,
            Read::Words(8, 0),
            Read::Words(16, 1),
            Read::Word,
        ]);
        for reads in &sequences {
            let response = response(response_len(reads));

            let words = response.clone();
            let mut unpacked = mock_oracle(move |_, _| words.clone());
            // SAFETY: a query without input
            unsafe { unpacked.send_query(TEST_QUERY_ID, 0) }.unwrap();
            assert_eq!(run(&mut unpacked, reads), response, "{reads:?}");
            assert!(unpacked.channel().response.is_empty());

            let tape = pack(&response, reads);
            let mut oracle = packed(tape);
            assert_eq!(run(&mut oracle, reads), response, "{reads:?}");
            assert!(oracle.channel().words.is_empty(), "{reads:?}");

            // through a reference, which forwards every read
            let mut oracle = packed(pack(&response, reads));
            assert_eq!(run(&mut &mut oracle, reads), response, "{reads:?}");
            assert!(oracle.channel().words.is_empty(), "{reads:?}");
        }
    }

    /// The default `read_u64` is two `read_word`s, the low half first
    #[test]
    fn read_u64_defaults_to_two_words() {
        let mut oracle = mock_oracle(|_, _| vec![0x1111_1111, 0x2222_2222, 3]);
        // SAFETY: a query without input
        unsafe { oracle.send_query(TEST_QUERY_ID, 0) }.unwrap();
        assert_eq!(oracle.read_u64().unwrap(), 0x2222_2222_1111_1111);
        assert_eq!(oracle.read_word().unwrap(), 3);
        assert!(oracle.channel().response.is_empty());
    }
}
