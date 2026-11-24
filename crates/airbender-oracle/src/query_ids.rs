//! The query IDs of the oracle: a guest starts every query with one (see [`MemoryOracle`]).
//!
//! The IDs are split into subspaces by bitmask, and this module allocates them all, so that the
//! queries of the platform and of a guest program never collide:
//! - the top bit is reserved;
//! - the second bit marks the basic queries, whose second byte selects the subspace;
//! - the platform owns the system and advice subspaces, and every ID in them is defined here;
//! - the other subspaces belong to ZKsync OS, which assigns the IDs inside them.
//!
//! The IDs are part of the wire protocol: a recorded tape answers the IDs it was recorded for.
//!
//! [`MemoryOracle`]: crate::MemoryOracle

/// Top bit, reserved
pub const RESERVED_SUBSPACE_MASK: u32 = 0x80_00_00_00;
/// Second bit, the basic queries
pub const BASIC_SUBSPACE_MASK: u32 = 0x40_00_00_00;

/// System queries, owned by the platform (e.g. [`DISCONNECT_ORACLE_QUERY_ID`])
#[allow(clippy::identity_op)]
pub const SYSTEM_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_00_00_00; // 0x40000000
/// Generic queries, owned by ZKsync OS
pub const GENERIC_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_01_00_00; // 0x40010000
/// Preimage and hash queries, owned by ZKsync OS
pub const PREIMAGE_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_02_00_00; // 0x40020000
/// Account state and storage queries, owned by ZKsync OS
pub const ACCOUNT_AND_STORAGE_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_03_00_00; // 0x40030000
/// State root and Merkle path queries, owned by ZKsync OS
pub const STATE_AND_MERKLE_PATHS_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_04_00_00; // 0x40040000
/// Computational advice (hints checked by the querier), owned by the platform
pub const ADVICE_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_05_00_00; // 0x40050000
/// Transaction queries, owned by ZKsync OS
pub const TRANSACTION_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_06_00_00; // 0x40060000
/// Block (and batch) queries, owned by ZKsync OS
pub const BLOCK_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_07_00_00; // 0x40070000

/// UART output, for debugging
pub const UART_QUERY_ID: u32 = 0xff_ff_ff_ff;

// ========== System subspace ==========

/// Disconnects the guest from the external oracle: execution continues on its own
#[allow(clippy::identity_op)]
pub const DISCONNECT_ORACLE_QUERY_ID: u32 = SYSTEM_SUBSPACE_MASK | 0; // 0x40000000

// ========== Advice subspace ==========

/// ModExp: the quotient and remainder of a multi-digit reduction (`airbender-crypto`'s `modexp`)
pub const MODEXP_ADVICE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x10; // 0x40050010

/// Field hints: inverses, square roots, quotients and pairing witnesses (`airbender-crypto`'s
/// `hints::FieldHintOp`)
pub const FIELD_OPS_ADVISE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x11; // 0x40050011

/// The KZG commitment and proof of a blob (ZKsync OS's blob commitment generator)
pub const BLOB_COMMITMENT_AND_PROOF_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x20; // 0x40050020

/// The quotient and remainder of two 256-bit integers. The guest sends the addresses of the
/// dividend and the divisor, and checks `q * d + r == n` and `r < d`.
pub const U256_DIV_REM_ADVICE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x30; // 0x40050030

/// The quotient of a 512-bit dividend by a 256-bit divisor. The guest sends the addresses of the
/// dividend halves and the divisor, derives `r = n - q * d` and checks `0 <= r < d`.
pub const U256_WIDE_DIV_REM_ADVICE_QUERY_ID: u32 = ADVICE_SUBSPACE_MASK | 0x31; // 0x40050031

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ids_are_the_wire_protocol() {
        assert_eq!(SYSTEM_SUBSPACE_MASK, 0x4000_0000);
        assert_eq!(ADVICE_SUBSPACE_MASK, 0x4005_0000);
        assert_eq!(BLOCK_SUBSPACE_MASK, 0x4007_0000);
        assert_eq!(DISCONNECT_ORACLE_QUERY_ID, 0x4000_0000);
        assert_eq!(MODEXP_ADVICE_QUERY_ID, 0x4005_0010);
        assert_eq!(FIELD_OPS_ADVISE_QUERY_ID, 0x4005_0011);
        assert_eq!(BLOB_COMMITMENT_AND_PROOF_QUERY_ID, 0x4005_0020);
        assert_eq!(U256_DIV_REM_ADVICE_QUERY_ID, 0x4005_0030);
        assert_eq!(U256_WIDE_DIV_REM_ADVICE_QUERY_ID, 0x4005_0031);
        assert_eq!(UART_QUERY_ID, u32::MAX);
    }
}
