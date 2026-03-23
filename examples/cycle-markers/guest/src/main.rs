#![no_std]
#![no_main]

use airbender::crypto::{sha3::Keccak256, MiniDigest};
use airbender::guest::cycle_marker;

#[airbender::main]
fn main() -> u32 {
    cycle_marker();
    let digest = Keccak256::digest(b"airbender cycle markers");
    cycle_marker();

    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}
