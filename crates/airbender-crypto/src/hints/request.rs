//! The request of a field hint query, how the oracle reads it, and the query itself.

use super::answer::HintAnswer;
use super::oracle::{HintError, HintOracle};
use super::FIELD_OPS_ADVISE_QUERY_ID;
use core::mem::{offset_of, MaybeUninit};

/// The request of a field hint query: the op, and the address and length (in `u32` words) of
/// the operand. `W` is the querier's word.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GenericFieldOpsHint<W> {
    pub op: u32,
    pub src_ptr: W,
    pub src_len_u32_words: u32,
}

pub type FieldOpsHint = GenericFieldOpsHint<u32>;
pub type FieldOpsHint64 = GenericFieldOpsHint<u64>;

/// A field hint. The oracle reads the operand by address, in the representation of the querier's
/// target (see `Secp256k1Element` and `hints::guest_layout`); answers are in that representation
/// too (see [`HintAnswer`]). The numbers are part of the protocol.
#[repr(u32)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldHintOp {
    /// secp256k1 base field square root: an element in; a candidate root and a "quadratic
    /// non-residue" flag out (the root of the input, or of its negation if it is not a square)
    Secp256k1BaseFieldSqrt = 0,
    /// secp256k1 base field inverse: an element in, its inverse out
    Secp256k1BaseFieldInverse,
    /// secp256k1 scalar field inverse: a scalar in, its inverse out
    Secp256k1ScalarFieldInverse,
    /// bn254 base field inverse: an element in, its inverse out
    Bn254BaseFieldInverse,
    /// bn254 `Fq12` inverse: an element in, its inverse out
    Bn254Fq12Inverse,
    /// bls12-381 base field square root: as for secp256k1 (`p = 3 mod 4`)
    Bls12381BaseFieldSqrt,
    /// bls12-381 base field inverse: an element in, its inverse out
    Bls12381BaseFieldInverse,
    /// bls12-381 `Fq12` inverse: an element in, its inverse out
    Bls12381Fq12Inverse,
    /// bn254 pairing claim (`hints::PairingClaim`): `(G1Affine, G2Affine)` pairs in; an
    /// identity flag, then `c`, `d = c^-1` (`Fq12`) and `s` (`Fq6`), or else the Miller loop
    /// inverse (`Fq12`)
    Bn254PairingResidueWitness,
    /// Slopes of the affine `G2` chains of a bn254 pairing point: a `G2Affine` in; a flag and 93
    /// `Fq2` subgroup-test slopes, then a flag and 87 line slopes out. A cleared flag marks an
    /// exceptional chain, for which the projective computation is used.
    Bn254G2PairingSlopes,
    /// KZG pairing claim for `e(P1, G2) e(P2, tau G2)`: `[G1Affine; 2]` in; an identity flag, then
    /// `d` (`Fq12`) and `s` (`Fq6`), or else the Miller loop inverse (`Fq12`)
    Bls12381KzgResidueWitness,
    /// secp256k1 base field division: numerator and denominator (adjacent) in, the quotient out
    Secp256k1BaseFieldDivision,
    /// bn254 base field division: numerator and denominator (adjacent) in, the quotient out
    Bn254BaseFieldDivision,
    /// bls12-381 base field division: numerator and denominator (adjacent) in, the quotient out
    Bls12381BaseFieldDivision,
    /// bls12-381 pairing claim: `hints::Bls12381Pair`s in; an identity flag, then `d, s`, or else
    /// the conjugated Miller loop inverse
    Bls12381PairingResidueWitness,
}

impl FieldHintOp {
    pub fn parse_u32(value: u32) -> Option<Self> {
        const ALL: [FieldHintOp; 15] = [
            FieldHintOp::Secp256k1BaseFieldSqrt,
            FieldHintOp::Secp256k1BaseFieldInverse,
            FieldHintOp::Secp256k1ScalarFieldInverse,
            FieldHintOp::Bn254BaseFieldInverse,
            FieldHintOp::Bn254Fq12Inverse,
            FieldHintOp::Bls12381BaseFieldSqrt,
            FieldHintOp::Bls12381BaseFieldInverse,
            FieldHintOp::Bls12381Fq12Inverse,
            FieldHintOp::Bn254PairingResidueWitness,
            FieldHintOp::Bn254G2PairingSlopes,
            FieldHintOp::Bls12381KzgResidueWitness,
            FieldHintOp::Secp256k1BaseFieldDivision,
            FieldHintOp::Bn254BaseFieldDivision,
            FieldHintOp::Bls12381BaseFieldDivision,
            FieldHintOp::Bls12381PairingResidueWitness,
        ];
        ALL.into_iter().find(|op| *op as u32 == value)
    }
}

/// Read access of the oracle to the memory of the querier.
pub trait QuerierMemory {
    /// Size in bytes of a word, and of an address, of the querier: 4 on the proving target, and
    /// `size_of::<usize>()` for a querier in the same process.
    fn word_size(&self) -> usize;

    /// Reads the `u32` at `address`, which must be 4-byte aligned.
    fn read_u32(&self, address: usize) -> Result<u32, HintError>;
}

