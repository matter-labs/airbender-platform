# Reth Block Replay

Fetches a block and its `ExecutionWitness` from a [reth](https://github.com/paradigmxyz/reth)
node, re-executes the block inside an Airbender guest using
[paradigmxyz/stateless](https://github.com/paradigmxyz/stateless), and generates a ZK proof
that the execution is correct.

The guest verifies gas usage, receipts root, and logs bloom against the block header.
State root verification is deferred (v1 treats `PostStateRootMismatch` as non-fatal).
The committed output is the block hash (`[u32; 8]`, 32 bytes).

## Prerequisites

- Docker (to run a reth dev node)
- [Foundry](https://getfoundry.sh/) (`cast` CLI for sending transactions)

## Build and run

Start reth in dev mode and generate a block that exercises `ecrecover`, `bn256Add`, and
`bn256ScalarMul`:

```sh
bash examples/reth-block-replay/docker/generate-blocks.sh
```

The setup script prints a final `BLOCK_NUM=...` line so you can reuse the generated block number
directly.

Build the guest and run the host:

```sh
cargo airbender build --project examples/reth-block-replay/guest

BLOCK_NUM=2 cargo run --release --manifest-path examples/reth-block-replay/host/Cargo.toml             # simulate only
BLOCK_NUM=2 cargo run --release --manifest-path examples/reth-block-replay/host/Cargo.toml -- --prove  # simulate + prove + verify (~35 min)
```

Expected output (with `--prove`):

```
Connecting to http://localhost:8545, fetching block 2...
Block 2: 1 transactions, gas_used=40604
Witness: <n> state nodes, 2 codes, 3 keys, 1 headers
Recovered 1 public keys
Expected block hash: 0x...
Simulation verified: block hash matches.
Proof verified: block 2 (hash=0x...) proven in ZK.
```

To stop the reth node:

```sh
docker compose -f examples/reth-block-replay/docker/docker-compose.yml down
```

## How it works

### Host (`host/src/main.rs`)

1. Connects to reth via HTTP JSON-RPC.
2. Fetches the raw block (`debug_getRawBlock`) and execution witness
   (`debug_executionWitness`, with a `debug_getExecutionWitness` fallback for compatibility).
3. Recovers transaction signer public keys from ECDSA signatures.
4. Resolves the chain config for the replayed block. On `reth --dev`, this uses the built-in
   dev chain config because `debug_chainConfig` only returns a partial config there.
5. Passes four inputs to the guest: block RLP bytes, witness, chain config (as JSON), and public keys.
6. Runs the guest in simulation, then optionally proves and verifies.

### Guest (`guest/src/main.rs`)

1. Installs Airbender's accelerated crypto provider via `revm_precompile::install_crypto()`.
2. Reads the four inputs from the host.
3. RLP-decodes the block and JSON-deserializes the chain config.
4. Builds a `ChainSpec` and `EthEvmConfig` from the chain config.
5. Calls `stateless_validation()` which:
   - Recovers signers from the provided public keys.
   - Validates ancestor headers from the witness.
   - Builds a trie from witness data and verifies it against the parent state root.
   - Re-executes all transactions via revm (using Airbender precompiles).
   - Validates gas_used, receipts_root, and logs_bloom against the block header.
6. Commits the block hash as `[u32; 8]` to the proof output.

### Custom precompiles (`guest/src/airbender_crypto.rs`)

The guest overrides revm's default cryptographic backends with Airbender-accelerated
implementations using `revm_precompile::install_crypto()`. This is the same pattern used
by RISC Zero Zeth (`R0vmCrypto`). The override is transparent to paradigmxyz/stateless —
no EVM config changes are needed.

Overridden precompiles:

- **ecrecover** (0x01): Airbender's optimized secp256k1 ecmult with precomputed generator
  tables and endomorphism decomposition, plus accelerated keccak256 for address hashing.
- **bn256Add** (0x06): BN254 G1 point addition via Airbender's delegated bigint field arithmetic.
- **bn256ScalarMul** (0x07): BN254 G1 scalar multiplication.
- **bn256Pairing** (0x08): BN254 pairing check (Miller loop + final exponentiation).

All other precompiles (sha256, ripemd160, modexp, blake2f, ecPairing for secp256r1, KZG,
BLS12-381) use revm's default software implementations.

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

This example vendors `revm-interpreter` 32.0.0 under `guest/vendor/revm-interpreter` and adds
`#[inline(never)]` to `memory_gas` (`src/gas.rs`). That prevents LLVM from combining the memory
gas math into a `mulh`-producing pattern while keeping the example self-contained and reproducible.

The guest is compiled with `opt-level = "s"`, LTO, and `codegen-units = 1` to fit within
the 4MB ROM limit (the crypto precomputed tables add significant `.rodata`). Note that
`opt-level = "z"` cannot be used — it causes LLVM to emit signed `div` instructions which
the unsigned-only prover config rejects at runtime.
