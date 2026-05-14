use riscv_transpiler::ir::{
    preprocess_bytecode, DecodingOptions, FullUnsignedMachineDecoderConfig, Instruction,
    ReducedMachineDecoderConfig,
};

/// Airbender Platform machine profiles with stable host-side semantics.
///
/// The upstream Airbender crates model machines as Rust types, which is useful
/// internally but brittle as a public platform API. This enum is the small set
/// of machine configurations that `airbender-host` is willing to name and keep
/// stable for normal users.
///
/// Full-signed decoding is intentionally omitted from the stable profile set for
/// now: upstream decoding can name signed multiplication/division instructions,
/// but the host runner does not execute those instruction variants.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum MachineProfile {
    /// Full machine without signed multiplication/division support.
    #[default]
    FullUnsigned,
    /// Reduced machine used by recursive verifier workloads.
    Reduced,
}

type PreprocessBytecodeFn = fn(&[u32]) -> Vec<Instruction>;

#[derive(Clone, Copy)]
pub(crate) struct TranspilerDecoderConfig {
    name: &'static str,
    preprocess_bytecode: PreprocessBytecodeFn,
    pub(crate) stable_profile: Option<MachineProfile>,
}

impl TranspilerDecoderConfig {
    pub(crate) fn from_profile(profile: MachineProfile) -> Self {
        match profile {
            MachineProfile::FullUnsigned => Self {
                name: "full unsigned",
                preprocess_bytecode: preprocess_bytecode::<FullUnsignedMachineDecoderConfig>,
                stable_profile: Some(profile),
            },
            MachineProfile::Reduced => Self {
                name: "reduced",
                preprocess_bytecode: preprocess_bytecode::<ReducedMachineDecoderConfig>,
                stable_profile: Some(profile),
            },
        }
    }

    pub(crate) fn unstable_raw<D>(name: &'static str) -> Self
    where
        D: DecodingOptions,
    {
        Self {
            name,
            preprocess_bytecode: preprocess_bytecode::<D>,
            stable_profile: None,
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn preprocess(&self, bytecode: &[u32]) -> Vec<Instruction> {
        (self.preprocess_bytecode)(bytecode)
    }
}

impl Default for TranspilerDecoderConfig {
    fn default() -> Self {
        Self::from_profile(MachineProfile::default())
    }
}

impl std::fmt::Debug for TranspilerDecoderConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TranspilerDecoderConfig")
            .field("name", &self.name)
            .field("stable_profile", &self.stable_profile)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{MachineProfile, TranspilerDecoderConfig};
    use riscv_transpiler::ir::{
        DebugReducedMachineDecoderConfig, FullMachineDecoderConfig, InstructionName,
    };

    const DIV_X3_X1_X2: u32 = 0x0220c1b3;
    const LBU_X1_FROM_X0: u32 = 0x00004083;

    #[test]
    fn raw_decoder_can_opt_into_full_signed_decoding() {
        let full_signed =
            TranspilerDecoderConfig::unstable_raw::<FullMachineDecoderConfig>("test full signed")
                .preprocess(&[DIV_X3_X1_X2]);
        let full_unsigned = TranspilerDecoderConfig::from_profile(MachineProfile::FullUnsigned)
            .preprocess(&[DIV_X3_X1_X2]);

        assert_eq!(full_signed[0].name, InstructionName::Div);
        assert_eq!(full_unsigned[0].name, InstructionName::Illegal);
    }

    #[test]
    fn profile_controls_subword_memory_decoding() {
        let full_unsigned = TranspilerDecoderConfig::from_profile(MachineProfile::FullUnsigned)
            .preprocess(&[LBU_X1_FROM_X0]);
        let reduced = TranspilerDecoderConfig::from_profile(MachineProfile::Reduced)
            .preprocess(&[LBU_X1_FROM_X0]);

        assert_eq!(full_unsigned[0].name, InstructionName::Lbu);
        assert_eq!(reduced[0].name, InstructionName::Illegal);
    }

    #[test]
    fn raw_decoder_accepts_upstream_decoder_configurations() {
        let instructions =
            TranspilerDecoderConfig::unstable_raw::<DebugReducedMachineDecoderConfig>(
                "debug reduced",
            )
            .preprocess(&[LBU_X1_FROM_X0]);

        assert_eq!(instructions[0].name, InstructionName::Lbu);
    }
}
