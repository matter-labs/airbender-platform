//! The big-integer arithmetic under the multi-digit path, one implementation for each kind of
//! target, selected by `cfg(bigint_ops)` alone:
//!
//! - `delegated` (with it: RV32, and the hosts that emulate its delegation): a digit is 256 bits,
//!   aligned for the bigint delegation, and every operation is a sequence of delegation calls;
//! - `limbs` (without it: RV64 and the other hosts): a digit is one `u64` limb, and the
//!   operations are carry chains over limb slices.
//!
//! Both export the same items, and the path above (`bigint`) is written on them alone, in digits
//! of whatever size:
//!
//! | Item | |
//! | --- | --- |
//! | `Digit`, `DIGIT_BYTES`, `DIGIT_ONE` | the digit (little-endian `u64` limbs), its size, the digit 1 |
//! | `Scratch` | working space of the products, made once per ModExp |
//! | `from_be_bytes_in_place`, `to_be_bytes` | a digit from and to big-endian bytes |
//! | `is_zero`, `is_one` | tests of one digit |
//! | `copy` | digits into fresh slots |
//! | `mul`, `square` | `out = a * b` and `out = a * a`, all digits of `out` written |
//! | `mul_add` | `out = q * m + r` in exactly the digits of `out`, or that it does not fit |
//! | `eq_value` | whether two digit strings are the same integer |
//! | `is_below` | `a < m` for strings of one length |
//! | `protocol_digits`, `pad_for_query` | a length in the 256-bit digits of the advice request, and the zeroes that complete the last of them |
//!
//! The advice protocol counts the dividend and the modulus of a request in 256-bit digits and
//! the answers in `u32` words; a digit of either size is a whole number of words, and
//! `protocol_digits` rounds a length up to the request's unit.

#[cfg(bigint_ops)]
mod delegated;
#[cfg(not(bigint_ops))]
mod limbs;

#[cfg(bigint_ops)]
pub(crate) use self::delegated::*;
#[cfg(not(bigint_ops))]
pub(crate) use self::limbs::*;

/// `u64` limbs of a digit
pub(crate) const DIGIT_LIMBS: usize = DIGIT_BYTES / 8;

/// Oracle (`u32`) words of a digit
pub(crate) const DIGIT_WORDS: usize = DIGIT_BYTES / 4;

/// The most digits of an operand (EIP-7823)
pub(crate) const MAX_DIGITS: usize = crate::modexp::MAX_OPERAND_BYTES / DIGIT_BYTES;
