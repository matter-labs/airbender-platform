//! The oracle side of the protocol on the host (the `oracle_processor` feature, `std`): an oracle
//! that dispatches queries to [`OracleQueryProcessor`]s, serves a querier in this process through
//! [`MemoryOracle`], and serves the RISC-V guest of the simulator as its non-determinism CSR
//! source.
//!
//! Two protocols are dispatched: the memory-based one of this crate (a query is the ID and an
//! input word, the response is raw words), and the iterator-based one of ZKsync OS (the ID, a
//! length-prefixed input of `usize` words, each sent as two `u32`s, and a length-prefixed
//! response of `usize` words read the same way), which [`OracleQueryProcessor::process_buffered_query`]
//! serves.

use crate::querier_memory::{NativeQuerierMemory, QuerierMemory, QuerierMemoryError};
use crate::query_ids::{DISCONNECT_ORACLE_QUERY_ID, UART_QUERY_ID};
use crate::{MemoryOracle, OracleError};
use core::marker::PhantomData;
pub use riscv_transpiler::vm::{NonDeterminismCSRSource, RamPeek};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

/// A CSR source that never peeks into memory: for the queries of a querier in this process.
pub struct DummyMemorySource;

impl RamPeek for DummyMemorySource {
    fn peek_word(&self, _address: u32) -> u32 {
        unreachable!("DummyMemorySource should not be read from")
    }
}

/// The memory of the RISC-V guest, for the processors of memory-based queries (see
/// [`OracleQueryProcessor::process_memory_query`]).
pub struct GuestMemory<'a, R: RamPeek + ?Sized>(pub &'a R);

impl<R: RamPeek + ?Sized> QuerierMemory for GuestMemory<'_, R> {
    fn word_size(&self) -> usize {
        size_of::<u32>()
    }

    fn read_u32(&self, address: usize) -> Result<u32, QuerierMemoryError> {
        let address = u32::try_from(address)
            .map_err(|_| QuerierMemoryError("guest address does not fit into u32"))?;
        if !address.is_multiple_of(4) {
            return Err(QuerierMemoryError("unaligned guest address"));
        }
        Ok(self.0.peek_word(address))
    }
}

/// Sized adapter so unsized peekers (`[u32]`, generic `R: ?Sized`) can be passed to the
/// `&dyn RamPeek` based query processors.
struct PeekRef<'a, R: RamPeek + ?Sized>(&'a R);

impl<R: RamPeek + ?Sized> RamPeek for PeekRef<'_, R> {
    #[inline(always)]
    fn peek_word(&self, address: u32) -> u32 {
        self.0.peek_word(address)
    }
}

/// The run an oracle serves: where the queries come from, and which responses its processors
/// produce (see [`OracleQueryProcessor::process_memory_query`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunMode {
    /// A native run: the queries come from a querier in this process, and are answered for it
    /// only.
    #[default]
    NativeRunOnly,
    /// A native run that records the prover input: the queries come from a querier in this
    /// process, and are also answered for the RISC-V guest, whose responses are saved for its
    /// replay (see [`ReadWitnessSource`]).
    NativeRunSavingForRiscV,
    /// A run of the RISC-V guest in the simulator: the queries, and the memory pointers in them,
    /// come from the guest, and are answered for it only.
    RiscVRun,
}

impl RunMode {
    /// Whether the queries come from a querier in this process, which the responses of the
    /// native run are produced for
    pub const fn produces_native_run_responses(self) -> bool {
        match self {
            Self::NativeRunOnly | Self::NativeRunSavingForRiscV => true,
            Self::RiscVRun => false,
        }
    }

    /// Whether the responses of the RISC-V guest run are produced: to serve the guest, or to
    /// save them for its replay
    pub const fn produces_guest_run_responses(self) -> bool {
        match self {
            Self::NativeRunOnly => false,
            Self::NativeRunSavingForRiscV | Self::RiscVRun => true,
        }
    }
}

/// The responses of a processor to a query: the words as a querier in this process reads them,
/// and the words as the RISC-V guest reads them (see [`OracleQueryProcessor::process_memory_query`]).
pub type BoxedResponseIterator = Box<dyn ExactSizeIterator<Item = usize> + 'static + Send + Sync>;

/// A responder to oracle queries, by query ID.
pub trait OracleQueryProcessor {
    /// IDs of the queries served with the iterator-based protocol (for example the block
    /// metadata of ZKsync OS).
    fn supported_query_ids(&self) -> Vec<u32> {
        Vec::new()
    }

    fn supports_query_id(&self, query_id: u32) -> bool {
        self.supported_query_ids().contains(&query_id)
    }

    /// Serves a query of the iterator-based protocol: the `usize` words of the input, and the
    /// memory of the guest for a processor that reads it.
    fn process_buffered_query(
        &mut self,
        query_id: u32,
        _query: Vec<usize>,
        _memory: &dyn RamPeek,
    ) -> BoxedResponseIterator {
        panic!("query ID 0x{query_id:08x} is not served with the iterator-based protocol")
    }

