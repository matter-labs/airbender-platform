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

/// Error from [`FramedRead::read`]: the frame ran out before the request could
/// be satisfied. `shortfall` is how many more bytes were requested than the
/// frame still holds; on this error the reader is left unadvanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndOfFrame {
    pub shortfall: usize,
}

/// A source of framed payload bytes read on demand. This keeps the framing and
/// word-transport concern in `core`, decoupled from any serializer — a bincode
/// bridge over `FramedRead` lives in the codec crate, so `core` needs no
/// dependency on bincode.
pub trait FramedRead {
    /// Fill `out` completely, or return [`EndOfFrame`] without advancing if the
    /// frame does not hold that many more bytes.
    fn read(&mut self, out: &mut [u8]) -> Result<(), EndOfFrame>;
}

/// Streaming counterpart to [`read_framed_bytes_with`]: a [`FramedRead`] source
/// that pulls framed words on demand instead of first materializing the whole
/// payload into a `Vec<u8>`. Only a single word is buffered at a time, so it
/// holds O(1) memory regardless of payload size — letting a decoder run at ~1x
/// peak memory rather than the ~2x of "buffer the blob, then decode it".
///
/// The word source must yield the frame length word first, then payload words,
/// exactly as [`frame_words_from_bytes`] lays them out.
pub struct FramedReader<F: FnMut() -> u32> {
    read_word: F,
    len: usize,
    remaining: usize,
    /// Payload words already pulled from the source, so [`Self::discard_rest_of_frame`]
    /// can consume the rest of the frame without over- or under-reading.
    pulled_words: usize,
    word: [u8; WORD_BYTES],
    /// Index of the next byte to hand out of `word`; `WORD_BYTES` means empty.
    word_pos: usize,
}

impl<F: FnMut() -> u32> FramedReader<F> {
    /// Consume the leading length word and prepare to stream the payload.
    pub fn new(mut read_word: F) -> Self {
        let len = read_word() as usize;
        Self {
            read_word,
            len,
            remaining: len,
            pulled_words: 0,
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

    /// Total number of payload words in the frame.
    fn frame_words(&self) -> usize {
        self.len.div_ceil(WORD_BYTES)
    }

    /// Pull and discard any payload words the decoder did not consume, leaving
    /// the source positioned at the next frame's length word. Buffered/decoded
    /// data is not touched, so [`Self::remaining`] still reports trailing bytes.
    ///
    /// Callers should invoke this before returning — success or failure — so a
    /// rejected frame does not desync a subsequent read, matching the buffered
    /// [`read_framed_bytes_with`] which always consumes the whole frame.
    ///
    /// Cost is bounded by the frame's length word: draining an honestly-framed
    /// frame reads only the words the source actually holds, but a frame whose
    /// length word is far larger than its real content will issue that many
    /// `read_word` calls. This is only reachable via a caller that recovers from
    /// an error and keeps reading; it is not a concern for honestly-framed input
    /// (as the guest's is) where the length word reflects the payload.
    pub fn discard_rest_of_frame(&mut self) {
        let frame_words = self.frame_words();
        while self.pulled_words < frame_words {
            (self.read_word)();
            self.pulled_words += 1;
        }
    }
}

impl<F: FnMut() -> u32> FramedRead for FramedReader<F> {
    fn read(&mut self, out: &mut [u8]) -> Result<(), EndOfFrame> {
        // Reject an unsatisfiable request up front so a failed read never leaves
        // the reader partially advanced.
        if self.remaining < out.len() {
            return Err(EndOfFrame {
                shortfall: out.len() - self.remaining,
            });
        }
        let mut written = 0;
        while written < out.len() {
            if self.word_pos == WORD_BYTES {
                self.word = (self.read_word)().to_be_bytes();
                self.word_pos = 0;
                self.pulled_words += 1;
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

    #[test]
    fn framed_reader_streams_same_bytes_as_buffered() {
        use super::{FramedRead, FramedReader};

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

    #[test]
    fn discard_consumes_exactly_the_rest_of_the_frame() {
        use super::{FramedRead, FramedReader};

        // Two frames back to back; partially read the first, then discard.
        let first = frame_words_from_bytes(b"hello world").expect("frame 1"); // 11 bytes, 3 words
        let second = frame_words_from_bytes(b"next").expect("frame 2");
        let mut words = first.clone();
        words.extend_from_slice(&second);

        let mut cursor = 0;
        let mut reader = FramedReader::new(|| {
            let word = words[cursor];
            cursor += 1;
            word
        });

        // Read only the first 2 bytes, leaving the rest of frame 1 unread.
        let mut out = [0u8; 2];
        reader.read(&mut out).expect("partial read");
        assert_eq!(&out, b"he");
        reader.discard_rest_of_frame();

        // The cursor must now sit exactly at frame 2's length word, i.e. all of
        // frame 1's words (length + payload) have been consumed and no more.
        assert_eq!(cursor, first.len());
        assert_eq!(words[cursor], b"next".len() as u32);
    }
}
