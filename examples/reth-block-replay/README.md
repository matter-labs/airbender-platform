# Reth Block Replay

Fetches a block and its `ExecutionWitness` from a [reth](https://github.com/paradigmxyz/reth)
node, re-executes the block inside an Airbender guest using
[paradigmxyz/stateless](https://github.com/paradigmxyz/stateless), and generates a ZK proof
that the execution is correct.

The guest verifies gas usage, receipts root, logs bloom, requests hash (when present), and state
root against the block header. The committed output is a Keccak-256 digest over that checked
correctness summary, exposed as the guest's `[u32; 8]` public output.

## Prerequisites

- Docker (to run a reth dev node)
- [Foundry](https://getfoundry.sh/) (`cast` CLI for sending transactions)

## Build and run

Start reth in dev mode and generate a block that exercises `ecrecover`, `sha256`, `ripemd160`,
`bn256Add`, and `bn256ScalarMul`:

```sh
python3 examples/reth-block-replay/docker/generate-blocks.py
```

The setup script prints a final `BLOCK_NUM=...` line so you can reuse the generated block number
directly.

Build the guest and run the host:

```sh
cargo airbender build --project examples/reth-block-replay/guest

cargo run --release --manifest-path examples/reth-block-replay/host/Cargo.toml -- --block-num 2
cargo run --release --manifest-path examples/reth-block-replay/host/Cargo.toml -- --block-num 2 --prove
```

`--rpc-url` defaults to `http://localhost:8545`, so you only need to pass it when replaying from a
different endpoint.

Expected output (with `--prove`):

```
Connecting to http://localhost:8545, fetching block 2...
Block 2: 1 transactions, gas_used=46189
Witness: <n> state nodes, 2 codes, 3 keys, 1 headers
Recovered 1 public keys
Expected block hash: 0x...
Expected public commitment: 0x...
Simulation verified: correctness commitment matches.
Proof verified: block 2 (hash=0x...) proven in ZK.
```

To stop the reth node:

```sh
docker compose -f examples/reth-block-replay/docker/docker-compose.yml down
```

## How it works

### Host (`host/src/main.rs`)

1. Parses `--rpc-url`, `--block-num`, and `--prove` with `clap`.
2. Connects to reth via HTTP JSON-RPC.
3. Fetches the raw block (`debug_getRawBlock`) and execution witness (`debug_executionWitness`).
4. Recovers transaction signer public keys from ECDSA signatures.
5. Resolves the chain config for the replayed block. On `reth --dev`, this uses the built-in
   dev chain config because `debug_chainConfig` only returns a partial config there.
6. Passes four inputs to the guest: block RLP bytes, witness, chain config (as JSON), and public keys.
7. Builds the expected public commitment from the block header and checks that the guest returns it.
8. Runs the guest in simulation, then optionally proves and verifies.

### Guest (`guest/src/main.rs`)

1. Installs a custom `revm_precompile::Crypto` backend via `revm_precompile::install_crypto()`.
2. Reads the four inputs from the host.
3. RLP-decodes the block and JSON-deserializes the chain config.
4. Builds a `ChainSpec` and `EthEvmConfig` from the chain config.
5. Calls `stateless_validation()` which:
   - Recovers signers from the provided public keys.
   - Validates ancestor headers from the witness.
   - Builds a trie from witness data and verifies it against the parent state root.
   - Re-executes all transactions via revm (using the installed `Crypto` backend).
   - Validates gas_used, receipts_root, logs_bloom, requests hash, and state_root against the block header.
6. Commits a shared `ReplayCommitment` digest as `[u32; 8]` to the proof output.

### Shared commitment (`shared/src/lib.rs`)

The example uses a dedicated shared crate for proof outputs. `ReplayCommitment` captures the
header fields that were checked by the replay and hashes them into a `B256`, while
`CommittableB256` handles the guest register layout for committing that digest.

### Custom crypto hooks (`guest/src/airbender_crypto.rs`)

The guest installs a custom `revm_precompile::Crypto` backend backed by `airbender::crypto`.
That keeps revm and `stateless` on the Airbender crypto stack for the precompile hooks used by
this example without changing the EVM configuration surface.

This example backend implements `sha256`, `ripemd160`, `secp256k1_ecrecover`,
`secp256r1_verify_signature`, `bn254_g1_add`, `bn254_g1_mul`, and `bn254_pairing_check`.
`modexp` and `blake2f` still use revm's default implementations because Airbender does not expose
matching primitives for them today.

## Serialization notes

Two types require special handling because their serde implementations are incompatible
with Airbender's bincode-based codec:

- **`Block`**: Uses RLP encoding (passed as `Vec<u8>`) because `Block`'s serde has bincode
  compatibility issues with tagged enums.
- **`ChainConfig`**: Uses JSON encoding (passed as `Vec<u8>`) because `ChainConfig` uses
  `#[serde(flatten)]` on its `extra_fields`, which bincode cannot handle.

All other types (`ExecutionWitness`, `Vec<UncompressedPublicKey>`) serialize directly through
the standard Airbender codec.

## The mulh patch

The Airbender prover uses `FullUnsignedMachineDecoderConfig` which does not support signed
multiply/divide instructions (`mulh`, `div`, `rem`). LLVM emits `mulh` when it optimizes
certain integer division patterns on rv32.

This example uses the same temporary
[`Jrigada/revm`](https://github.com/Jrigada/revm/tree/fix/memory-gas-inline-never) fork as
`examples/revm-basic`. The fork carries the `#[inline(never)]` fix on `memory_gas`, which prevents
LLVM from combining the memory gas math into a `mulh`-producing pattern.

The guest is compiled with `opt-level = "s"`, LTO, and `codegen-units = 1` to fit within
the 4MB ROM limit (the crypto precomputed tables add significant `.rodata`). Note that
`opt-level = "z"` cannot be used — it causes LLVM to emit signed `div` instructions which
the unsigned-only prover config rejects at runtime.