    /// IDs of the queries served with the memory-based protocol (see the [crate
    /// documentation](crate)).
    fn supported_memory_query_ids(&self) -> Vec<u32> {
        Vec::new()
    }

    /// Serves a query of the memory-based protocol. `input_word` is the input word the querier
    /// sent, and `memory` reads the memory of the querier it may refer to. `mode` tells the
    /// querier apart: a querier in this process (a native run), or the RISC-V guest in the
    /// simulator, which the memory pointers of the query then belong to.
    ///
    /// Appends the response words, exactly as the querier reads them, for the runs of `mode`: to
    /// `native_run_responses` as a querier in this process reads them, to `guest_run_responses`
    /// as the RISC-V guest reads them. The oracle serves the responses of its querier, and a
    /// native run that records the prover input saves the responses of the guest run, which the
    /// guest reads in place of the native querier on replay (see [`ReadWitnessSource`]); a run on
    /// the RISC-V guest has no responses of a native run. A response that is the same on every
    /// target goes through [`respond_to_every_target`]; the field hints, for example, follow the
    /// representation of field elements on each target.
    fn process_memory_query(
        &mut self,
        query_id: u32,
        _input_word: usize,
        _memory: &dyn QuerierMemory,
        _mode: RunMode,
        _native_run_responses: &mut Vec<u32>,
        _guest_run_responses: &mut Vec<u32>,
    ) {
        panic!("query ID 0x{query_id:08x} is not served with the memory-based protocol")
    }
}

/// Appends a response that is the same on every target to the responses of the runs of `mode`
/// (see [`OracleQueryProcessor::process_memory_query`]).
pub fn respond_to_every_target(
    mode: RunMode,
    mut response: Vec<u32>,
    native_run_responses: &mut Vec<u32>,
    guest_run_responses: &mut Vec<u32>,
) {
    match mode {
        RunMode::NativeRunOnly => native_run_responses.append(&mut response),
        RunMode::NativeRunSavingForRiscV => {
            guest_run_responses.extend_from_slice(&response);
            native_run_responses.append(&mut response);
        }
        RunMode::RiscVRun => guest_run_responses.append(&mut response),
    }
}

/// An oracle that buffers incoming queries till they are complete, then dispatches them to its
/// processors. Adding a processor checks that no two serve the same query ID. `E` is the error
/// type of its querier in this process (see [`MemoryOracle`]).
pub struct ZkEENonDeterminismSource<E: OracleError> {
    /// The run the oracle serves, which the processors of memory-based queries respond for.
    mode: RunMode,
    query_buffer: Option<QueryBuffer>,
    current_query_id: Option<u32>,
    current_iterator: Option<Box<dyn ExactSizeIterator<Item = usize> + 'static>>,
    iterator_len_to_indicate: Option<u32>,
    high_half: Option<u32>,
    is_connected_to_external_oracle: bool,
    /// The processors that serve the queries.
    processors: Vec<Box<dyn OracleQueryProcessor + 'static>>,
    /// Query ID to the index of its processor in `processors`, for the iterator-based protocol.
    ranges: BTreeMap<u32, usize>,
    /// The same for the queries served with the memory-based protocol.
    memory_query_ranges: BTreeMap<u32, usize>,
    /// Words of the response to the current memory-based query that the querier did not read
    /// yet.
    memory_response: VecDeque<u32>,
    /// The response of the RISC-V guest run to the current memory-based query of a querier in
    /// this process: the words the guest reads in place of the querier, for a native run that
    /// records them (see [`ReadWitnessSource`]).
    guest_run_response: Vec<u32>,
    /// The memory-based query whose input word the guest sends next, when running as the CSR
    /// source.
    memory_query_awaiting_input: Option<u32>,
    _error: PhantomData<fn() -> E>,
}

impl<E: OracleError> Default for ZkEENonDeterminismSource<E> {
    fn default() -> Self {
        Self {
            mode: RunMode::default(),
            query_buffer: None,
            current_query_id: None,
            current_iterator: None,
            iterator_len_to_indicate: None,
            high_half: None,
            is_connected_to_external_oracle: false,
            processors: Vec::new(),
            ranges: BTreeMap::new(),
            memory_query_ranges: BTreeMap::new(),
            memory_response: VecDeque::new(),
            guest_run_response: Vec::new(),
            memory_query_awaiting_input: None,
            _error: PhantomData,
        }
    }
}

impl<E: OracleError> ZkEENonDeterminismSource<E> {
    /// An oracle without processors for the run `mode` (the default is a native run only).
    pub fn new(mode: RunMode) -> Self {
        let mut source = Self::default();
        source.mode = mode;
        source
    }

    /// The run the oracle serves.
    pub fn run_mode(&self) -> RunMode {
        self.mode
    }

    /// Sets the run the oracle serves, before it serves queries.
    pub fn set_run_mode(&mut self, mode: RunMode) {
        self.mode = mode;
    }

    /// Whether the querier is still connected (no [`DISCONNECT_ORACLE_QUERY_ID`] query yet).
    pub fn is_connected(&self) -> bool {
        self.is_connected_to_external_oracle
    }

