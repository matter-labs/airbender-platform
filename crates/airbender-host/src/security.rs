/// Cryptographic security target for real Airbender proofs.
///
/// The GKR-based prover stack ships a single 100-bit configuration. The level
/// is still recorded in proof and verification-key envelopes so artifacts
/// produced by a different configuration cannot be mixed up silently, and so
/// callers keep an explicit knob once more configurations exist again.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum SecurityLevel {
    /// Use the 100-bit security configuration.
    #[default]
    Bits100,
}

impl SecurityLevel {
    pub fn bits(self) -> u16 {
        match self {
            Self::Bits100 => 100,
        }
    }

    pub(crate) fn to_pipeline(self) -> prover_pipeline::SecurityLevel {
        match self {
            Self::Bits100 => prover_pipeline::SecurityLevel::Sec100,
        }
    }

    pub(crate) fn from_pipeline(level: prover_pipeline::SecurityLevel) -> Self {
        match level {
            prover_pipeline::SecurityLevel::Sec100 => Self::Bits100,
        }
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.bits())
    }
}
