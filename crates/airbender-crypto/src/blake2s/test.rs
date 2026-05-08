// Tests that run the blake2s test program as a RISC-V binary and verify correctness.
//
// Prerequisites: build the test program binaries first:
//   cd src/blake2s/test_program && ./dump_bin.sh

use airbender_host::{Program, Runner};
use std::path::PathBuf;

fn test_program_dist_dir(app_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/blake2s/test_program/dist")
        .join(app_name)
}

#[test]
pub fn run_naive_test() {
    let program = Program::load(test_program_dist_dir("app_native_blake"))
        .expect("failed to load program — did you run dump_bin.sh?");
    let runner = program
        .transpiler_runner()
        .with_cycles(1 << 25)
        .build()
        .expect("failed to build runner");
    let result = runner.run(&[]).expect("execution failed");
    assert_eq!(result.receipt.output[0], 1);
}

#[test]
pub fn run_extended_delegation_test() {
    let program = Program::load(test_program_dist_dir("app_extended_delegation_blake"))
        .expect("failed to load program — did you run dump_bin.sh?");
    let runner = program
        .transpiler_runner()
        .with_cycles(1 << 25)
        .build()
        .expect("failed to build runner");
    let result = runner.run(&[]).expect("execution failed");
    assert_eq!(result.receipt.output[0], 1);
}
