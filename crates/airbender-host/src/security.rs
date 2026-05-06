/// Cryptographic security target for real Airbender proofs.
///
/// Airbender exposes 80-bit and 100-bit security as independent proving modes.
/// The host SDK keeps that choice explicit in the proof and verification-key
/// envelopes so callers cannot accidentally verify a proof with artifacts built
/// for a different security target.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum SecurityLevel {
    /// Use the 80-bit security configuration.
    Bits80,
    /// Use the 100-bit security configuration.
    #[default]
    Bits100,
}

impl SecurityLevel {
    pub fn bits(self) -> u16 {
        match self {
            Self::Bits80 => 80,
            Self::Bits100 => 100,
        }
    }
}

impl From<SecurityLevel> for verifier_common::SecurityModel {
    fn from(security: SecurityLevel) -> Self {
        match security {
            SecurityLevel::Bits80 => Self::Security80,
            SecurityLevel::Bits100 => Self::Security100,
        }
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.bits())
    }
}
