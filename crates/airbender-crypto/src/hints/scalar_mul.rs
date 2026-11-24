//! The witness of a scalar multiplication (`affine_glv::MulWitness`) as a hint: the request of
//! the bn254 `G1` multiplication, and the words of a witness.

#[cfg(any(feature = "oracle_implementations", test))]
use super::answer::HintTarget;
use super::answer::{read_hint_answer, HintAnswer};
use super::request::{send_field_hint_query, FieldHintOp};
use crate::affine_glv::MulWitness;
use crate::bn254::{Fq, Fr, G1Affine};
use airbender_oracle::MemoryOracle;
#[cfg(any(feature = "oracle_implementations", test))]
use alloc::vec::Vec;
use core::mem::MaybeUninit;

/// The operand of [`FieldHintOp::Bn254G1MulWitness`]: the scalar, reduced modulo the group
/// order, and the coordinates of the point, which is not the point at infinity; each in the
/// querier's representation (see `hints::guest_layout`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bn254G1MulRequest {
    pub scalar: Fr,
    pub x: Fq,
    pub y: Fq,
}

/// The words of a witness: the coordinates of the product as base field elements of the
/// querier's target, then each coefficient as the 4 little-endian words of its two's complement
/// (the same on every target). The coordinates are checked to be canonical; that the witness is
/// right is for `affine_glv::check_mul_witness`.
impl<F: HintAnswer> HintAnswer for MulWitness<F> {
    #[inline(always)]
    fn read<'a, O: MemoryOracle>(
        oracle: &mut O,
        dst: &'a mut MaybeUninit<Self>,
    ) -> Result<&'a mut Self, O::Error> {
        let this = dst.as_mut_ptr();
        // SAFETY: the coordinates and the coefficients are disjoint places inside `dst`, which
        // they initialize
        unsafe {
            F::read(oracle, &mut *(&raw mut (*this).x).cast())?;
            F::read(oracle, &mut *(&raw mut (*this).y).cast())?;
            // each coefficient is four words: two halves, the low one first
            let mut coefficients = [0i128; 4];
            for coefficient in &mut coefficients {
                let low = oracle.read_u64()?;
                let high = oracle.read_u64()?;
                *coefficient = (u128::from(low) | u128::from(high) << 64) as i128;
            }
            (&raw mut (*this).coefficients).write(coefficients);
            Ok(dst.assume_init_mut())
        }
    }

    #[cfg(any(feature = "oracle_implementations", test))]
    fn write(&self, target: HintTarget, response: &mut Vec<u32>) {
        self.x.write(target, response);
        self.y.write(target, response);
        for coefficient in self.coefficients {
            for word in coefficient.to_le_bytes().as_chunks::<4>().0 {
                response.push(u32::from_le_bytes(*word));
            }
        }
    }
}

/// The witness of `scalar * point` from the oracle, unchecked; `None` if the oracle has none.
/// `scalar` is not zero and `point` is a point of the curve other than the point at infinity.
pub fn bn254_g1_mul_witness<O: MemoryOracle>(
    oracle: &mut O,
    point: &G1Affine,
    scalar: &Fr,
) -> Option<MulWitness<Fq>> {
    let request = Bn254G1MulRequest {
        scalar: *scalar,
        x: point.x,
        y: point.y,
    };
    // SAFETY: `Bn254G1MulRequest` is the operand type of the multiplication witness
    unsafe { send_field_hint_query(oracle, FieldHintOp::Bn254G1MulWitness, &request) }
        .expect("must send the field hint query");
    let present: bool = read_hint_answer(oracle).expect("the hint answer is well-formed");
    let witness =
        present.then(|| read_hint_answer(oracle).expect("the hint answer is well-formed"));
    oracle
        .finish_query()
        .expect("the hint answer has no excess data");
    witness
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::test_support::TapeOracle;
    use ark_ff::Field;

    /// The words of a witness read back as the witness, for the coefficients of either sign and
    /// the extremes of their type; a coordinate that is not canonical is not read
    #[test]
    fn a_witness_is_read_as_it_is_written() {
        let witness = MulWitness {
            x: Fq::from(3u64).pow([77]),
            y: -Fq::from(5u64),
            coefficients: [-1, i128::MIN, i128::MAX, 0x0123_4567_89ab_cdef_0011_2233],
        };
        let mut words = Vec::new();
        witness.write(HintTarget::Native, &mut words);
        assert_eq!(words.len(), 8 + 8 + 16);
        assert_eq!(words[16..20], [u32::MAX; 4]);
        assert_eq!(words[20..24], [0, 0, 0, 1 << 31]);
        let mut oracle = TapeOracle::new(words.iter().copied());
        let read: MulWitness<Fq> = read_hint_answer(&mut oracle).unwrap();
        assert_eq!(read, witness);
        assert!(oracle.words.is_empty());

        // the modulus in place of `y`
        let modulus = <Fq as ark_ff::PrimeField>::MODULUS;
        for (word, limb) in words[8..16].chunks_mut(2).zip(modulus.0) {
            word[0] = limb as u32;
            word[1] = (limb >> 32) as u32;
        }
        let mut oracle = TapeOracle::new(words);
        assert!(read_hint_answer::<_, MulWitness<Fq>>(&mut oracle).is_err());
    }
}
