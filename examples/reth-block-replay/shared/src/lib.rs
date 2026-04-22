#![cfg_attr(not(test), no_std)]

extern crate alloc;

use airbender_core::guest::Commit;
use alloc::vec::Vec;
use alloy_consensus::BlockHeader;
use alloy_primitives::{keccak256, Bloom, B256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommittableB256(pub B256);

impl CommittableB256 {
    pub fn public_output_words(&self) -> [u32; 8] {
        let bytes = self.0.as_slice();
        let mut out = [0u32; 8];

        for (i, chunk) in bytes.chunks_exact(4).enumerate() {
            out[i] = u32::from_le_bytes(chunk.try_into().unwrap());
        }

        out
    }
}

impl Commit for CommittableB256 {
    fn commit_words(&self) -> [u32; 8] {
        self.public_output_words()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayCommitment {
    pub block_hash: B256,
    pub state_root: B256,
    pub receipts_root: B256,
    pub logs_bloom: Bloom,
    pub gas_used: u64,
    pub requests_hash: Option<B256>,
}

impl ReplayCommitment {
    pub fn from_header<H: BlockHeader>(block_hash: B256, header: &H) -> Self {
        Self {
            block_hash,
            state_root: header.state_root(),
            receipts_root: header.receipts_root(),
            logs_bloom: header.logs_bloom(),
            gas_used: header.gas_used(),
            requests_hash: header.requests_hash(),
        }
    }

    pub fn digest(&self) -> B256 {
        let mut encoded = Vec::with_capacity(393);
        encoded.extend_from_slice(self.block_hash.as_slice());
        encoded.extend_from_slice(self.state_root.as_slice());
        encoded.extend_from_slice(self.receipts_root.as_slice());
        encoded.extend_from_slice(self.logs_bloom.data());
        encoded.extend_from_slice(&self.gas_used.to_le_bytes());

        match self.requests_hash {
            Some(hash) => {
                encoded.push(1);
                encoded.extend_from_slice(hash.as_slice());
            }
            None => encoded.push(0),
        }

        keccak256(encoded)
    }

    pub fn public_output_words(&self) -> [u32; 8] {
        CommittableB256(self.digest()).public_output_words()
    }
}

impl Commit for ReplayCommitment {
    fn commit_words(&self) -> [u32; 8] {
        self.public_output_words()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committable_b256_matches_register_layout() {
        let digest = CommittableB256(B256::from([
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa,
            0x99, 0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ]));

        let words = digest.public_output_words();
        assert_eq!(words[0], 0x5566_7788);
        assert_eq!(words[1], 0x1122_3344);
        assert_eq!(words[2], 0xccdd_eeff);
        assert_eq!(words[7], 0x0807_0605);
    }

    #[test]
    fn replay_commitment_digest_changes_with_checked_fields() {
        let baseline = ReplayCommitment {
            block_hash: B256::repeat_byte(0x11),
            state_root: B256::repeat_byte(0x22),
            receipts_root: B256::repeat_byte(0x33),
            logs_bloom: Bloom::from([0x44; 256]),
            gas_used: 45,
            requests_hash: None,
        };

        let mut changed = baseline;
        changed.state_root = B256::repeat_byte(0x55);

        assert_ne!(baseline.digest(), changed.digest());
        assert_ne!(
            baseline.public_output_words(),
            changed.public_output_words()
        );
    }
}
