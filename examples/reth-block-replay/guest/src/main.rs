#![no_main]

mod airbender_crypto;

use airbender::guest::read;
use alloy_genesis::Genesis;
use alloy_rlp::Decodable;
use alloy_rpc_types_debug::ExecutionWitness;
use reth_block_replay_shared::ReplayCommitment;
use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::Block;
use reth_evm_ethereum::EthEvmConfig;
use stateless::{stateless_validation, UncompressedPublicKey};
use std::sync::Arc;

#[airbender::main]
fn main() -> ReplayCommitment {
    assert!(
        revm_precompile::install_crypto(airbender_crypto::AirbenderCrypto),
        "failed to install AirbenderCrypto — default crypto was already set"
    );

    let block_rlp: Vec<u8> = read().expect("failed to read block RLP");
    let witness: ExecutionWitness = read().expect("failed to read witness");
    let chain_config_json: Vec<u8> = read().expect("failed to read chain config JSON");
    let public_keys: Vec<UncompressedPublicKey> = read().expect("failed to read public keys");

    let chain_config: alloy_genesis::ChainConfig =
        serde_json::from_slice(&chain_config_json).expect("failed to parse chain config JSON");

    let block = Block::decode(&mut block_rlp.as_slice()).expect("failed to decode block RLP");

    let genesis = Genesis {
        config: chain_config,
        ..Default::default()
    };
    let chain_spec = Arc::new(ChainSpec::from(genesis));
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    let expected_block_hash = block.header.hash_slow();
    let expected_commitment = ReplayCommitment::from_header(expected_block_hash, &block.header);

    let (hash, _output) = stateless_validation(block, public_keys, witness, chain_spec, evm_config)
        .unwrap_or_else(|e| panic!("stateless validation failed: {e}"));
    assert_eq!(hash, expected_block_hash, "block hash mismatch");

    expected_commitment
}
