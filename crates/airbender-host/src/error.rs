#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("codec error: {0}")]
    Codec(airbender_codec::CodecError),
    #[error("wire error: {0}")]
    Wire(airbender_core::wire::WireError),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("simulator error: {0}")]
    Simulator(String),
    #[error("transpiler error: {0}")]
    Transpiler(String),
    #[error("runner error: {0}")]
    Runner(String),
    #[error("prover error: {0}")]
    Prover(String),
    #[error("verification error: {0}")]
    Verification(String),
}

pub type Result<T> = std::result::Result<T, HostError>;

impl From<airbender_codec::CodecError> for HostError {
    fn from(err: airbender_codec::CodecError) -> Self {
        Self::Codec(err)
    }
}

impl From<airbender_core::wire::WireError> for HostError {
    fn from(err: airbender_core::wire::WireError) -> Self {
        Self::Wire(err)
    }
}
