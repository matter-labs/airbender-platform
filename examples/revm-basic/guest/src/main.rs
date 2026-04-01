#![no_main]

use airbender::guest::read;
use revm_basic_shared::RunInput;

#[airbender::main]
fn main() -> u32 {
    let input: RunInput = read().expect("failed to read input");
    let gas_used = revm_basic_shared::run(&input).expect("revm execution failed");
    gas_used as u32
}
