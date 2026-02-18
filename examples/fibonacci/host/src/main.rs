use airbender_host::{Inputs, Program, Prover, Result, Runner, VerificationRequest, Verifier};
use std::path::PathBuf;

fn main() -> Result<()> {
    let prove = std::env::args().skip(1).any(|arg| arg == "--prove");
    // We assume that the guest program is located in `../guest/dist/app`,
    // which is the default behavior of `cargo airbender build`.
    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app");
    let program = Program::load(&dist_dir)?;

    let n: u32 = 10;
    let expected = 55u32;
    let mut inputs = Inputs::new();
    inputs.push(&n)?;

    let simulator = program.simulator_runner().build()?;
    let execution = simulator.run(inputs.words())?;
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

    let prover = program.dev_prover().build()?;
    let prove_result = prover.prove(inputs.words())?;
    let proof_output = prove_result.receipt.output[0];
    println!(
        "Proof generated: cycles={}, output={}",
        prove_result.cycles, proof_output
    );
    assert_eq!(
        exec_output, proof_output,
        "execution and proof output mismatch"
    );

    let verifier = program.dev_verifier().build()?;
    let vk = verifier.generate_vk()?;
    verifier.verify(
        &prove_result.proof,
        &vk,
        VerificationRequest::dev(inputs.words(), &expected),
    )?;
    println!("Proof verified.");

    Ok(())
}
