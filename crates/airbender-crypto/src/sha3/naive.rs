use sha3::Digest;
pub use sha3::Keccak256;

impl crate::MiniDigest for Keccak256 {
    type HashOutput = crate::Bytes32;

    #[inline(always)]
    fn new() -> Self {
        <Keccak256 as Digest>::new()
    }

    // #[inline(always)]
    #[inline(never)]
    fn digest(input: impl AsRef<[u8]>) -> Self::HashOutput {
        let mut hasher = <Keccak256 as Digest>::new();
        <Keccak256 as Digest>::update(&mut hasher, input);
        let digest = <Keccak256 as Digest>::finalize(hasher);
        let mut result = crate::Bytes32::ZERO;
        #[allow(deprecated)] // TODO: to be fixed in `zksync-os/crypto` first
        result.0.copy_from_slice(digest.as_ref());
        result
    }

    #[inline(always)]
    fn update(&mut self, input: impl AsRef<[u8]>) {
        <Keccak256 as Digest>::update(self, input);
    }

    #[inline(always)]
    fn finalize(self) -> Self::HashOutput {
        crate::Bytes32(<Keccak256 as Digest>::finalize(self).into())
    }

    #[inline(always)]
    fn finalize_reset(&mut self) -> Self::HashOutput {
        crate::Bytes32(<Keccak256 as Digest>::finalize_reset(self).into())
    }
}