    #[track_caller]
    pub fn add_external_processor<P: OracleQueryProcessor + 'static>(&mut self, processor: P) {
        let processor_id = self.processors.len();
        for id in processor.supported_query_ids() {
            let existing = self.ranges.insert(id, processor_id);
            assert!(
                existing.is_none() && !self.memory_query_ranges.contains_key(&id),
                "more than one processor for query id 0x{id:08x}"
            );
        }
        for id in processor.supported_memory_query_ids() {
            let existing = self.memory_query_ranges.insert(id, processor_id);
            assert!(
                existing.is_none() && !self.ranges.contains_key(&id),
                "more than one processor for query id 0x{id:08x}"
            );
        }
        self.processors.push(Box::new(processor));
        self.is_connected_to_external_oracle = true;
    }

    /// Serves an iterator-based query of a querier in this process: the `usize` words of the
    /// input, and the words of the response (an empty response once disconnected).
    pub fn raw_query(
        &mut self,
        query_type: u32,
        input: Vec<usize>,
    ) -> Result<Box<dyn ExactSizeIterator<Item = usize> + 'static>, E> {
        if !self.is_connected_to_external_oracle {
            return Ok(Box::new([].into_iter()));
        }
        let Some(processor) = self.ranges.get(&query_type).copied() else {
            return Err(E::new("invalid query ID"));
        };
        let processor = &mut self.processors[processor];
        let started = query_timing::start();
        let response = processor.process_buffered_query(query_type, input, &DummyMemorySource);
        query_timing::record(query_type, None, started);
        Ok(response)
    }

    /// Serves a memory-based query of the querier of the run, and keeps the response for it to
    /// read, and in a native run that records the prover input the response of the RISC-V guest
    /// run.
    fn process_memory_query(
        &mut self,
        query_id: u32,
        input_word: usize,
        memory: &dyn QuerierMemory,
    ) -> Result<(), E> {
        let Some(processor_id) = self.memory_query_ranges.get(&query_id).copied() else {
            return Err(E::new("invalid query ID"));
        };
        let mode = self.mode;
        let mut native_run_responses = Vec::new();
        let mut guest_run_responses = Vec::new();
        let started = query_timing::start();
        self.processors[processor_id].process_memory_query(
            query_id,
            input_word,
            memory,
            mode,
            &mut native_run_responses,
            &mut guest_run_responses,
        );
        query_timing::record(query_id, None, started);
        // the responses of the runs of the mode, and only those
        let responds_for_mode = match mode {
            RunMode::NativeRunOnly => guest_run_responses.is_empty(),
            RunMode::NativeRunSavingForRiscV => {
                native_run_responses.is_empty() == guest_run_responses.is_empty()
            }
            RunMode::RiscVRun => native_run_responses.is_empty(),
        };
        if !responds_for_mode {
            return Err(E::new(
                "the query processor does not respond for the run mode",
            ));
        }
        match mode {
            RunMode::NativeRunOnly => self.memory_response = native_run_responses.into(),
            RunMode::NativeRunSavingForRiscV => {
                self.memory_response = native_run_responses.into();
                self.guest_run_response = guest_run_responses;
            }
            RunMode::RiscVRun => self.memory_response = guest_run_responses.into(),
        }
        Ok(())
    }

    fn process_buffered_query(&mut self, memory: &dyn RamPeek) {
        assert!(self.current_iterator.is_none());
        assert!(self.current_query_id.is_none());

        let buffer = self.query_buffer.take().expect("must exist");
        let query_id = buffer.query_type;
        let buffer = buffer.buffer;
        let Some(processor_id) = self.ranges.get(&query_id).copied() else {
            panic!("Can not process query with ID = 0x{query_id:08x}");
        };
        let processor = &mut self.processors[processor_id];
        let started = query_timing::start();
        let new_iterator = processor.process_buffered_query(query_id, buffer, memory);
        query_timing::record(query_id, None, started);

        let result_len = new_iterator.len() * 2; // NOTE for mismatch of 32/64-bit archs
        self.iterator_len_to_indicate = Some(result_len as u32);
        if result_len > 0 {
            self.current_query_id = Some(query_id);
            self.current_iterator = Some(new_iterator);
        }
    }

    /// The next word the guest reads from the CSR. The iterators hold `usize` (64-bit) items,
    /// which the guest reads as two words, low half first.
    pub fn read_csr_word(&mut self) -> u32 {
        // We mocked reads, so it's filtered out before
        if !self.is_connected_to_external_oracle {
            return 0;
        }

        if let Some(word) = self.memory_response.pop_front() {
            return word;
        }

        if let Some(iterator_len_to_indicate) = self.iterator_len_to_indicate.take() {
            return iterator_len_to_indicate;
        }

        // This is the 32 bits remaining from the previous item - return them now.
        if let Some(high) = self.high_half.take() {
            return high;
        }
        // If we didn't have any partial data left, we should fetch another element from the iterator.
        let Some(current_iterator) = self.current_iterator.as_mut() else {
            panic!("trying to read, but data is not prepared");
        };
        let next = current_iterator.next().expect("must contain next element");
        if current_iterator.len() == 0 {
            // we are done - there are no more elements left after this one.
            self.current_query_id = None;
            self.current_iterator = None;
        }
        // Split the 64 bits into 2 pieces - one is put into 'high' field, to be returned later
        // and the other one is returned immediately.
        let high = (next >> 32) as u32;
        let low = next as u32;
        self.high_half = Some(high);

        low
    }

    /// A word the guest writes to the CSR: a query ID, the length or a word of an iterator-based
    /// input, or the input word of a memory-based query, which is served at once with `memory`.
    pub fn write_csr_word(&mut self, memory: &dyn RamPeek, value: u32) {
        if let Some(query_id) = self.current_query_id.take() {
            eprintln!(
                "Current query ID = 0x{query_id:08x} iterator is not consumed in full, but received value 0x{value:08x}"
            );
        }

        // may have something from remains
        if let Some(iterator) = self.current_iterator.take() {
            if iterator.len() != 0 {
                eprintln!(
                    "Current iterator is not consumed in full, but received value 0x{value:08x}"
                );
            }
        }
        self.iterator_len_to_indicate = None;
        self.high_half = None;
        if !self.memory_response.is_empty() {
            eprintln!(
                "Response to a memory-based query is not consumed in full, but received value 0x{value:08x}"
            );
            self.memory_response.clear();
        }

        if let Some(query_id) = self.memory_query_awaiting_input.take() {
            // a memory-based query is two words: the ID and the input word, which may be a guest
            // address that the processor reads through while the guest waits
            if query_id == DISCONNECT_ORACLE_QUERY_ID {
                self.is_connected_to_external_oracle = false;
            } else {
                assert_eq!(
                    self.mode,
                    RunMode::RiscVRun,
                    "the oracle serves a native run, not the RISC-V guest"
                );
                self.process_memory_query(query_id, value as usize, &GuestMemory(memory))
                    .expect("must serve the memory-based query");
            }
        } else if let Some(query_buffer) = self.query_buffer.as_mut() {
            let complete = query_buffer.write(value);
            if complete {
                self.process_buffered_query(memory);
            }
        } else {
            if !self.is_connected_to_external_oracle && value != UART_QUERY_ID {
                // we are not interested in general to start another query
                return;
            }

            if self.memory_query_ranges.contains_key(&value) || value == DISCONNECT_ORACLE_QUERY_ID
            {
                self.memory_query_awaiting_input = Some(value);
                return;
            }

            let new_buffer = QueryBuffer::empty_for_query_type(value);
            self.query_buffer = Some(new_buffer);
        }
    }
}

