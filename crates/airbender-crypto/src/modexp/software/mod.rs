//! ModExp without hints: the MPNat code of aurora-engine
//! (<https://github.com/aurora-is-near/aurora-engine>) with an explicit allocator. Vectors are
//! never grown in place (no `Allocator::grow`): a longer vector is a new allocation and a copy.

// the `as` conversions are reviewed and safe
#![allow(clippy::as_conversions)]
use alloc::vec::Vec;
use core::alloc::Allocator;

mod arith;
mod mpnat;

macro_rules! vec_in {
    // `n` copies of `elem`
    ($alloc:expr; $elem:expr; $n:expr) => {{
        let mut v = Vec::with_capacity_in($n, $alloc);
        v.resize($n, $elem);
        v
    }};

    // one element
    ($alloc:expr; $x:expr) => {{
        let mut v = Vec::with_capacity_in(1, $alloc);
        v.push($x);
        v
    }};
}
use vec_in;

/// Computes `(base ^ exp) % modulus`, where all values are given as big-endian
/// encoded bytes.
pub fn modexp<A: Allocator + Clone>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
    allocator: A,
) -> Vec<u8, A> {
    let m = mpnat::MPNat::from_big_endian(modulus, allocator.clone());
    if m.digits.len() == 1 && m.digits[0] == 0 {
        return Vec::new_in(allocator);
    }
    let mut x = mpnat::MPNat::from_big_endian(base, allocator.clone());
    let result = x.modpow(exp, &m, allocator.clone());
    result.to_big_endian(allocator)
}
