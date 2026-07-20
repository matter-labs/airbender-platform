//! Canonical host/guest input wire format.
//!
//! The input stream is encoded as `u32` words where:
//! - the first word stores payload byte length,
//! - each following word stores up to 4 payload bytes in big-endian order,
//! - the final word is zero-padded when payload length is not a multiple of 4.

use alloc::vec::Vec;
use core::fmt;

const WORD_BYTES: usize = 4;

/// Errors that can occur while framing input payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    PayloadTooLarge { len: usize },
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::PayloadTooLarge { len } => {
                write!(f, "payload length {len} exceeds u32 framing limit")
            }
        }
    }
}

fn frame_len_word(len: usize) -> Result<u32, WireError> {
    u32::try_from(len).map_err(|_| WireError::PayloadTooLarge { len })
}

/// Read one framed payload from a word source.
///
/// The provided callback must yield the frame length word first, then payload words.
pub fn read_framed_bytes_with(mut read_word: impl FnMut() -> u32) -> Vec<u8> {
    let len = read_word() as usize;
    let words_needed = len.div_ceil(WORD_BYTES);

    let mut bytes = Vec::with_capacity(len);
    let mut remaining = len;
    for _ in 0..words_needed {
        let word_bytes = read_word().to_be_bytes();
        let bytes_to_take = remaining.min(WORD_BYTES);
        bytes.extend_from_slice(&word_bytes[..bytes_to_take]);
        remaining -= bytes_to_take;
    }

    bytes
}

/// Streaming counterpart to [`read_framed_bytes_with`]: a [`bincode`] reader
/// that pulls framed words on demand instead of first materializing the whole
/// payload into a `Vec<u8>`. Only a single word is buffered at a time, so it
/// holds O(1) memory regardless of payload size — letting a decoder run at ~1x
/// peak memory rather than the ~2x of "buffer the blob, then decode it".
///
/// The word source must yield the frame length word first, then payload words,
/// exactly as [`frame_words_from_bytes`] lays them out.
#[cfg(feature = "stream")]
pub struct FramedReader<F: FnMut() -> u32> {
    read_word: F,
    len: usize,
    remaining: usize,
    word: [u8; WORD_BYTES],
    /// Index of the next byte to hand out of `word`; `WORD_BYTES` means empty.
    word_pos: usize,
}

#[cfg(feature = "stream")]
impl<F: FnMut() -> u32> FramedReader<F> {
    /// Consume the leading length word and prepare to stream the payload.
    pub fn new(mut read_word: F) -> Self {
        let len = read_word() as usize;
        Self {
            read_word,
            len,
            remaining: len,
            word: [0u8; WORD_BYTES],
            // Empty to start, so the first byte requested pulls a word.
            word_pos: WORD_BYTES,
        }
    }

    /// Total framed payload length in bytes (from the leading length word).
    pub fn payload_len(&self) -> usize {
        self.len
    }

    /// Payload bytes not yet handed to the decoder; zero once fully consumed.
    /// A non-zero value after a successful decode means trailing bytes.
    pub fn remaining(&self) -> usize {
        self.remaining
    }
}

#[cfg(feature = "stream")]
impl<F: FnMut() -> u32> bincode::de::read::Reader for FramedReader<F> {
    fn read(&mut self, out: &mut [u8]) -> Result<(), bincode::error::DecodeError> {
        let mut written = 0;
        while written < out.len() {
            // Reject insufficient input before touching any state, so a failed
            // read leaves the reader untouched.
            if self.remaining == 0 {
                return Err(bincode::error::DecodeError::UnexpectedEnd {
                    additional: out.len() - written,
                });
            }
            if self.word_pos == WORD_BYTES {
                self.word = (self.read_word)().to_be_bytes();
                self.word_pos = 0;
            }
            // Bytes left in the current word, capped by unconsumed payload so
            // the final word's zero padding is never handed to the decoder.
            let available = (WORD_BYTES - self.word_pos).min(self.remaining);
            let n = available.min(out.len() - written);
            out[written..written + n].copy_from_slice(&self.word[self.word_pos..self.word_pos + n]);
            self.word_pos += n;
            self.remaining -= n;
            written += n;
        }
        Ok(())
    }
}

/// Frame payload bytes into input words consumed by the runtime.
pub fn frame_words_from_bytes(bytes: &[u8]) -> Result<Vec<u32>, WireError> {
    let len_word = frame_len_word(bytes.len())?;
    let word_count = bytes.len().div_ceil(WORD_BYTES);
    let mut words = Vec::with_capacity(1 + word_count);
    words.push(len_word);
    for chunk in bytes.chunks(WORD_BYTES) {
        let mut padded = [0u8; WORD_BYTES];
        padded[..chunk.len()].copy_from_slice(chunk);
        words.push(u32::from_be_bytes(padded));
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::{frame_len_word, frame_words_from_bytes, read_framed_bytes_with, WireError};

    #[test]
    fn framing_roundtrip() {
        let bytes = b"airbender";
        let words = frame_words_from_bytes(bytes).expect("frame words");
        assert_eq!(words[0], bytes.len() as u32);
        let mut cursor = 0;
        let reconstructed = read_framed_bytes_with(|| {
            let word = words[cursor];
            cursor += 1;
            word
        });
        assert_eq!(reconstructed, bytes);
    }

    #[test]
    fn closure_reader_handles_partial_word() {
        let bytes = [0x12u8, 0x34, 0x56];
        let words = frame_words_from_bytes(&bytes).expect("frame words");
        let mut cursor = 0;
        let reconstructed = read_framed_bytes_with(|| {
            let word = words[cursor];
            cursor += 1;
            word
        });
        assert_eq!(reconstructed, bytes);
    }

    #[test]
    fn rejects_lengths_above_u32_max() {
        let err = frame_len_word(usize::MAX).expect_err("must reject oversized length");
        assert_eq!(err, WireError::PayloadTooLarge { len: usize::MAX });
    }

    #[cfg(feature = "stream")]
    #[test]
    fn framed_reader_streams_same_bytes_as_buffered() {
        use super::FramedReader;
        use bincode::de::read::Reader;

        // Empty, aligned, and padded-final-word lengths.
        for bytes in [b"".as_slice(), b"abcd", b"abcde", b"airbender!!"] {
            let words = frame_words_from_bytes(bytes).expect("frame words");
            let mut cursor = 0;
            let mut reader = FramedReader::new(|| {
                let word = words[cursor];
                cursor += 1;
                word
            });
            assert_eq!(reader.payload_len(), bytes.len());

            let mut out = alloc::vec![0u8; bytes.len()];
            reader.read(&mut out).expect("read payload");
            assert_eq!(out, bytes);
            assert_eq!(reader.remaining(), 0, "payload fully consumed");

            // Reading past the frame errors rather than panicking or over-reading.
            let mut extra = [0u8; 1];
            assert!(reader.read(&mut extra).is_err());
        }
    }
}