/// The querier runs in this process (forward mode, and the native run that records the prover
/// input).
impl<E: OracleError> MemoryOracle for ZkEENonDeterminismSource<E> {
    type Error = E;

    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), E> {
        if !self.mode.produces_native_run_responses() {
            return Err(E::new(
                "the oracle serves the RISC-V guest, not a querier in this process",
            ));
        }
        self.guest_run_response.clear();
        if !self.memory_response.is_empty() {
            self.memory_response.clear();
            return Err(E::new("previous oracle response was not consumed in full"));
        }
        if query_id == DISCONNECT_ORACLE_QUERY_ID {
            self.is_connected_to_external_oracle = false;
        }
        if !self.is_connected_to_external_oracle {
            // as when running as the CSR source, reads return zeroes
            return Ok(());
        }
        // SAFETY: the querier of this process exposes the memory the input word refers to while
        // it sends the query, i.e. during this call
        let memory = unsafe { NativeQuerierMemory::new() };
        self.process_memory_query(query_id, input_word, &memory)
    }

    fn read_word(&mut self) -> Result<u32, E> {
        match self.memory_response.pop_front() {
            Some(word) => Ok(word),
            None if !self.is_connected_to_external_oracle => Ok(0),
            None => Err(E::new("oracle response is shorter than expected")),
        }
    }

    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), E> {
        if self.memory_response.len() < num_words {
            if !self.is_connected_to_external_oracle {
                // SAFETY: guaranteed by the caller
                unsafe { dst.write_bytes(0, num_words) };
                return Ok(());
            }
            return Err(E::new("oracle response is shorter than expected"));
        }
        for (i, word) in self.memory_response.drain(..num_words).enumerate() {
            // SAFETY: `i < num_words`, and the caller guarantees `dst` is valid for `num_words`
            // words
            unsafe { dst.add(i).write(word) };
        }
        Ok(())
    }

    fn finish_query(&mut self) -> Result<(), E> {
        if self.memory_response.is_empty() {
            Ok(())
        } else {
            self.memory_response.clear();
            Err(E::new("oracle response contains excess data"))
        }
    }
}

/// The non-determinism CSR of the RISC-V guest in the simulator.
impl<E: OracleError> NonDeterminismCSRSource for ZkEENonDeterminismSource<E> {
    fn read(&mut self) -> u32 {
        self.read_csr_word()
    }

    fn write_with_memory_access<R: RamPeek + ?Sized>(&mut self, ram: &R, value: u32) {
        self.write_csr_word(&PeekRef(ram), value);
    }

