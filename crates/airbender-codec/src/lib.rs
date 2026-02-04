#![no_std]

//! Versioned serialization and framing for Airbender host/guest communication.

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;

#[cfg(test)]
extern crate std;

/// Stable codec version for host/guest communication.
pub const AIRBENDER_CODEC_V0: u32 = 0;

/// A stable, versioned serializer used by Airbender host and guest programs.
pub trait AirbenderCodec {
    /// Version identifier baked into manifests and tooling.
    const VERSION: u32;

    /// Serialize a value into a byte payload.
    fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, CodecError>;

    /// Deserialize a value from a byte payload.
    fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>;
}

/// Initial codec based on `bincode` v2 with a fixed configuration.
pub struct AirbenderCodecV0;

impl AirbenderCodec for AirbenderCodecV0 {
    const VERSION: u32 = AIRBENDER_CODEC_V0;

    fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
        bincode::serde::encode_to_vec(value, bincode::config::standard())
            .map_err(CodecError::Encode)
    }

    fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
        let (decoded, read_len) =
            bincode::serde::decode_from_slice(bytes, bincode::config::standard())
                .map_err(CodecError::Decode)?;
        if read_len != bytes.len() {
            return Err(CodecError::TrailingBytes {
                expected: bytes.len(),
                read: read_len,
            });
        }
        Ok(decoded)
    }
}

/// Encode a byte payload into a word stream with a leading byte-length word.
///
/// Words are big-endian so the guest can reconstruct bytes by reading CSR words.
pub fn frame_words_from_bytes(bytes: &[u8]) -> Vec<u32> {
    let word_count = bytes.len().div_ceil(4);
    let mut words = Vec::with_capacity(1 + word_count);
    words.push(bytes.len() as u32);
    for chunk in bytes.chunks(4) {
        let mut padded = [0u8; 4];
        padded[..chunk.len()].copy_from_slice(chunk);
        words.push(u32::from_be_bytes(padded));
    }
    words
}

/// Reconstruct a byte payload from a length and word stream.
pub fn bytes_from_frame_words(len: usize, words: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(len);
    for word in words {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes.truncate(len);
    bytes
}

#[derive(Debug)]
pub enum CodecError {
    Encode(bincode::error::EncodeError),
    Decode(bincode::error::DecodeError),
    TrailingBytes { expected: usize, read: usize },
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Encode(_) => f.write_str("failed to encode value"),
            CodecError::Decode(_) => f.write_str("failed to decode value"),
            CodecError::TrailingBytes { expected, read } => {
                write!(f, "decoded {read} bytes but expected {expected}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    struct Sample {
        value: u32,
        payload: alloc::vec::Vec<u8>,
    }

    #[test]
    fn codec_roundtrip() {
        let sample = Sample {
            value: 42,
            payload: vec![1u8, 2, 3, 4, 5],
        };
        let encoded = AirbenderCodecV0::encode(&sample).expect("encode");
        let decoded: Sample = AirbenderCodecV0::decode(&encoded).expect("decode");
        assert_eq!(decoded, sample);
    }

    #[test]
    fn framing_roundtrip() {
        let bytes = b"airbender";
        let words = frame_words_from_bytes(bytes);
        assert_eq!(words[0], bytes.len() as u32);
        let reconstructed = bytes_from_frame_words(bytes.len(), &words[1..]);
        assert_eq!(reconstructed, bytes);
    }
}
