use airbender_host::{Inputs, Program, Prover, Runner, VerificationRequest, Verifier};
use revm_basic_shared::WitnessInput;
use std::path::PathBuf;

const CALLER: [u8; 20] = [0x11; 20];
const CONTRACT: [u8; 20] = [0x22; 20];
const GAS_LIMIT: u64 = 100_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prove = std::env::args().skip(1).any(|arg| arg == "--prove");

    let witness = WitnessInput {
        caller: CALLER,
        tx_to: CONTRACT,
        gas_limit: GAS_LIMIT,
    };

    let expected_gas = revm_basic_shared::run_witness(&witness)?;
    let expected = expected_gas as u32;
    println!("Native revm: gas_used={expected_gas}");

    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app");
    let program = Program::load(&dist_dir)?;

    let mut inputs = Inputs::new();
    inputs.push(&witness)?;

    let runner = program.transpiler_runner().build()?;
    let execution = runner.run(inputs.words())?;
    let guest_output = execution.receipt.output[0];
    println!(
        "Guest execution: cycles={}, reached_end={}, gas_used={}",
        execution.cycles_executed, execution.reached_end, guest_output
    );
    assert_eq!(guest_output, expected, "guest and native gas mismatch");

    if !prove {
        println!("Skipping proof (pass `--prove` to generate and verify).");
        return Ok(());
    }

    let prover = program.dev_prover().build()?;
    let prove_result = prover.prove(inputs.words())?;
    assert_eq!(
        execution.receipt.output, prove_result.receipt.output,
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
