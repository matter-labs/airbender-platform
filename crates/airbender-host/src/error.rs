use core::fmt;

#[derive(Debug)]
pub enum HostError {
    Io(std::io::Error),
    Codec(airbender_codec::CodecError),
    InvalidManifest(String),
    Simulator(String),
    Transpiler(String),
    Prover(String),
    Verification(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::Io(err) => write!(f, "io error: {err}"),
            HostError::Codec(err) => write!(f, "codec error: {err}"),
            HostError::InvalidManifest(err) => write!(f, "invalid manifest: {err}"),
            HostError::Simulator(err) => write!(f, "simulator error: {err}"),
            HostError::Transpiler(err) => write!(f, "transpiler error: {err}"),
            HostError::Prover(err) => write!(f, "prover error: {err}"),
            HostError::Verification(err) => write!(f, "verification error: {err}"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        HostError::Io(err)
    }
}

impl From<airbender_codec::CodecError> for HostError {
    fn from(err: airbender_codec::CodecError) -> Self {
        HostError::Codec(err)
    }
}

pub type Result<T> = std::result::Result<T, HostError>;
