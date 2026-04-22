mod utils;

use airbender_host::{Inputs, Program, Prover, Runner, VerificationRequest, Verifier};
use alloy_primitives::Bytes;
use alloy_provider::{ext::DebugApi, ProviderBuilder};
use alloy_rlp::{Decodable, Encodable};
use clap::Parser;
use eyre::Result;
use reth_block_replay_shared::ReplayCommitment;
use reth_ethereum_primitives::Block;
use std::path::PathBuf;
use utils::{fetch_chain_config, fetch_execution_witness, recover_signers};

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
        "block {} has no transactions; run examples/reth-block-replay/docker/generate-blocks.py first",
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
    let expected_commitment = ReplayCommitment::from_header(expected_hash, &block.header);
    println!("Expected block hash: {expected_hash}");
    println!(
        "Expected public commitment: {}",
        expected_commitment.digest()
    );

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
        execution.receipt.output,
        expected_commitment.public_output_words(),
        "guest commitment mismatch"
    );
    println!("Simulation verified: correctness commitment matches.");

    if !args.prove {
        println!("Skipping proof (pass `--prove` to generate and verify).");
        return Ok(());
    }

    let prover = program.cpu_prover().build()?;
    let prove_result = prover.prove(inputs.words())?;
    assert_eq!(
        prove_result.receipt.output,
        expected_commitment.public_output_words(),
        "proof output mismatch"
    );

    let verifier = program
        .real_verifier(airbender_host::ProverLevel::Base)
        .build()?;
    let vk = verifier.generate_vk()?;
    verifier.verify(
        &prove_result.proof,
        &vk,
        VerificationRequest::real(&expected_commitment.public_output_words()),
    )?;
    println!(
        "Proof verified: block {} (hash={}) proven in ZK.",
        args.block_num, expected_hash
    );

    Ok(())
}