    fn write_with_memory_access_raw(&mut self, ram: &[u32], value: u32) {
        self.write_csr_word(&PeekRef(ram), value);
    }

    fn write_with_memory_access_dyn(&mut self, ram: &dyn RamPeek, value: u32) {
        self.write_csr_word(ram, value);
    }
}

struct QueryBuffer {
    query_type: u32,
    remaining_len: Option<usize>,
    write_low: bool,
    buffer: Vec<usize>,
}

impl QueryBuffer {
    fn empty_for_query_type(query_type: u32) -> Self {
        Self {
            query_type,
            remaining_len: None,
            write_low: true,
            buffer: Vec::new(),
        }
    }

    fn write(&mut self, value: u32) -> bool {
        // NOTE: we have to match between 32 bit inner env and 64 bit outer env
        if let Some(remaining_len) = self.remaining_len.as_mut() {
            if self.write_low {
                self.buffer.push(value as usize);
                self.write_low = false;
            } else {
                let last = self.buffer.last_mut().unwrap();
                *last |= (value as usize) << 32;
                self.write_low = true;
            }
            *remaining_len -= 1;

            *remaining_len == 0
        } else {
            self.remaining_len = Some(value as usize);
            // with nothing else to expect, the query is complete
            value == 0
        }
    }
}

impl<E: OracleError> Drop for ZkEENonDeterminismSource<E> {
    fn drop(&mut self) {
        query_timing::report_and_reset();
    }
}

/// Opt-in accounting of the host time spent answering oracle queries, per query ID and, for
/// processors that report it, per operation inside the query. Enabled by the environment
/// variable `ORACLE_QUERY_TIMING`; the table is printed to stderr when an oracle is dropped.
pub mod query_timing {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    type Key = (u32, Option<u32>);
    static ENABLED: OnceLock<bool> = OnceLock::new();
    static STATS: Mutex<BTreeMap<Key, (u64, Duration)>> = Mutex::new(BTreeMap::new());

    pub fn enabled() -> bool {
        *ENABLED.get_or_init(|| std::env::var_os("ORACLE_QUERY_TIMING").is_some())
    }

    /// The start of a timed section, `None` when the accounting is off
    pub fn start() -> Option<Instant> {
        enabled().then(Instant::now)
    }

    /// Adds the time since `started` to the query `query_id` (`operation` = a kind inside it)
    pub fn record(query_id: u32, operation: Option<u32>, started: Option<Instant>) {
        let Some(started) = started else { return };
        let elapsed = started.elapsed();
        let mut stats = STATS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = stats.entry((query_id, operation)).or_default();
        entry.0 += 1;
        entry.1 += elapsed;
    }

    pub fn report_and_reset() {
        if !enabled() {
            return;
        }
        let stats = core::mem::take(
            &mut *STATS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        if stats.is_empty() {
            return;
        }
        let mut rows: Vec<_> = stats.into_iter().collect();
        rows.sort_by_key(|(_, (_, time))| core::cmp::Reverse(*time));
        let total: Duration = rows
            .iter()
            .filter(|((_, operation), _)| operation.is_none())
            .map(|(_, (_, time))| *time)
            .sum();
        eprintln!("oracle query timing, {total:?} in total:");
        for ((query_id, operation), (count, time)) in rows {
            let kind = match operation {
                Some(operation) => format!("0x{query_id:08x} op {operation}"),
                None => format!("0x{query_id:08x}"),
            };
            eprintln!(
                "  {kind:<22} {count:>9} queries {:>12.3} ms {:>10.2} us/query",
                time.as_secs_f64() * 1e3,
                time.as_secs_f64() * 1e6 / count as f64
            );
        }
    }
}

/// Wraps the original source and remembers all the read accesses: the words the guest reads as
/// its non-determinism input when it runs the same computation.
pub struct ReadWitnessSource<E: OracleError> {
    original_source: ZkEENonDeterminismSource<E>,
    read_items: Rc<RefCell<Vec<u32>>>,
}

impl<E: OracleError> ReadWitnessSource<E> {
    pub fn new(original_source: ZkEENonDeterminismSource<E>) -> Self {
        Self {
            original_source,
            read_items: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn get_read_items(&self) -> Rc<RefCell<Vec<u32>>> {
        self.read_items.clone()
    }

    /// The wrapped oracle.
    pub fn inner(&self) -> &ZkEENonDeterminismSource<E> {
        &self.original_source
    }

    /// As [`ZkEENonDeterminismSource::raw_query`], recording the length and the words of the
    /// response as the guest reads them.
    pub fn raw_query(
        &mut self,
        query_type: u32,
        input: Vec<usize>,
    ) -> Result<Box<dyn ExactSizeIterator<Item = usize> + 'static>, E> {
        let inner = self.original_source.raw_query(query_type, input)?;
        // First add the length of the iterator.
        let len = inner.len();
        {
            let mut read_items = self.read_items.borrow_mut();
            // Len is multiplied by 2 to account for 32/64-bit mismatch
            let len_u32 = u32::try_from(len.checked_mul(2).expect("response length overflow"))
                .expect("iterator length does not fit into u32");
            read_items.push(len_u32);
        }
        let read_items = Rc::clone(&self.read_items);
        let wrapped: Box<dyn ExactSizeIterator<Item = usize> + 'static> =
            Box::new(inner.inspect(move |v| {
                record_usize_as_u32_words(&mut read_items.borrow_mut(), *v);
            }));
        Ok(wrapped)
    }
}

impl<E: OracleError> NonDeterminismCSRSource for ReadWitnessSource<E> {
    fn read(&mut self) -> u32 {
        let item = self.original_source.read();
        // On read - remember the items.
        self.read_items.borrow_mut().push(item);
        item
    }

