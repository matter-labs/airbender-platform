#![no_std]
#![no_main]

use airbender::crypto::{sha3::Keccak256, MiniDigest};
use airbender::guest::record_cycles;

#[airbender::main]
fn main() -> u32 {
    let digest = record_cycles(|| Keccak256::digest(b"airbender cycle markers"));

    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}
