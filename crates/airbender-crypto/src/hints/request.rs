// Ported from zksync-os e855bca2:basic_system/src/system_functions/field_ops.rs
// Ported from zksync-os e855bca2:zk_ee/src/oracle/memory_io/host.rs
//! The request of a field hint query, how the oracle reads it, and the query itself.

use super::answer::HintAnswer;
use super::oracle::{HintError, HintOracle};
use super::FIELD_OPS_ADVISE_QUERY_ID;
use core::mem::{offset_of, MaybeUninit};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GenericFieldOpsHint<W> {
    pub op: u32,
    pub src_ptr: W,
    pub src_len_u32_words: u32,
}

pub type FieldOpsHint = GenericFieldOpsHint<u32>;
pub type FieldOpsHint64 = GenericFieldOpsHint<u64>;

/// A field hint. The operand of an op is sent by address: the oracle reads the operand where it
/// is, in the representation of the target of the querier, which it knows from the run mode (a
/// querier in this process: the value itself; the RISC-V guest: its memory, see
/// `Secp256k1Element` and `hints::guest_layout`). The answers are in the representation of
/// the target too (see [`HintAnswer`]).
#[repr(u32)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldHintOp {
    /// secp256k1 base field square root: an element in; a candidate root and a "quadratic
    /// non-residue" flag out (the root of the input, or of its negation when the input is not a
    /// square)
    Secp256k1BaseFieldSqrt = 0,
    /// secp256k1 base field inverse: an element in, its inverse out
    Secp256k1BaseFieldInverse,
    /// secp256k1 scalar field inverse: a scalar in, its inverse out
    Secp256k1ScalarFieldInverse,
    /// bn254 base field inverse: an element in, its inverse out
    Bn254BaseFieldInverse,
    /// bn254 `Fq12` inverse: an element in, its inverse out
    Bn254Fq12Inverse,
    /// bls12-381 base field square root: an element in; a candidate root and a "quadratic
    /// non-residue" flag out. As for secp256k1 (`p = 3 mod 4`), the candidate is the root of the
    /// input, or of its negation when the input is not a square.
    Bls12381BaseFieldSqrt,
    /// bls12-381 base field inverse: an element in, its inverse out
    Bls12381BaseFieldInverse,
    /// bls12-381 `Fq12` inverse: an element in, its inverse out
    Bls12381Fq12Inverse,
    /// The prover's claim about a bn254 pairing product (see `crypto::residue_witness` and
    /// `hints::PairingClaim`): the slice of the affine pairs `(G1Affine, G2Affine)` in; an
    /// "is the identity" flag, then for an identity the witness `c`, `d = c^-1` (`Fq12` each) and
    /// the scaling factor (`Fq6`), otherwise the inverse of the Miller loop output (`Fq12`)
    Bn254PairingResidueWitness,
    /// The slopes of the two affine `G2` chains of a bn254 pairing input point (see
    /// `crypto::bn254::curves::g2_affine`): the `G2Affine` point in; a flag and the 93 slopes
    /// (`Fq2`) of the subgroup membership test, then a flag and the 87 slopes of the line
    /// precomputation out. A cleared flag means the chain hit an exceptional case and the
    /// projective computation is to be used.
    Bn254G2PairingSlopes,
    /// The prover's claim about the KZG proof pairing product `e(P1, G2) e(P2, tau G2)`: the
    /// `G1Affine` points `[P1, P2]` in; an "is the identity" flag, then for an identity the
    /// witness `d` (`Fq12`) and the scaling factor (`Fq6`), otherwise the inverse of the Miller
    /// loop output (`Fq12`)
    Bls12381KzgResidueWitness,
    /// secp256k1 base field division: the numerator and the denominator (two adjacent elements)
    /// in, the quotient out
    Secp256k1BaseFieldDivision,
    /// bn254 base field division: the numerator and the denominator (two adjacent elements)
    /// in, the quotient out
    Bn254BaseFieldDivision,
    /// bls12-381 base field division: the numerator and the denominator (two adjacent
    /// elements) in, the quotient out
    Bls12381BaseFieldDivision,
    /// Arbitrary validated BLS12-381 affine pairs (`hints::Bls12381Pair`) in;
    /// identity flag then `d, s` for an identity, otherwise the conjugated Miller loop inverse.
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

/// Reads a word of the querier (`QuerierMemory::word_size` bytes), e.g. an element of the address array of
/// a composite input, or a field of a request struct of querier words.
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

/// The request of a hint query, as the oracle reads it through the input word, which is the address
/// of a `GenericFieldOpsHint<usize>` of the querier: a [`FieldOpsHint`] on the proving target, a
/// [`FieldOpsHint64`] for a querier in the same process. Returns the op, and the address and the
/// length (in `u32` words) of the operand.
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

/// Sends the request for the hint `op` on `operand`, which the oracle reads where it is (see
/// [`FieldHintOp`]). The answer is then received with [`HintAnswer::read`] (or
/// [`read_hint_answer`](super::read_hint_answer)), piecemeal if needed, and the query ended with
/// [`HintOracle::finish_query`].
///
/// # Safety
///
/// `operand` must be a value of the operand type of `op` (see [`FieldHintOp`]), e.g. a
/// `FieldElement` for [`FieldHintOp::Secp256k1BaseFieldInverse`] or `[FieldElement; 2]` for
/// [`FieldHintOp::Secp256k1BaseFieldDivision`]: an oracle that answers in this process reads it as
/// one ([`HintOracle::send_query`]).
#[inline(always)]
pub unsafe fn send_field_hint_query<O: HintOracle, T: ?Sized>(
    oracle: &mut O,
    op: FieldHintOp,
    operand: &T,
) -> Result<(), HintError> {
    let size = core::mem::size_of_val(operand);
    debug_assert!(size.is_multiple_of(4) && core::mem::align_of_val(operand) >= 4);
    // the oracle reads the request and the operand while it receives the query: their addresses
    // are exposed
    let request = GenericFieldOpsHint::<usize> {
        op: op as u32,
        src_ptr: core::ptr::from_ref(operand)
            .cast::<u8>()
            .expose_provenance(),
        src_len_u32_words: (size / 4) as u32,
    };
    // SAFETY: the input word is the address of `request`, a live `GenericFieldOpsHint<usize>`
    // (the request of `FIELD_OPS_ADVISE_QUERY_ID` on this target), whose operand pointer
    // references `operand`, alive for the call, and a value of the operand type of `op` (the
    // caller's obligation)
    unsafe {
        oracle.send_query(
            FIELD_OPS_ADVISE_QUERY_ID,
            core::ptr::from_ref(&request).expose_provenance(),
        )
    }
}

/// Asks the oracle for the hint `op` on `operand` (see [`send_field_hint_query`]), and receives the
/// answer straight into `dst`. The answer is checked to be well-formed only: it is up to the caller
/// to check that it is right.
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
