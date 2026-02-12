# Using Crypto on Guest and Host

Airbender provides a shared crypto crate, `airbender-crypto`, designed for both host and guest use.

## Add Dependency

Base dependency (works on host and guest):

```toml
[dependencies]
airbender-crypto = { path = "../../crates/airbender-crypto" }
```

For guest builds where you want delegated backends enabled, use:

```toml
[dependencies]
airbender-crypto = { path = "../../crates/airbender-crypto", features = ["proving"] }
```

The `proving` feature turns on delegation-related features (`bigint_ops`, `keccak_special5`, `single_round_with_control`).

## What the Crate Exposes

Common entry points include:

- Hashing: `sha256`, `sha3`, `ripemd160`, `blake2s`
- Curve/tooling re-exports: `k256`, `p256`
- Airbender-specific secp helpers: `secp256k1`, `secp256r1`
- Pairing/field modules: `bn254`, `bls12_381`

## Example: Shared Hashing Code (Delegation-Aware)

```rust
use airbender_crypto::sha3::Keccak256;
use airbender_crypto::MiniDigest;

pub fn hash32(data: &[u8]) -> [u8; 32] {
    Keccak256::digest(data)
}
```

`sha3::Keccak256` can use a delegated guest backend when the right guest features are enabled. (Unlike `sha256`, which currently has no delegated path.)

## Why Delegation Matters on RISC-V Guests

On Airbender guest targets (`riscv32`), delegated crypto paths are important because they move expensive low-level arithmetic/permutation work to VM-specific delegation backends.

In practice this means:

- Lower guest execution cost for heavy crypto operations.
- Better proving efficiency for crypto-intensive guest logic.
- Stable API surface: your host and guest Rust code can stay the same while backend selection changes by target/features.

## Modules With Delegation Implemented

On `riscv32` guests (typically with `features = ["proving"]`), delegated implementations exist for:

- `sha3` (`Keccak256`) via `keccak_special5`.
- `blake2s` (`Blake2s256`) via `single_round_with_control`.
- `secp256k1` field/scalar arithmetic via `bigint_ops`.
- `secp256r1` field/scalar arithmetic via `bigint_ops`.
- `bn254` base-field/extension-field and dependent curve arithmetic (with delegated `Fq`; `Fr` remains the arkworks implementation).
- `bls12_381` field and dependent curve arithmetic (`Fq`, `Fr`, and extension fields).

## Modules Without Delegation Today

- `sha256`
- `ripemd160`
- `k256`
- `p256`

## Practical Guidance

- Start with shared, deterministic crypto utilities that run on both host and guest.
- Validate behavior with host tests first, then run guest execution/proof flows.
- For complete secp-focused usage, see crate tests:
  - [`crates/airbender-crypto/tests/secp256k1.rs`](../crates/airbender-crypto/tests/secp256k1.rs)
  - [`crates/airbender-crypto/tests/secp256r1.rs`](../crates/airbender-crypto/tests/secp256r1.rs)
