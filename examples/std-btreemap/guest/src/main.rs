#![no_main]

use airbender::guest::read;
use std::collections::BTreeMap;

#[airbender::main]
fn main() -> u32 {
    let base: u32 = read().expect("failed to read base");

    let mut map = BTreeMap::new();
    map.insert("alpha".to_string(), base);
    map.insert("beta".to_string(), base + 1);
    map.insert("gamma".to_string(), base + 2);

    map.values().copied().sum()
}