pub(super) fn offset_address(address: usize, offset: usize) -> Result<usize, HintError> {
    address
        .checked_add(offset)
        .ok_or(HintError("querier address overflows"))
}

/// Reads a querier word (`QuerierMemory::word_size` bytes, low half first).
pub fn read_querier_word<M: QuerierMemory + ?Sized>(
    memory: &M,
    address: usize,
) -> Result<usize, HintError> {
    let low = memory.read_u32(address)?;
    match memory.word_size() {
        4 => Ok(low as usize),
        8 => {
            let high = memory.read_u32(offset_address(address, 4)?)?;
            usize::try_from(u64::from(low) | (u64::from(high) << 32))
                .map_err(|_| HintError("querier word does not fit into usize"))
        }
        _ => Err(HintError("unsupported querier word size")),
    }
}

/// Reads the `GenericFieldOpsHint<usize>` at `address` of the querier ([`FieldOpsHint`] on the
/// guest, [`FieldOpsHint64`] in this process): the op, and the operand's address and length.
pub fn read_field_hint_request<M: QuerierMemory + ?Sized>(
    memory: &M,
    address: usize,
) -> Result<(u32, usize, u32), HintError> {
    let (op, src_ptr, src_len_u32_words) = match memory.word_size() {
        4 => (
            offset_of!(FieldOpsHint, op),
            offset_of!(FieldOpsHint, src_ptr),
            offset_of!(FieldOpsHint, src_len_u32_words),
        ),
        8 => (
            offset_of!(FieldOpsHint64, op),
            offset_of!(FieldOpsHint64, src_ptr),
            offset_of!(FieldOpsHint64, src_len_u32_words),
        ),
        _ => return Err(HintError("unsupported querier word size")),
    };
    let field = |offset: usize| {
        address
            .checked_add(offset)
            .ok_or(HintError("querier address overflows"))
    };
    Ok((
        memory.read_u32(field(op)?)?,
        read_querier_word(memory, field(src_ptr)?)?,
        memory.read_u32(field(src_len_u32_words)?)?,
    ))
}

/// Sends the request for the hint `op` on `operand`. The caller then reads the answer with
/// [`HintAnswer::read`] and ends the query with [`HintOracle::finish_query`].
///
/// # Safety
///
/// `operand` must be a value of the operand type of `op` (see [`FieldHintOp`]), e.g.
/// `[FieldElement; 2]` for [`FieldHintOp::Secp256k1BaseFieldDivision`]: an oracle in this process
/// reads it as one.
#[inline(always)]
pub unsafe fn send_field_hint_query<O: HintOracle, T: ?Sized>(
    oracle: &mut O,
    op: FieldHintOp,
    operand: &T,
) -> Result<(), HintError> {
    let size = core::mem::size_of_val(operand);
    debug_assert!(size.is_multiple_of(4) && core::mem::align_of_val(operand) >= 4);
    // the oracle reads the request and the operand by address
    let request = GenericFieldOpsHint::<usize> {
        op: op as u32,
        src_ptr: core::ptr::from_ref(operand)
            .cast::<u8>()
            .expose_provenance(),
        src_len_u32_words: (size / 4) as u32,
    };
    // SAFETY: the input word is the address of `request`, live for the call, whose operand is
    // `operand`, a value of the operand type of `op` (the caller's obligation)
    unsafe {
        oracle.send_query(
            FIELD_OPS_ADVISE_QUERY_ID,
            core::ptr::from_ref(&request).expose_provenance(),
        )
    }
}

/// Asks for the hint `op` on `operand` and receives the answer into `dst`, checked to be
/// well-formed only: the caller checks that it is right.
///
/// # Safety
///
/// As for [`send_field_hint_query`]: `operand` must be a value of the operand type of `op`.
#[inline(always)]
pub unsafe fn query_field_hint_into<'a, O: HintOracle, R: HintAnswer, T: ?Sized>(
    oracle: &mut O,
    op: FieldHintOp,
    operand: &T,
    dst: &'a mut MaybeUninit<R>,
) -> &'a mut R {
    // SAFETY: guaranteed by the caller
    unsafe { send_field_hint_query(oracle, op, operand) }.expect("must send the field hint query");
    let answer = R::read(oracle, dst).expect("the hint answer is well-formed");
    oracle
        .finish_query()
        .expect("the hint answer has no excess data");
    answer
}

/// As [`query_field_hint_into`], with the answer returned as a new value.
///
/// # Safety
///
/// As for [`send_field_hint_query`]: `operand` must be a value of the operand type of `op`.
#[inline(always)]
pub unsafe fn query_field_hint<O: HintOracle, R: HintAnswer, T: ?Sized>(
    oracle: &mut O,
    op: FieldHintOp,
    operand: &T,
) -> R {
    let mut answer = MaybeUninit::uninit();
    // SAFETY: guaranteed by the caller
    unsafe { query_field_hint_into(oracle, op, operand, &mut answer) };
    // SAFETY: initialized by `query_field_hint_into`
    unsafe { answer.assume_init() }
}
