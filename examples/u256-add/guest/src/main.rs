#![no_std]
#![no_main]

use airbender::guest::read;
use ruint::aliases::U256;

#[airbender::main]
fn main() -> bool {
    let a: U256 = read().expect("failed to read input a");
    let b: U256 = read().expect("failed to read input b");
    let c: U256 = read().expect("failed to read input c");

    let sum = a + b;
    assert_eq!(sum, c, "u256 addition check failed");

    true
}
