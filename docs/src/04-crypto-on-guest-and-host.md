# Using Crypto on Guest and Host

`airbender-crypto` provides cryptographic primitives that work on both host and guest. When using `airbender-sdk` with the `crypto` feature, delegation backends are enabled automatically. If you depend on `airbender-crypto` directly, you need to enable the `proving` feature for delegated implementations — otherwise primitives fall back to their naive software versions.

## Add Dependency

The recommended approach is through the SDK:

```toml
[dependencies]
airbender = { package = "airbender-sdk", features = ["std", "crypto"] }
```

For direct usage with more fine-grained control over features:

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

## Secp256k1 Hooks

EC recovery (`ecrecover`) involves three expensive operations: a field-element square root to decompress the public key, a field-element inversion to convert from Jacobian to affine coordinates, and a scalar inversion for the recovery formula. On a RISC-V guest these dominate the proving cost of every `ecrecover` call.

The `Secp256k1Hooks` trait lets you replace these three operations with your own implementation. The primary use case is **hint-and-verify**: the host precomputes the result and passes it as a hint, and the guest checks it with a single multiplication (`a * a⁻¹ == 1`) instead of recomputing from scratch. This is much cheaper to prove.

### Using custom hooks

Most callers should use the standard `secp256k1::recover` API, which computes everything directly.
To make use of hooks, implement `Secp256k1Hooks` and pass them to `secp256k1::recover_with_hooks`.

Each trait method receives a mutable reference to the value that needs the operation and should store the result in-place. For inversions, verify with `candidate * input == 1`. For square roots, verify with `candidate² == input`.

The `_with_hooks` variants are also available on the lower-level operations:

- `Affine::decompress_with_hooks` — point decompression (uses `fe_sqrt_and_assign`)
- `Jacobian::to_affine_with_hooks` — coordinate conversion (uses `fe_invert_and_assign`)
- `recover_with_context_and_hooks` — recovery with explicit precomputed context

Hook implementations must produce correct results. The recovery functions trust the hook output and do not re-verify it. The verification logic belongs in the hook implementation itself.

### Example

The [`examples/ecrecover-hooks/`](https://github.com/matter-labs/airbender-platform/tree/main/examples/ecrecover-hooks) example demonstrates the full hint-and-verify flow: the host precomputes hints with `CapturingHooks`, the guest verifies them cheaply with `PrecomputedHintHooks`.

## Practical Tips

- Write shared crypto code that runs on both host and guest. Test on the host first (faster iteration), then run guest execution/proof flows.
- For secp usage examples, see the crate tests:
  - [`tests/secp256k1.rs`](https://github.com/matter-labs/airbender-platform/blob/main/crates/airbender-crypto/tests/secp256k1.rs)
  - [`tests/secp256r1.rs`](https://github.com/matter-labs/airbender-platform/blob/main/crates/airbender-crypto/tests/secp256r1.rs)
