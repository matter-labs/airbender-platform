//! The query IDs of the oracle: a guest starts every query with one (see [`MemoryOracle`]).
//!
//! The IDs are split into subspaces by bitmask, so that the queries of the platform and of a
//! guest program never collide:
//! - the top bit is reserved;
//! - the second bit marks the basic queries, whose second byte selects the subspace;
//! - the system subspace is Airbender's: its IDs, the basic and system masks and the UART marker
//!   are defined in `riscv_transpiler::oracle::query_ids`, and re-exported here with the
//!   `oracle_processor` feature (the host);
//! - the platform owns the advice subspace, and every ID in it is defined here;
//! - the other subspaces belong to ZKsync OS, which assigns the IDs inside them.
//!
//! The IDs are part of the wire protocol: a recorded tape answers the IDs it was recorded for.
//!
//! [`MemoryOracle`]: crate::MemoryOracle

#[cfg(feature = "oracle_processor")]
pub use riscv_transpiler::oracle::query_ids::{
    BASIC_SUBSPACE_MASK, DISCONNECT_ORACLE_QUERY_ID, SYSTEM_SUBSPACE_MASK, UART_QUERY_ID,
    ZK_MASKING_QUERY_ID,
};

/// Top bit, reserved
pub const RESERVED_SUBSPACE_MASK: u32 = 0x80_00_00_00;

// The subspaces of the basic queries (the second bit, Airbender's `BASIC_SUBSPACE_MASK`) other
// than the system one

/// Generic queries, owned by ZKsync OS
pub const GENERIC_SUBSPACE_MASK: u32 = 0x40_01_00_00;
/// Preimage and hash queries, owned by ZKsync OS
pub const PREIMAGE_SUBSPACE_MASK: u32 = 0x40_02_00_00;
/// Account state and storage queries, owned by ZKsync OS
pub const ACCOUNT_AND_STORAGE_SUBSPACE_MASK: u32 = 0x40_03_00_00;
/// State root and Merkle path queries, owned by ZKsync OS
pub const STATE_AND_MERKLE_PATHS_SUBSPACE_MASK: u32 = 0x40_04_00_00;
/// Computational advice (hints checked by the querier), owned by the platform
pub const ADVICE_SUBSPACE_MASK: u32 = 0x40_05_00_00;
/// Transaction queries, owned by ZKsync OS
pub const TRANSACTION_SUBSPACE_MASK: u32 = 0x40_06_00_00;
/// Block (and batch) queries, owned by ZKsync OS
pub const BLOCK_SUBSPACE_MASK: u32 = 0x40_07_00_00;

/// The subspaces above are those of Airbender's basic queries, and none is its system subspace.
#[cfg(feature = "oracle_processor")]
const _: () = {
    let subspaces = [
        GENERIC_SUBSPACE_MASK,
        PREIMAGE_SUBSPACE_MASK,
        ACCOUNT_AND_STORAGE_SUBSPACE_MASK,
        STATE_AND_MERKLE_PATHS_SUBSPACE_MASK,
        ADVICE_SUBSPACE_MASK,
        TRANSACTION_SUBSPACE_MASK,
        BLOCK_SUBSPACE_MASK,
    ];
    let mut i = 0;
    while i < subspaces.len() {
        // the second byte selects the subspace: 1 to 7 here, 0 is the system one
        assert!(subspaces[i] == BASIC_SUBSPACE_MASK | ((i as u32 + 1) << 16));
        assert!(subspaces[i] != SYSTEM_SUBSPACE_MASK);
        assert!(subspaces[i] & RESERVED_SUBSPACE_MASK == 0);
        i += 1;
    }
};

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
        assert_eq!(ADVICE_SUBSPACE_MASK, 0x4005_0000);
        assert_eq!(BLOCK_SUBSPACE_MASK, 0x4007_0000);
        assert_eq!(MODEXP_ADVICE_QUERY_ID, 0x4005_0010);
        assert_eq!(FIELD_OPS_ADVISE_QUERY_ID, 0x4005_0011);
        assert_eq!(BLOB_COMMITMENT_AND_PROOF_QUERY_ID, 0x4005_0020);
        assert_eq!(U256_DIV_REM_ADVICE_QUERY_ID, 0x4005_0030);
        assert_eq!(U256_WIDE_DIV_REM_ADVICE_QUERY_ID, 0x4005_0031);
    }

    /// The IDs that Airbender defines, as this crate re-exports them
    #[cfg(feature = "oracle_processor")]
    #[test]
    fn the_system_ids_are_the_wire_protocol() {
        assert_eq!(BASIC_SUBSPACE_MASK, 0x4000_0000);
        assert_eq!(SYSTEM_SUBSPACE_MASK, 0x4000_0000);
        assert_eq!(DISCONNECT_ORACLE_QUERY_ID, 0x4000_0000);
        assert_eq!(ZK_MASKING_QUERY_ID, 0x4000_0001);
        assert_eq!(UART_QUERY_ID, u32::MAX);
    }
}