    fn write_with_memory_access<R: RamPeek + ?Sized>(&mut self, ram: &R, value: u32) {
        self.original_source.write_with_memory_access(ram, value);
    }

    fn write_with_memory_access_raw(&mut self, ram: &[u32], value: u32) {
        self.original_source
            .write_with_memory_access_raw(ram, value);
    }

    fn write_with_memory_access_dyn(&mut self, ram: &dyn RamPeek, value: u32) {
        self.original_source
            .write_with_memory_access_dyn(ram, value);
    }
}

/// Records the responses of the RISC-V guest run, the words the guest reads in place of the
/// querier of this process, which the processors produce along with the responses they serve in
/// the [`RunMode::NativeRunSavingForRiscV`] mode (see
/// [`OracleQueryProcessor::process_memory_query`]). A disconnected oracle does not respond, and
/// the zeroes the querier reads then are recorded as read: the guest reads zeroes then too.
impl<E: OracleError> MemoryOracle for ReadWitnessSource<E> {
    type Error = E;

    unsafe fn send_query(&mut self, query_id: u32, input_word: usize) -> Result<(), E> {
        if self.original_source.mode != RunMode::NativeRunSavingForRiscV {
            return Err(E::new(
                "recording the prover input needs the run mode that saves the responses for the RISC-V guest",
            ));
        }
        // SAFETY: the caller's contract, unchanged
        unsafe { self.original_source.send_query(query_id, input_word)? };
        let guest_run_response = core::mem::take(&mut self.original_source.guest_run_response);
        self.read_items.borrow_mut().extend(guest_run_response);
        Ok(())
    }

    fn read_word(&mut self) -> Result<u32, E> {
        let word = self.original_source.read_word()?;
        if !self.original_source.is_connected_to_external_oracle {
            self.read_items.borrow_mut().push(word);
        }
        Ok(word)
    }

    unsafe fn write_words(&mut self, dst: *mut u32, num_words: usize) -> Result<(), E> {
        // SAFETY: guaranteed by the caller
        unsafe { self.original_source.write_words(dst, num_words)? };
        if !self.original_source.is_connected_to_external_oracle {
            // SAFETY: the words were just written
            let words = unsafe { core::slice::from_raw_parts(dst, num_words) };
            self.read_items.borrow_mut().extend_from_slice(words);
        }
        Ok(())
    }

