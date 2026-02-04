//! Word-based transports for guest communication.

/// Word-based transport used by the guest to communicate with the host.
pub trait Transport {
    fn read_word(&mut self) -> u32;
    fn write_word(&mut self, word: u32);
}

/// CSR-backed transport for real guest execution.
pub struct CsrTransport;

#[cfg(target_arch = "riscv32")]
impl Transport for CsrTransport {
    fn read_word(&mut self) -> u32 {
        airbender_rt::sys::read_word()
    }

    fn write_word(&mut self, word: u32) {
        airbender_rt::sys::write_word(word);
    }
}

#[cfg(not(target_arch = "riscv32"))]
impl Transport for CsrTransport {
    fn read_word(&mut self) -> u32 {
        panic!("csr transport is only available on riscv32")
    }

    fn write_word(&mut self, _word: u32) {
        panic!("csr transport is only available on riscv32")
    }
}

/// In-memory transport for unit tests and host-side checks.
#[derive(Default)]
pub struct MockTransport {
    reads: alloc::vec::Vec<u32>,
    writes: alloc::vec::Vec<u32>,
    cursor: usize,
}

impl MockTransport {
    /// Create a mock transport seeded with preloaded read words.
    pub fn new(reads: alloc::vec::Vec<u32>) -> Self {
        Self {
            reads,
            writes: alloc::vec::Vec::new(),
            cursor: 0,
        }
    }

    /// Append additional read words to the transport.
    pub fn push_reads(&mut self, words: impl IntoIterator<Item = u32>) {
        self.reads.extend(words);
    }

    /// Inspect words written by the guest.
    pub fn writes(&self) -> &[u32] {
        &self.writes
    }

    /// Consume the transport and return captured writes.
    pub fn into_writes(self) -> alloc::vec::Vec<u32> {
        self.writes
    }
}

impl Transport for MockTransport {
    fn read_word(&mut self) -> u32 {
        let Some(word) = self.reads.get(self.cursor) else {
            panic!("mock transport exhausted");
        };
        self.cursor += 1;
        *word
    }

    fn write_word(&mut self, word: u32) {
        self.writes.push(word);
    }
}
