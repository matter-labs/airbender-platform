use crate::error::Result;
use airbender_codec::{frame_words_from_bytes, AirbenderCodec, AirbenderCodecV0};

/// Typed input builder for host-to-guest communication.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    words: Vec<u32>,
}

impl Inputs {
    pub fn new() -> Self {
        Self { words: Vec::new() }
    }

    /// Serialize and append a typed input value.
    pub fn push<T: serde::Serialize>(&mut self, value: &T) -> Result<()> {
        let bytes = AirbenderCodecV0::encode(value)?;
        self.push_bytes(&bytes);
        Ok(())
    }

    /// Append raw bytes as a framed input payload.
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        let words = frame_words_from_bytes(bytes);
        self.words.extend(words);
    }

    /// Access the framed input words.
    pub fn words(&self) -> &[u32] {
        &self.words
    }
}
