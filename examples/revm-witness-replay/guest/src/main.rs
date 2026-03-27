#![no_main]

#[path = "../../revm_runner.rs"]
mod revm_runner;
#[path = "../../shared.rs"]
mod shared;

use airbender::guest::read;
use shared::WitnessInput;

#[airbender::main]
fn main() -> u32 {
    let witness: WitnessInput = read().expect("failed to read witness input");
    let gas_used = revm_runner::run_witness(&witness).expect("revm execution failed");
    gas_used as u32
}
