use airbender_host::{Inputs, Program, Prover, Runner, VerificationRequest, Verifier};
use alloy_primitives::{Bytes, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rlp::{Decodable, Encodable};
use alloy_rpc_types_debug::ExecutionWitness;
use eyre::Result;
use reth_ethereum_primitives::{Block, TransactionSigned};
use stateless::UncompressedPublicKey;
use std::path::PathBuf;

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

fn dev_chain_config() -> alloy_genesis::ChainConfig {
    use alloy_primitives::U256;
    alloy_genesis::ChainConfig {
        chain_id: 1337,
        homestead_block: Some(0),
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
        ..Default::default()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let prove = std::env::args().skip(1).any(|arg| arg == "--prove");
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".into());
    let block_num: u64 = std::env::var("BLOCK_NUM")
        .unwrap_or_else(|_| "1".into())
        .parse()?;

    let block_hex = format!("0x{block_num:x}");
    println!("Connecting to {rpc_url}, fetching block {block_num}...");

    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);

    let raw_block: Bytes = provider
        .raw_request("debug_getRawBlock".into(), vec![serde_json::Value::String(block_hex.clone())])
        .await?;
    let block = Block::decode(&mut raw_block.as_ref())
        .map_err(|e| eyre::eyre!("failed to decode block RLP: {e}"))?;

    println!(
        "Block {}: {} transactions, gas_used={}",
        block_num,
        block.body.transactions.len(),
        block.header.gas_used
    );

    let witness: ExecutionWitness = provider
        .raw_request(
            "debug_executionWitness".into(),
            vec![serde_json::Value::String(block_hex)],
        )
        .await?;

    println!(
        "Witness: {} state nodes, {} codes, {} keys, {} headers",
        witness.state.len(),
        witness.codes.len(),
        witness.keys.len(),
        witness.headers.len(),
    );

    let public_keys = recover_signers(&block.body.transactions)?;
    println!("Recovered {} public keys", public_keys.len());

    let chain_config = dev_chain_config();

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

    if !prove {
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
        block_num, expected_hash
    );

    Ok(())
}
