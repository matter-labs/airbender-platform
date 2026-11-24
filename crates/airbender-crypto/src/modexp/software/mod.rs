//! ModExp without hints: the MPNat code of aurora-engine
//! (<https://github.com/aurora-is-near/aurora-engine>) on fixed-capacity digit vectors
//! ([`FixedVec`]), sized by the EIP-7823 bound on the operands: nothing is allocated.

// the `as` conversions are reviewed and safe
#![allow(clippy::as_conversions)]

use super::fixed_vec::FixedVec;
use super::{ModExpOutput, MAX_OPERAND_BYTES};

mod arith;
mod mpnat;

use self::mpnat::{Word, WORD_BYTES};

/// The most digits of an operand (EIP-7823)
pub const MAX_WORDS: usize = MAX_OPERAND_BYTES / WORD_BYTES;

/// The digits of an `MPNat`: room for a product of two operands and the overflow word of a
/// normalization
pub type Digits = FixedVec<Word, { 2 * MAX_WORDS + 1 }>;

/// Computes `(base ^ exp) % modulus`, where all values are given as big-endian
/// encoded bytes. The base and the modulus must have at most [`MAX_OPERAND_BYTES`] significant
/// bytes (EIP-7823), or this panics.
pub fn modexp(base: &[u8], exp: &[u8], modulus: &[u8]) -> ModExpOutput {
    let m = mpnat::MPNat::from_big_endian(super::checked_operand(modulus));
    if m.digits.len() == 1 && m.digits[0] == 0 {
        return ModExpOutput::new();
    }
    let mut x = mpnat::MPNat::from_big_endian(super::checked_operand(base));
    let result = x.modpow(exp, &m);
    result.to_big_endian()
}
