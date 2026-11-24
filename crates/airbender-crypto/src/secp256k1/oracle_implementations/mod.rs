//! The oracle side of the secp256k1 hints: the answers to the field hint queries of the
//! recovery (`hints::Secp256k1HooksWithOracle`), computed natively, and the prover's side of
//! the witnessed recovery ([`witness`]).

#[cfg(feature = "secp256k1-static-context")]
pub mod witness;

use super::field::FieldElement;
use super::scalars::Scalar;
use super::RecoveryWitness;
use crate::hints::{HintTarget, RecoveryRequest, Secp256k1Element};
use crate::oracle_common::Operand;

/// `(candidate, is_quadratic_non_residue)`: `candidate² == input` for a square, `-input` otherwise
///
/// # Safety
///
/// For a querier in this process `operand` must be the operand of this op, a `FieldElement`
/// (what `Operand::new` was promised): it is read in place as that type. A guest operand is read
/// through the guest's memory only.
pub unsafe fn base_field_sqrt(operand: &Operand) -> (FieldElement, bool) {
    // SAFETY: the caller's obligation
    let el: FieldElement = unsafe { operand.secp256k1_element() };
    assert!(!el.is_zero());
    let mut candidate = el;
    let is_quadratic_residue = candidate.sqrt_in_place();
    (candidate, !is_quadratic_residue)
}

/// # Safety
///
/// For a querier in this process `operand` must be the operand of this op, a `FieldElement`
/// (what `Operand::new` was promised): it is read in place as that type. A guest operand is read
/// through the guest's memory only.
pub unsafe fn base_field_inverse(operand: &Operand) -> FieldElement {
    // SAFETY: the caller's obligation
    let mut el: FieldElement = unsafe { operand.secp256k1_element() };
    assert!(!el.is_zero());
    el.invert_in_place();
    el
}

/// The quotient of the numerator and the (non-zero) denominator, two adjacent elements
///
/// # Safety
///
/// For a querier in this process `operand` must be the operand of this op, a `[FieldElement; 2]`
/// (what `Operand::new` was promised): it is read in place as that type. A guest operand is read
/// through the guest's memory only.
pub unsafe fn base_field_division(operand: &Operand) -> FieldElement {
    // SAFETY: the caller's obligation
    let [numerator, denominator]: [FieldElement; 2] = unsafe { operand.secp256k1_elements() };
    assert!(!denominator.is_zero());
    let mut quotient = denominator;
    quotient.invert_in_place();
    quotient.mul_in_place(&numerator);
    quotient
}

/// # Safety
///
/// For a querier in this process `operand` must be the operand of this op, a `Scalar`
/// (what `Operand::new` was promised): it is read in place as that type. A guest operand is read
/// through the guest's memory only.
pub unsafe fn scalar_field_inverse(operand: &Operand) -> Scalar {
    // SAFETY: the caller's obligation
    let el: Scalar = unsafe { operand.secp256k1_element() };
    assert!(!el.is_zero());
    el.invert_vartime()
}

/// The witness of the key recovery the operand (a `RecoveryRequest`) asks for, if there is one:
/// none for a key at infinity, for which the recovery fails as computed. Without the static
/// context there is no witness either, and the querier computes the key.
///
/// # Safety
///
/// For a querier in this process `operand` must be the operand of this op, a `RecoveryRequest`
/// (what `Operand::new` was promised): it is read in place as that type. A guest operand is read
/// through the guest's memory only.
pub unsafe fn recovery_witness(operand: &Operand) -> Option<RecoveryWitness> {
    let request: RecoveryRequest = match operand.querier() {
        // SAFETY: the caller's obligation
        HintTarget::Native => unsafe { operand.native_value() },
        HintTarget::Guest => {
            assert_eq!(operand.size(), 5 * 32, "the operand is a recovery request");
            let words = |i: usize| -> [u32; 8] {
                operand.guest_words(32 * i, 8).try_into().expect("8 words")
            };
            RecoveryRequest {
                r: Scalar::from_guest_operand_words(&words(0)),
                s: Scalar::from_guest_operand_words(&words(1)),
                message: Scalar::from_guest_operand_words(&words(2)),
                x: FieldElement::from_guest_operand_words(&words(3)),
                y: FieldElement::from_guest_operand_words(&words(4)),
            }
        }
    };
    #[cfg(feature = "secp256k1-static-context")]
    {
        witness::recovery_witness(&request.point(), &request.r, &request.s, &request.message)
    }
    #[cfg(not(feature = "secp256k1-static-context"))]
    {
        let _ = request;
        None
    }
}
