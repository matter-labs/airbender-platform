#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use airbender_core::wire::FramedRead;
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

/// Bridges a [`FramedRead`] source (owned by `airbender-core`, bincode-free) to
/// [`bincode`]'s reader trait, so the framing layer stays decoupled from the
/// serializer. Maps frame exhaustion to bincode's `UnexpectedEnd`.
struct FramedBincodeReader<'a, R: FramedRead>(&'a mut R);

impl<R: FramedRead> bincode::de::read::Reader for FramedBincodeReader<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), bincode::error::DecodeError> {
        self.0
            .read(bytes)
            .map_err(|end| bincode::error::DecodeError::UnexpectedEnd {
                additional: end.shortfall,
            })
    }
}

impl AirbenderCodecV0 {
    /// Decode a value from a streaming [`FramedRead`] source using the same
    /// configuration as [`AirbenderCodec::decode`].
    ///
    /// Unlike the slice-based `decode`, the end-of-input check is left to the
    /// caller: a reader knows the frame bounds, bincode does not. Callers that
    /// need the codec's trailing-byte strictness should verify the reader is
    /// fully consumed and raise [`CodecError::TrailingBytes`] themselves (see
    /// `airbender_core::wire::FramedReader::remaining`).
    ///
    /// For `DeserializeOwned` types this consumes exactly the same bytes with
    /// the same validation as `decode`. One failure-mode difference: a
    /// `deserialize_bytes` visitor (e.g. `serde_bytes`) is served zero-copy by
    /// the slice path but goes through `Vec<u8>::decode` here, which allocates
    /// the claimed length up front — so a bogus inner length fails as an
    /// allocation abort rather than a clean `DecodeError`. Ordinary `Vec<u8>`
    /// and `String` fields are unaffected (both paths allocate up front).
    pub fn decode_from_reader<T, R>(reader: &mut R) -> Result<T, CodecError>
    where
        T: serde::de::DeserializeOwned,
        R: FramedRead,
    {
        bincode::serde::decode_from_reader(FramedBincodeReader(reader), bincode::config::standard())
            .map_err(CodecError::Decode)
    }
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
}
