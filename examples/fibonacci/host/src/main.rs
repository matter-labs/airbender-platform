use airbender_host::{Inputs, Program, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let prove = std::env::args().skip(1).any(|arg| arg == "--prove");
    let program = Program::load(dist_dir())?;

    let n: u32 = 10;
    let expected = 55u32;
    let mut inputs = Inputs::new();
    inputs.push(&n)?;

    let execution = program.execute(&inputs, None)?;
    let exec_output = execution.receipt.output[0];
    println!(
        "Execution finished: cycles={}, reached_end={}, output={}",
        execution.cycles_executed, execution.reached_end, exec_output
    );
    assert_eq!(exec_output, expected, "unexpected fibonacci output");

    if !prove {
        println!("Skipping proof generation (pass `--prove` to generate and verify proof).");
        return Ok(());
    }

    let prove_result = program.prove(&inputs, None)?;
    let proof_output = prove_result.receipt.output[0];
    println!(
        "Proof generated: cycles={}, output={}",
        prove_result.cycles, proof_output
    );
    assert_eq!(
        exec_output, proof_output,
        "execution and proof output mismatch"
    );

    let vk = program.compute_vk()?;
    program.verify(&prove_result.proof, &vk)?;
    println!("Proof verified.");

    Ok(())
}

fn dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app")
}
