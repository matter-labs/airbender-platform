# Using Crypto on Guest and Host

`airbender-crypto` provides cryptographic primitives that work on both host and guest. On the guest, supported primitives automatically use prover-accelerated delegation backends. Same API, dramatically lower proving cost.

## Add Dependency

Standalone:

```toml
[dependencies]
airbender-crypto = { path = "../../crates/airbender-crypto" }
```

For guest builds with delegation enabled:

```toml
[dependencies]
airbender-crypto = { path = "../../crates/airbender-crypto", features = ["proving"] }
```

Or via the SDK re-export (always enables delegation):

```toml
[dependencies]
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", features = ["crypto"] }
```

Then import from `airbender::crypto`.

## Available Primitives

- **Hashing:** `sha256`, `sha3` (Keccak256), `ripemd160`, `blake2s`
- **Curves:** `k256`, `p256` (re-exports), `secp256k1`, `secp256r1` (Airbender-specific helpers)
- **Pairing/field:** `bn254`, `bls12_381`

## Example

```rust
use airbender_crypto::sha3::Keccak256;
use airbender_crypto::MiniDigest;

pub fn hash32(data: &[u8]) -> [u8; 32] {
    Keccak256::digest(data)
}
```

`MiniDigest` is a simplified digest trait that returns a fixed `[u8; 32]`. This code works identically on host and guest. On the guest with `proving` enabled, `Keccak256` routes through a delegated backend that's dramatically cheaper to prove.

## Why Delegation Matters

On RISC-V guests, crypto operations are expensive to prove because every instruction becomes part of the execution trace. Delegated backends move heavy arithmetic to VM-specific circuits that the prover handles natively.

In practice:

- **Lower proving cost** for crypto-heavy guest logic
- **Same code** on host and guest. Backend selection happens via target and features.
- **Transparent.** You don't need to change your API calls.

Prefer delegated primitives when they're available. They can be orders of magnitude cheaper on the guest.

## Delegation Status

**Delegated** (use these when possible):

- `sha3` (Keccak256) - via `keccak_special5`
- `blake2s` (Blake2s256) - via `single_round_with_control`
- `secp256k1` field/scalar - via `bigint_ops`
- `secp256r1` field/scalar - via `bigint_ops`
- `bn254` base/extension field and curve arithmetic (delegated `Fq`; `Fr` uses arkworks)
- `bls12_381` field and curve arithmetic (`Fq`, `Fr`, and extension fields)

**Also available, without delegation** (software-only, higher proving cost):

- `sha256`
- `ripemd160`
- `k256`
- `p256`

## Practical Tips

- Write shared crypto code that runs on both host and guest. Test on the host first (faster iteration), then run guest execution/proof flows.
- For secp usage examples, see the crate tests:
  - [`tests/secp256k1.rs`](https://github.com/matter-labs/airbender-platform/blob/main/crates/airbender-crypto/tests/secp256k1.rs)
  - [`tests/secp256r1.rs`](https://github.com/matter-labs/airbender-platform/blob/main/crates/airbender-crypto/tests/secp256r1.rs)
