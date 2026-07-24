//! A fused blake2s-256 Merkle path hasher over the `blake2_with_compression`
//! delegation primitive.
//!
//! [`Blake2RoundFunctionEvaluator::compress_node`] is a fused two-to-one node
//! compression: it keeps the running hash inside the evaluator across folds, so
//! each level marshals only the 32-byte sibling and issues one delegated
//! compression (no per-level hasher init or finalize). This module wraps that
//! primitive — and its `unsafe` buffer contracts — behind a safe API.
//!
//! Folding is byte-identical to plain digest chaining:
//!
//! ```text
//! fold(h, sibling, on_left) == blake2s256(on_left ? sibling || h : h || sibling)
//! ```
//!
//! (`compress_node::<false>` starts each compression from the blake2s IV with
//! `t = 64` and the final-block flag set — a fresh single-block hash.)

use blake2s_u32::state_with_extended_control::Blake2RoundFunctionEvaluator;
use blake2s_u32::{
    AlignedArray64, BLAKE2S_BLOCK_SIZE_BYTES, BLAKE2S_BLOCK_SIZE_U32_WORDS,
    BLAKE2S_EXTENDED_STATE_WIDTH_IN_U32_WORDS, BLAKE2S_STATE_WIDTH_IN_U32_WORDS,
};

/// [`Blake2RoundFunctionEvaluator::new`] is sound only on RISC-V: it relies on
/// zero-initialized memory via `assume_init`, which is UB on native hosts. This
/// reproduces the same value by zeroing every (public) field explicitly, then
/// calling `reset()` just as `new()` does.
fn zeroed_evaluator() -> Blake2RoundFunctionEvaluator {
    let mut evaluator = Blake2RoundFunctionEvaluator {
        state: [0u32; BLAKE2S_STATE_WIDTH_IN_U32_WORDS],
        extended_state: [0u32; BLAKE2S_EXTENDED_STATE_WIDTH_IN_U32_WORDS],
        input_buffer: AlignedArray64::from_value(0u32),
        t: 0,
    };
    evaluator.reset();
    evaluator
}

/// A running blake2s-256 hash folded along a Merkle path.
///
/// Seed it with [`from_single_block`](Self::from_single_block) (the leaf hash),
/// then [`fold`](Self::fold) one tree level at a time; [`finalize`](Self::finalize)
/// reads the current running hash (typically compared against a root).
#[derive(Clone, Debug)]
pub struct Blake2sPathHasher {
    evaluator: Blake2RoundFunctionEvaluator,
}

impl Blake2sPathHasher {
    /// Starts the running hash at `blake2s256(bytes)` for a single ≤ 64-byte
    /// input block (zero-padded internally).
    ///
    /// # Panics
    ///
    /// Panics if `bytes` is longer than one blake2s block (64 bytes).
    pub fn from_single_block(bytes: &[u8]) -> Self {
        assert!(
            bytes.len() <= BLAKE2S_BLOCK_SIZE_BYTES,
            "single-block input must be <= {BLAKE2S_BLOCK_SIZE_BYTES} bytes, got {}",
            bytes.len(),
        );
        let mut evaluator = zeroed_evaluator();

        let mut block = [0u8; BLAKE2S_BLOCK_SIZE_BYTES];
        block[..bytes.len()].copy_from_slice(bytes);
        let buffer = evaluator.get_witness_buffer();
        for (word, chunk) in buffer.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_le_bytes(chunk.try_into().expect("chunk is 4 bytes"));
        }

