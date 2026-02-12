use crate::error::Result;
use airbender_codec::{frame_words_from_bytes, AirbenderCodec, AirbenderCodecV0};
use std::fmt::Write as _;
use std::path::Path;

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

    /// Write input words as CLI-compatible hex (`8` hex chars per `u32`).
    pub fn write_hex_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut hex = String::new();
        for word in &self.words {
            writeln!(&mut hex, "{word:08x}").expect("writing to string cannot fail");
        }
        std::fs::write(path, hex)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Inputs;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_hex_file_with_one_word_per_line() {
        let mut inputs = Inputs::new();
        inputs.push_bytes(&[0x29]);

        let file_path = test_file_path("inputs-hex");
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("create test parent directory");
        }

        inputs
            .write_hex_file(&file_path)
            .expect("write input hex file");

        let written = fs::read_to_string(&file_path).expect("read written input hex file");
        assert_eq!(written, "00000001\n29000000\n");

        fs::remove_file(&file_path).expect("remove input hex file");
    }

    fn test_file_path(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tmp")
            .join(format!("{prefix}-{timestamp}-{}", std::process::id()))
            .join("input.hex")
    }
}
