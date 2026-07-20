//! Guest input helpers backed by the Airbender codec.

use crate::transport::Transport;
use airbender_codec::{AirbenderCodecV0, CodecError};
use airbender_core::wire::FramedReader;
use core::fmt;

/// Errors that can occur when decoding inputs on the guest.
#[derive(Debug)]
pub enum GuestError {
    Codec(CodecError),
    UnsupportedTarget,
}

impl From<CodecError> for GuestError {
    fn from(err: CodecError) -> Self {
        GuestError::Codec(err)
    }
}

impl fmt::Display for GuestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuestError::Codec(err) => write!(f, "{err}"),
            GuestError::UnsupportedTarget => {
                f.write_str("csr transport is only available on riscv32")
            }
        }
    }
}

/// Read a single value from the CSR-based transport.
pub fn read<T: serde::de::DeserializeOwned>() -> Result<T, GuestError> {
    #[cfg(target_arch = "riscv32")]
    {
        let mut transport = crate::transport::CsrTransport;
        read_with(&mut transport)
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        Err(GuestError::UnsupportedTarget)
    }
}

/// Read a single value using an explicit transport.
///
/// Decodes straight from the framed word transport without first buffering the
/// serialized blob into a `Vec<u8>`, so peak memory is the decoded value alone
/// (~1x) rather than blob-plus-value (~2x). The accept/reject set is identical
/// to the buffered path: same bincode config, and a value that does not consume
/// the whole frame is still a [`CodecError::TrailingBytes`].
pub fn read_with<T: serde::de::DeserializeOwned>(
    transport: &mut impl Transport,
) -> Result<T, GuestError> {
    let mut reader = FramedReader::new(|| transport.read_word());
    let value = AirbenderCodecV0::decode_from_reader(&mut reader).map_err(GuestError::Codec)?;
    let remaining = reader.remaining();
    if remaining != 0 {
        let expected = reader.payload_len();
        return Err(GuestError::Codec(CodecError::TrailingBytes {
            expected,
            read: expected - remaining,
        }));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use airbender_codec::AirbenderCodec; // for `AirbenderCodecV0::encode` in tests
    use airbender_core::wire::frame_words_from_bytes;
    use alloc::vec;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    struct Payload {
        counter: u32,
        bytes: alloc::vec::Vec<u8>,
    }

    #[test]
    fn reads_value_from_transport() {
        let payload = Payload {
            counter: 7,
            bytes: vec![10u8, 20, 30],
        };
        let encoded = AirbenderCodecV0::encode(&payload).expect("encode");
        let words = frame_words_from_bytes(&encoded).expect("frame words");
        let mut transport = MockTransport::new(words);
        let decoded: Payload = read_with(&mut transport).expect("read");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn reads_large_multiword_payload() {
        // Many words plus a big Vec<u8> bincode reads in one chunk — the shape
        // of a real input, and a non-multiple-of-4 length exercises padding.
        let payload = Payload {
            counter: u32::MAX,
            bytes: (0..10_000u32).map(|i| (i * 31 + 7) as u8).collect(),
        };
        let encoded = AirbenderCodecV0::encode(&payload).expect("encode");
        let words = frame_words_from_bytes(&encoded).expect("frame words");
        let mut transport = MockTransport::new(words);
        let decoded: Payload = read_with(&mut transport).expect("read");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn rejects_trailing_bytes_like_the_buffered_codec() {
        // A frame carrying more bytes than the value consumes must fail with
        // `TrailingBytes`, matching the slice-based `AirbenderCodecV0::decode`.
        let payload = Payload {
            counter: 1,
            bytes: vec![9u8],
        };
        let mut encoded = AirbenderCodecV0::encode(&payload).expect("encode");
        encoded.extend_from_slice(&[0u8; 5]); // trailing bytes
        let words = frame_words_from_bytes(&encoded).expect("frame words");
        let mut transport = MockTransport::new(words);
        let err = read_with::<Payload>(&mut transport).expect_err("must reject trailing bytes");
        assert!(matches!(
            err,
            GuestError::Codec(CodecError::TrailingBytes { .. })
        ));
    }
}
