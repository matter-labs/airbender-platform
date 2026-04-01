use airbender_host::{Inputs, Program, Prover, Runner, VerificationRequest, Verifier};
use alloy_genesis::ChainConfig;
use alloy_primitives::{Bytes, B256, U256};
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use alloy_rlp::{Decodable, Encodable};
use alloy_rpc_types_debug::ExecutionWitness;
use clap::Parser;
use eyre::Result;
use reth_ethereum_primitives::{Block, TransactionSigned};
use stateless::UncompressedPublicKey;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Replay a reth block with its execution witness inside Airbender")]
struct Args {
    /// JSON-RPC endpoint exposing the reth debug APIs used by this example.
    #[arg(long, default_value = "http://localhost:8545")]
    rpc_url: String,
    /// Block number to replay from the connected reth node.
    #[arg(long, default_value_t = 1)]
    block_num: u64,
    /// Generate and verify a proof after the simulation pass.
    #[arg(long)]
    prove: bool,
}

fn recover_signers(txs: &[TransactionSigned]) -> Result<Vec<UncompressedPublicKey>> {
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

fn b256_to_u32x8(hash: B256) -> [u32; 8] {
    let bytes = hash.as_slice();
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = u32::from_le_bytes([
            bytes[i * 4],
            bytes[i * 4 + 1],
            bytes[i * 4 + 2],
            bytes[i * 4 + 3],
        ]);
    }
    out
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

async fn fetch_execution_witness<P: Provider>(
    provider: &P,
    block_num: u64,
) -> Result<ExecutionWitness> {
    Ok(provider.debug_execution_witness(block_num.into()).await?)
}

async fn fetch_chain_config<P: Provider>(provider: &P) -> Result<ChainConfig> {
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!(
        "Connecting to {}, fetching block {}...",
        args.rpc_url, args.block_num
    );

    let provider = ProviderBuilder::new().connect_http(args.rpc_url.parse()?);

    let raw_block: Bytes = provider.debug_get_raw_block(args.block_num.into()).await?;
    let block = Block::decode(&mut raw_block.as_ref())
        .map_err(|e| eyre::eyre!("failed to decode block RLP: {e}"))?;

    eyre::ensure!(
        !block.body.transactions.is_empty(),
        "block {} has no transactions; run examples/reth-block-replay/docker/generate-blocks.sh first",
        args.block_num
    );

    println!(
        "Block {}: {} transactions, gas_used={}",
        args.block_num,
        block.body.transactions.len(),
        block.header.gas_used
    );

    let witness = fetch_execution_witness(&provider, args.block_num).await?;

    println!(
        "Witness: {} state nodes, {} codes, {} keys, {} headers",
        witness.state.len(),
        witness.codes.len(),
        witness.keys.len(),
        witness.headers.len(),
    );

    let public_keys = recover_signers(&block.body.transactions)?;
    println!("Recovered {} public keys", public_keys.len());

    let chain_config = fetch_chain_config(&provider).await?;

    let expected_hash = block.header.hash_slow();
    let expected_commitment = b256_to_u32x8(expected_hash);
    println!("Expected block hash: {expected_hash}");

    let mut block_rlp = Vec::new();
    block.encode(&mut block_rlp);

    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app");
    let program = Program::load(&dist_dir)?;

    let chain_config_json = serde_json::to_vec(&chain_config)?;

    let mut inputs = Inputs::new();
    inputs.push(&block_rlp)?;
    inputs.push(&witness)?;
    inputs.push(&chain_config_json)?;
    inputs.push(&public_keys)?;

    let runner = program.transpiler_runner().build()?;
    let execution = runner.run(inputs.words())?;

    println!(
        "Guest execution: cycles={}, reached_end={}",
        execution.cycles_executed, execution.reached_end,
    );

    assert_eq!(
        execution.receipt.output, expected_commitment,
        "guest commitment mismatch"
    );
    println!("Simulation verified: block hash matches.");

    if !args.prove {
        println!("Skipping proof (pass `--prove` to generate and verify).");
        return Ok(());
    }

    let prover = program.cpu_prover().build()?;
    let prove_result = prover.prove(inputs.words())?;
    assert_eq!(
        prove_result.receipt.output, expected_commitment,
        "proof output mismatch"
    );

    let verifier = program
        .real_verifier(airbender_host::ProverLevel::Base)
        .build()?;
    let vk = verifier.generate_vk()?;
    verifier.verify(
        &prove_result.proof,
        &vk,
        VerificationRequest::real(&expected_commitment),
    )?;
    println!(
        "Proof verified: block {} (hash={}) proven in ZK.",
        args.block_num, expected_hash
    );

    Ok(())
}
