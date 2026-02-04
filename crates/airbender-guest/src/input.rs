//! Guest input helpers backed by the Airbender codec.

use crate::transport::Transport;
use airbender_codec::{bytes_from_frame_words, AirbenderCodec, AirbenderCodecV0, CodecError};
use alloc::vec::Vec;
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
pub fn read_with<T: serde::de::DeserializeOwned>(
    transport: &mut impl Transport,
) -> Result<T, GuestError> {
    let len = transport.read_word() as usize;
    let words_needed = len.div_ceil(4);
    let mut words = Vec::with_capacity(words_needed);
    for _ in 0..words_needed {
        words.push(transport.read_word());
    }
    let bytes = bytes_from_frame_words(len, &words);
    AirbenderCodecV0::decode(&bytes).map_err(GuestError::Codec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use airbender_codec::frame_words_from_bytes;
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
        let words = frame_words_from_bytes(&encoded);
        let mut transport = MockTransport::new(words);
        let decoded: Payload = read_with(&mut transport).expect("read");
        assert_eq!(decoded, payload);
    }
}
