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
/// (~1x) rather than blob-plus-value (~2x).
///
/// For any input the buffered path could allocate, the accept/reject set is
/// identical: same bincode config, and a value that does not consume the whole
/// frame is still a [`CodecError::TrailingBytes`]. Streaming additionally
/// succeeds on inputs so large the buffered path's up-front `Vec::with_capacity`
/// would fail — which is the point of the change.
///
/// Like the buffered path, the whole frame is consumed from `transport` whether
/// the decode succeeds or fails, so a subsequent `read_with` stays aligned.
pub fn read_with<T: serde::de::DeserializeOwned>(
    transport: &mut impl Transport,
) -> Result<T, GuestError> {
    let mut reader = FramedReader::new(|| transport.read_word());
    let result = AirbenderCodecV0::decode_from_reader(&mut reader);
    // Drain any words the decoder left behind (on error, or trailing bytes)
    // before returning, so the transport is positioned at the next frame.
    reader.discard_rest_of_frame();
    let value = result.map_err(GuestError::Codec)?;
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

    #[test]
    fn rejected_frame_does_not_desync_the_next_frame() {
        // A frame that fails (trailing bytes) must still be fully drained, so a
        // second frame on the same transport decodes correctly afterwards.
        let bad = Payload {
            counter: 1,
            bytes: vec![9u8, 8, 7],
        };
        let good = Payload {
            counter: 42,
            bytes: vec![1u8, 2, 3, 4, 5],
        };

        let mut bad_encoded = AirbenderCodecV0::encode(&bad).expect("encode");
        bad_encoded.extend_from_slice(&[0u8; 6]); // trailing bytes -> rejected frame
        let good_encoded = AirbenderCodecV0::encode(&good).expect("encode");

        let mut words = frame_words_from_bytes(&bad_encoded).expect("frame bad");
        words.extend(frame_words_from_bytes(&good_encoded).expect("frame good"));
        let mut transport = MockTransport::new(words);

        let err = read_with::<Payload>(&mut transport).expect_err("first frame rejected");
        assert!(matches!(
            err,
            GuestError::Codec(CodecError::TrailingBytes { .. })
        ));
        // The second frame is intact only if the first was fully consumed.
        let decoded: Payload = read_with(&mut transport).expect("second frame decodes");
        assert_eq!(decoded, good);
    }

    #[test]
    fn mid_decode_failure_does_not_desync_the_next_frame() {
        // Exercises the `decode == Err -> discard_rest_of_frame` branch (the
        // trailing-bytes test above hits the `decode == Ok` branch): a frame
        // that fails *during* decode, with words still unread, must still be
        // drained so the following frame decodes.
        #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
        struct HasBool {
            flag: bool,
            rest: u32,
        }

        // 0x02 is not a valid bool discriminant, so decode fails after one byte,
        // leaving the rest of this 8-byte (2-word) frame unread.
        let bad_frame_bytes = [2u8, 0, 0, 0, 0, 0, 0, 0];
        let good = Payload {
            counter: 99,
            bytes: vec![7u8, 7, 7],
        };
        let good_encoded = AirbenderCodecV0::encode(&good).expect("encode");

        let mut words = frame_words_from_bytes(&bad_frame_bytes).expect("frame bad");
        words.extend(frame_words_from_bytes(&good_encoded).expect("frame good"));
        let mut transport = MockTransport::new(words);

        let err = read_with::<HasBool>(&mut transport).expect_err("decode must fail");
        assert!(matches!(err, GuestError::Codec(CodecError::Decode(_))));
        let decoded: Payload = read_with(&mut transport).expect("second frame decodes");
        assert_eq!(decoded, good);
    }
}
