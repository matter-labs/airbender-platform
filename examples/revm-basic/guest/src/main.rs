#![no_main]

use airbender::guest::read;
use revm_basic_shared::WitnessInput;

#[airbender::main]
fn main() -> u32 {
    let witness: WitnessInput = read().expect("failed to read witness input");
    let gas_used = revm_basic_shared::run_witness(&witness).expect("revm execution failed");
    gas_used as u32
}