    fn finish_query(&mut self) -> Result<(), E> {
        self.original_source.finish_query()
    }
}

/// Appends `value` as the guest reads a `usize` item: two words, low half first.
pub fn record_usize_as_u32_words(dst: &mut Vec<u32>, value: usize) {
    let v = value as u64;
    dst.push((v & 0xFFFF_FFFF) as u32);
    dst.push((v >> 32) as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestError(&'static str);

    impl OracleError for TestError {
        fn new(message: &'static str) -> Self {
            Self(message)
        }
    }

    type Source = ZkEENonDeterminismSource<TestError>;

    const TEST_QUERY_ID: u32 = 0x1234_5678;

    struct FixedResponseProcessor;

    impl OracleQueryProcessor for FixedResponseProcessor {
        fn supported_query_ids(&self) -> Vec<u32> {
            vec![TEST_QUERY_ID]
        }

        fn process_buffered_query(
            &mut self,
            query_id: u32,
            query: Vec<usize>,
            _memory: &dyn RamPeek,
        ) -> BoxedResponseIterator {
            assert_eq!(query_id, TEST_QUERY_ID);
            assert_eq!(query, vec![7usize]);
            Box::new(vec![0x1122_3344_5566_7788usize, 0x99aa_bbcc_ddee_ff00usize].into_iter())
        }
    }

    #[test]
    fn read_witness_source_records_length_and_words() {
        let mut oracle = Source::default();
        oracle.add_external_processor(FixedResponseProcessor);

        let mut source = ReadWitnessSource::new(oracle);
        let response: Vec<usize> = source.raw_query(TEST_QUERY_ID, vec![7]).unwrap().collect();
        assert_eq!(
            response,
            vec![0x1122_3344_5566_7788usize, 0x99aa_bbcc_ddee_ff00usize]
        );
        assert_eq!(
            *source.get_read_items().borrow(),
            vec![4, 0x5566_7788, 0x1122_3344, 0xddee_ff00, 0x99aa_bbcc,]
        );
    }

    const SWAP_QUERY_ID: u32 = 0x1234_0001;

    /// Reads two 8-word values at the addresses of the array at the input word, and answers
    /// them swapped
    struct SwapProcessor;

    impl OracleQueryProcessor for SwapProcessor {
        fn supported_memory_query_ids(&self) -> Vec<u32> {
            vec![SWAP_QUERY_ID]
        }

        fn process_memory_query(
            &mut self,
            query_id: u32,
            input_word: usize,
            memory: &dyn QuerierMemory,
            mode: RunMode,
            native_run_responses: &mut Vec<u32>,
            guest_run_responses: &mut Vec<u32>,
        ) {
            use crate::querier_memory::{read_querier_u32_words_into, read_querier_word};
            assert_eq!(query_id, SWAP_QUERY_ID);
            let word_size = memory.word_size();
            let a = read_querier_word(memory, input_word).unwrap();
            let b = read_querier_word(memory, input_word + word_size).unwrap();
            let mut response = vec![0u32; 16];
            read_querier_u32_words_into(memory, b, &mut response[..8]).unwrap();
            read_querier_u32_words_into(memory, a, &mut response[8..]).unwrap();
            respond_to_every_target(mode, response, native_run_responses, guest_run_responses);
        }
    }

    const REPRESENTATION_QUERY_ID: u32 = 0x1234_0003;

    /// Echoes the 8 words at the input word, in a representation that depends on the target: as
    /// is for a querier in this process, in reverse order for the guest.
    struct RepresentationProcessor {
        follows_mode: bool,
    }

    impl OracleQueryProcessor for RepresentationProcessor {
        fn supported_memory_query_ids(&self) -> Vec<u32> {
            vec![REPRESENTATION_QUERY_ID]
        }

        fn process_memory_query(
            &mut self,
            query_id: u32,
            input_word: usize,
            memory: &dyn QuerierMemory,
            mode: RunMode,
            native_run_responses: &mut Vec<u32>,
            guest_run_responses: &mut Vec<u32>,
        ) {
            assert_eq!(query_id, REPRESENTATION_QUERY_ID);
            let mut words = [0u32; 8];
            crate::querier_memory::read_querier_u32_words_into(memory, input_word, &mut words)
                .unwrap();
            if !self.follows_mode || mode.produces_native_run_responses() {
                native_run_responses.extend_from_slice(&words);
            }
            if self.follows_mode && mode.produces_guest_run_responses() {
                guest_run_responses.extend(words.iter().rev());
            }
        }
    }

    const REPRESENTATION: RepresentationProcessor = RepresentationProcessor { follows_mode: true };

    fn words(seed: u32) -> [u32; 8] {
        core::array::from_fn(|i| seed.wrapping_add(i as u32))
    }

    /// The swap query of a querier in this process
    fn swap<O: MemoryOracle>(
        oracle: &mut O,
        a: &[u32; 8],
        b: &[u32; 8],
    ) -> Result<[u32; 16], O::Error> {
        let addresses = [
            a.as_ptr().expose_provenance(),
            b.as_ptr().expose_provenance(),
        ];
        // SAFETY: the array of addresses and the operands are alive during the query
        unsafe { oracle.send_query(SWAP_QUERY_ID, addresses.as_ptr().expose_provenance())? };
        let mut response = [0u32; 16];
        // SAFETY: the buffer holds 16 words
        unsafe { oracle.write_words(response.as_mut_ptr(), 16)? };
        oracle.finish_query()?;
        Ok(response)
    }

    /// The echo query of a querier in this process
    fn echo<O: MemoryOracle>(oracle: &mut O, a: &[u32; 8]) -> Result<[u32; 8], O::Error> {
        // SAFETY: the operand is alive during the query
        unsafe { oracle.send_query(REPRESENTATION_QUERY_ID, a.as_ptr().expose_provenance())? };
        let mut response = [0u32; 8];
        // SAFETY: the buffer holds 8 words
        unsafe { oracle.write_words(response.as_mut_ptr(), 8)? };
        oracle.finish_query()?;
        Ok(response)
    }

    #[test]
    fn memory_queries_in_process_record_the_words_the_guest_reads() {
        let mut oracle = Source::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        let (a, b) = (words(1), words(100));
        assert_eq!(
            swap(&mut source, &a, &b),
            Ok([b, a].concat().try_into().unwrap())
        );
        assert_eq!(*source.get_read_items().borrow(), [b, a].concat());
    }

    #[test]
    fn memory_queries_as_the_csr_source_of_the_guest() {
        let mut oracle = Source::new(RunMode::RiscVRun);
        oracle.add_external_processor(FixedResponseProcessor);
        oracle.add_external_processor(SwapProcessor);
        let (a, b) = (words(1), words(100));
        // guest memory: `a` at 0x100, `b` at 0x200, and the address array of the composite at 0x300
        let mut ram = [0u32; 256];
        ram[0x40..0x48].copy_from_slice(&a);
        ram[0x80..0x88].copy_from_slice(&b);
        ram[0xc0] = 0x100;
        ram[0xc1] = 0x200;

        for _ in 0..2 {
            // a memory-based query: the ID and the input word, then exactly the response words
            oracle.write_with_memory_access(&ram, SWAP_QUERY_ID);
            oracle.write_with_memory_access(&ram, 0x300);
            let response: Vec<u32> = (0..16).map(|_| oracle.read()).collect();
            assert_eq!(response, [b, a].concat());

            // an iterator-based query in between keeps its framing
            for word in [TEST_QUERY_ID, 2, 7, 0] {
                oracle.write_with_memory_access(&ram, word);
            }
            let response: Vec<u32> = (0..5).map(|_| oracle.read()).collect();
            assert_eq!(
                response,
                [4, 0x5566_7788, 0x1122_3344, 0xddee_ff00, 0x99aa_bbcc]
            );
        }
    }

    fn reversed(value: &[u32; 8]) -> [u32; 8] {
        let mut words = *value;
        words.reverse();
        words
    }

    #[test]
    fn a_native_run_is_served_its_responses_and_records_those_of_the_guest_run() {
        let (a, b) = (words(1), words(100));

        let mut oracle = Source::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(REPRESENTATION);
        assert_eq!(echo(&mut oracle, &a), Ok(a));

        let mut oracle = Source::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(REPRESENTATION);
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        assert_eq!(echo(&mut source, &a), Ok(a));
        // a response that is the same on every target
        assert_eq!(
            swap(&mut source, &a, &b),
            Ok([b, a].concat().try_into().unwrap())
        );
        let expected = [reversed(&a), b, a].concat();
        assert_eq!(*source.get_read_items().borrow(), expected);
    }

    #[test]
    fn the_guest_is_served_the_responses_of_the_guest_run() {
        let mut oracle = Source::new(RunMode::RiscVRun);
        oracle.add_external_processor(REPRESENTATION);
        let a = words(1);
        // guest memory: `a` at 0x100
        let mut ram = [0u32; 128];
        ram[0x40..0x48].copy_from_slice(&a);
        oracle.write_with_memory_access(&ram, REPRESENTATION_QUERY_ID);
        oracle.write_with_memory_access(&ram, 0x100);
        let response: Vec<u32> = (0..8).map(|_| oracle.read()).collect();
        assert_eq!(response, reversed(&a));
    }

    #[test]
    fn recording_needs_the_mode_that_saves_the_responses_of_the_guest_run() {
        let mut oracle = Source::new(RunMode::NativeRunOnly);
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        let a = words(1);
        assert!(swap(&mut source, &a, &a).is_err());
        assert!(source.get_read_items().borrow().is_empty());
    }

    #[test]
    fn a_processor_that_does_not_respond_for_the_mode_is_rejected() {
        let mut oracle = Source::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(RepresentationProcessor {
            follows_mode: false,
        });
        oracle.add_external_processor(SwapProcessor);
        let mut source = ReadWitnessSource::new(oracle);
        let a = words(1);
        assert_eq!(
            echo(&mut source, &a),
            Err(TestError(
                "the query processor does not respond for the run mode"
            ))
        );
        // the failed query is over
        assert_eq!(
            swap(&mut source, &a, &a),
            Ok([a, a].concat().try_into().unwrap())
        );
        assert_eq!(*source.get_read_items().borrow(), [a, a].concat());
    }

    #[test]
    fn a_querier_in_this_process_is_not_served_in_a_risc_v_run() {
        let mut oracle = Source::new(RunMode::RiscVRun);
        oracle.add_external_processor(REPRESENTATION);
        assert!(echo(&mut oracle, &words(1)).is_err());
    }

    #[test]
    #[should_panic(expected = "not the RISC-V guest")]
    fn the_guest_is_not_served_in_a_native_run() {
        let mut oracle = Source::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(REPRESENTATION);
        let ram = [0u32; 128];
        oracle.write_with_memory_access(&ram, REPRESENTATION_QUERY_ID);
        oracle.write_with_memory_access(&ram, 0x100);
    }

    #[test]
    fn a_disconnected_oracle_records_the_zeroes_read() {
        let mut oracle = Source::new(RunMode::NativeRunSavingForRiscV);
        oracle.add_external_processor(REPRESENTATION);
        let mut source = ReadWitnessSource::new(oracle);
        // SAFETY: the disconnect query has no input
        unsafe { source.send_query(DISCONNECT_ORACLE_QUERY_ID, 0) }.unwrap();
        source.finish_query().unwrap();
        assert!(!source.inner().is_connected());
        assert_eq!(echo(&mut source, &words(1)), Ok([0; 8]));
        assert_eq!(*source.get_read_items().borrow(), vec![0; 8]);
    }

    #[test]
    #[should_panic(expected = "more than one processor")]
    fn a_query_id_is_served_with_one_protocol() {
        struct IteratorSwap;

        impl OracleQueryProcessor for IteratorSwap {
            fn supported_query_ids(&self) -> Vec<u32> {
                vec![SWAP_QUERY_ID]
            }
        }

        let mut oracle = Source::default();
        oracle.add_external_processor(SwapProcessor);
        oracle.add_external_processor(IteratorSwap);
    }
}
