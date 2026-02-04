#![no_std]
#![no_main]

use airbender::guest::read;

#[airbender::main]
fn main() -> u32 {
    let value: u32 = read().expect("failed to read input");
    value + 1
}
