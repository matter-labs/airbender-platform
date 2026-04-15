use alloy_genesis::ChainConfig;
use alloy_primitives::U256;
use alloy_provider::{ext::DebugApi, Provider};
use alloy_rpc_types_debug::ExecutionWitness;
use eyre::Result;
use reth_ethereum_primitives::TransactionSigned;
use stateless::UncompressedPublicKey;

pub fn recover_signers(txs: &[TransactionSigned]) -> Result<Vec<UncompressedPublicKey>> {
    txs.iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .map(|keys| {
                    UncompressedPublicKey(
                        keys.to_encoded_point(false).as_bytes().try_into().unwrap(),
                    )
                })
                .map_err(|e| eyre::eyre!("failed to recover signature for tx #{i}: {e}"))
        })
        .collect()
}

pub async fn fetch_execution_witness<P: Provider>(
    provider: &P,
    block_num: u64,
) -> Result<ExecutionWitness> {
    Ok(provider.debug_execution_witness(block_num.into()).await?)
}

pub async fn fetch_chain_config<P: Provider>(provider: &P) -> Result<ChainConfig> {
    let chain_id = provider.get_chain_id().await?;

    if chain_id == 1337 {
        return Ok(dev_chain_config());
    }

    let mut chain_config: ChainConfig =
        provider.raw_request("debug_chainConfig".into(), ()).await?;

    if chain_config.chain_id != chain_id {
        eprintln!(
            "debug_chainConfig returned chain_id={}, overriding with eth_chainId={chain_id}",
            chain_config.chain_id
        );
        chain_config.chain_id = chain_id;
    }

    Ok(chain_config)
}

fn dev_chain_config() -> ChainConfig {
    ChainConfig {
        chain_id: 1337,
        homestead_block: Some(0),
        dao_fork_block: Some(0),
        dao_fork_support: true,
        eip150_block: Some(0),
        eip155_block: Some(0),
        eip158_block: Some(0),
        byzantium_block: Some(0),
        constantinople_block: Some(0),
        petersburg_block: Some(0),
        istanbul_block: Some(0),
        berlin_block: Some(0),
        london_block: Some(0),
        terminal_total_difficulty: Some(U256::ZERO),
        terminal_total_difficulty_passed: true,
        shanghai_time: Some(0),
        cancun_time: Some(0),
        prague_time: Some(0),
        osaka_time: Some(0),
        ..Default::default()
    }
}
