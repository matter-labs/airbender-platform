use airbender_host::{Inputs, Program, ProverLevel, Result};
use ruint::aliases::U256;
use std::path::PathBuf;

fn main() -> Result<()> {
    let prove = std::env::args().skip(1).any(|arg| arg == "--prove");
    let program = Program::load(dist_dir())?;

    let a = U256::from(1u64);
    let b = U256::from(2u64);
    let c = U256::from(3u64);

    let mut inputs = Inputs::new();
    inputs.push(&a)?;
    inputs.push(&b)?;
    inputs.push(&c)?;

    let execution = program.execute(&inputs, None)?;
    let exec_valid = execution.receipt.output[0] == 1;
    println!(
        "Execution finished: cycles={}, reached_end={}, valid={}",
        execution.cycles_executed, execution.reached_end, exec_valid
    );
    assert!(exec_valid, "guest reported invalid sum");

    if !prove {
        println!("Skipping proof generation (pass `--prove` to generate and verify proof).");
        return Ok(());
    }

    let prover = program
        .gpu_prover()
        .with_level(ProverLevel::RecursionUnified)
        .build()?;
    let prove_result = program.prove(&prover, &inputs)?;
    let proof_valid = prove_result.receipt.output[0] == 1;
    println!(
        "Proof generated: cycles={}, valid={}",
        prove_result.cycles, proof_valid
    );

    let vk = program.compute_vk()?;
    program.verify(&prove_result.proof, &vk)?;
    println!("Proof verified.");

    assert_eq!(
        exec_valid, proof_valid,
        "execution and proof output mismatch"
    );

    Ok(())
}

fn dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app")
}