        // SAFETY: the full input buffer is initialized (zero-padded) above, and
        // the byte length matches the data written, as the evaluator requires.
        unsafe {
            evaluator.run_round_function_with_byte_len::<false>(bytes.len(), true);
        }
        Self { evaluator }
    }

    /// Replaces the running hash `h` with `blake2s256(sibling || h)` when
    /// `sibling_on_left`, or `blake2s256(h || sibling)` otherwise — one Merkle
    /// tree level, as a single fused delegated compression.
    pub fn fold(&mut self, sibling: &[u8; 32], sibling_on_left: bool) {
        let buffer = self.evaluator.get_witness_buffer();
        for (word, chunk) in buffer[..BLAKE2S_BLOCK_SIZE_U32_WORDS / 2]
            .iter_mut()
            .zip(sibling.chunks_exact(4))
        {
            *word = u32::from_le_bytes(chunk.try_into().expect("chunk is 4 bytes"));
        }
        // `is_right` = the *running* hash is the right child = sibling on the left.
        self.evaluator.compress_node::<false>(sibling_on_left);
    }

    /// Returns the current running hash.
    pub fn finalize(&self) -> [u8; 32] {
        let words = self.evaluator.read_state_for_output();
        let mut out = [0u8; 32];
        for (chunk, word) in out.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blake2_ext::{Blake2s256 as ReferenceBlake2s, Digest};
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};

    fn reference_hash(bytes: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(&ReferenceBlake2s::digest(bytes));
        out
    }

    /// Deterministic pseudo-random test bytes (seeded ChaCha8).
    fn pseudo_random_bytes(seed: u64, len: usize) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut bytes = vec![0u8; len];
        rng.fill_bytes(&mut bytes);
        bytes
    }

    #[test]
    fn single_block_matches_reference() {
        for len in [0usize, 1, 31, 32, 40, 63, 64] {
            for seed in [1u64, 42, 0xdead_beef] {
                let bytes = pseudo_random_bytes(seed ^ len as u64, len);
                assert_eq!(
                    Blake2sPathHasher::from_single_block(&bytes).finalize(),
                    reference_hash(&bytes),
                    "single block mismatch at len {len} seed {seed}",
                );
            }
        }
        // Edge byte patterns.
        for byte in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
            let bytes = [byte; 40];
            assert_eq!(
                Blake2sPathHasher::from_single_block(&bytes).finalize(),
                reference_hash(&bytes),
            );
        }
    }

    #[test]
    #[should_panic(expected = "single-block input must be <= 64 bytes")]
    fn rejects_oversized_block() {
        Blake2sPathHasher::from_single_block(&[0u8; 65]);
    }

    #[test]
    fn fold_matches_reference_chain() {
        // A full 256-level chain exercising both sibling orders, checked at
        // every step against the reference `blake2s(left || right)` chain.
        let leaf = pseudo_random_bytes(7, 40);
        let mut hasher = Blake2sPathHasher::from_single_block(&leaf);
        let mut running = reference_hash(&leaf);

        for depth in 0..256usize {
            let mut sibling = [0u8; 32];
            sibling.copy_from_slice(&pseudo_random_bytes(depth as u64 + 1000, 32));
            let sibling_on_left = depth % 3 == 0; // mixed direction pattern

            hasher.fold(&sibling, sibling_on_left);
            let mut concat = [0u8; 64];
            if sibling_on_left {
                concat[..32].copy_from_slice(&sibling);
                concat[32..].copy_from_slice(&running);
            } else {
                concat[..32].copy_from_slice(&running);
                concat[32..].copy_from_slice(&sibling);
            }
            running = reference_hash(&concat);

            assert_eq!(
                hasher.finalize(),
                running,
                "fold mismatch at depth {depth} (sibling_on_left = {sibling_on_left})",
            );
        }
    }

    #[test]
    fn fold_single_level_both_orders() {
        let leaf = [0xABu8; 32];
        let sibling = [0xCDu8; 32];
        for sibling_on_left in [false, true] {
            let mut hasher = Blake2sPathHasher::from_single_block(&leaf);
            hasher.fold(&sibling, sibling_on_left);
            let mut concat = [0u8; 64];
            let leaf_hash = reference_hash(&leaf);
            if sibling_on_left {
                concat[..32].copy_from_slice(&sibling);
                concat[32..].copy_from_slice(&leaf_hash);
            } else {
                concat[..32].copy_from_slice(&leaf_hash);
                concat[32..].copy_from_slice(&sibling);
            }
            assert_eq!(hasher.finalize(), reference_hash(&concat));
        }
    }
}
