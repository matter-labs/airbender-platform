//! The hints on RV32 without `bigint_ops`, where they are not there (the bn254 and bls12-381
//! ones answer straight into the delegated representation, see `hints::guest_layout`):
//! stand-ins with the names and the signatures of the hints that the pairing checks, the MSMs
//! and the KZG proof check ask, each of which panics with `hints_unavailable`. Those endpoints
//! take the hints through the oracle at many points of their code, not in one arm, so on this
//! target their code stays the same, and runs as on any other without an oracle; with one, it
//! panics where it would ask its first hint, as the hinted arms of the other endpoints do.

use crate::affine_glv::Divider;
use crate::hints::HintOracle;
use core::mem::MaybeUninit;

pub(super) use crate::hints::{checked_inverse, PairingClaim};

/// A divider that takes hints, or the hinted slopes of a G2 point. No value is ever made: the
/// functions that would return one panic first. The type is not empty all the same, so that the
/// compiler does not take the code after such a call in the endpoints for unreachable.
pub(super) struct Unavailable(());

impl Unavailable {
    /// The divider of the hints of `oracle` (as `Bls12381FqDivider::new`)
    // the value after the panic is for the type only
    #[allow(unreachable_code)]
    pub(super) fn new<O: HintOracle>(_oracle: &mut O) -> Self {
        super::hints_unavailable();
        Self(())
    }
}

/// As `G2SlopeHints`, which ends its query when it is dropped (this never runs: no value is made)
impl Drop for Unavailable {
    fn drop(&mut self) {}
}

impl<F> Divider<F> for Unavailable {
    fn divide<'q>(
        &mut self,
        _fraction: &mut [F; 2],
        _quotient: &'q mut MaybeUninit<F>,
    ) -> &'q mut F {
        super::hints_unavailable()
    }
}

/// `hints::Bls12381FqDivider`
pub(super) type Bls12381FqDivider = Unavailable;

/// `hints::bls12_381_fq_sqrt`
pub(super) fn bls12_381_fq_sqrt<O: HintOracle>(
    _oracle: &mut O,
    _a: &crate::bls12_381::Fq,
) -> Option<crate::bls12_381::Fq> {
    super::hints_unavailable()
}

/// `hints::bls12_381_kzg_residue_witness`
pub(super) fn bls12_381_kzg_residue_witness<O: HintOracle>(
    _oracle: &mut O,
    _points: &[crate::bls12_381::G1Affine; 2],
) -> PairingClaim<crate::bls12_381::Fq12, crate::bls12_381::Fq6> {
    super::hints_unavailable()
}

// The hints of the endpoints that allocate

/// `hints::Bls12381Fq2Divider`
#[cfg(any(feature = "alloc", test))]
pub(super) type Bls12381Fq2Divider = Unavailable;

/// `hints::Bn254FqDivider`
#[cfg(any(feature = "alloc", test))]
pub(super) type Bn254FqDivider = Unavailable;

/// `hints::G2SlopeHints`
#[cfg(any(feature = "alloc", test))]
impl Unavailable {
    pub(super) fn subgroup_chain(&mut self) -> Option<Self> {
        super::hints_unavailable()
    }

    pub(super) fn line_chain(&mut self) -> Option<Self> {
        super::hints_unavailable()
    }
}

/// `hints::bn254_g2_pairing_slopes`
#[cfg(any(feature = "alloc", test))]
pub(super) fn bn254_g2_pairing_slopes<O: HintOracle>(
    _oracle: &mut O,
    _q: &crate::bn254::G2Affine,
) -> Unavailable {
    super::hints_unavailable()
}

/// `hints::bn254_pairing_residue_witness`
#[cfg(any(feature = "alloc", test))]
pub(super) fn bn254_pairing_residue_witness<O: HintOracle>(
    _oracle: &mut O,
    _pairs: &[(crate::bn254::G1Affine, crate::bn254::G2Affine)],
) -> PairingClaim<crate::bn254::Fq12, crate::bn254::Fq6> {
    super::hints_unavailable()
}

/// `hints::bls12_381_fq_inverse`
#[cfg(any(feature = "alloc", test))]
pub(super) fn bls12_381_fq_inverse<O: HintOracle>(
    _oracle: &mut O,
    _a: &crate::bls12_381::Fq,
) -> Option<crate::bls12_381::Fq> {
    super::hints_unavailable()
}

/// `hints::bls12_381_fq2_inverse`
#[cfg(any(feature = "alloc", test))]
pub(super) fn bls12_381_fq2_inverse<O: HintOracle>(
    _oracle: &mut O,
    _value: &crate::bls12_381::Fq2,
) -> Option<crate::bls12_381::Fq2> {
    super::hints_unavailable()
}

/// `hints::bls12_381_pairing_residue_witness`
#[cfg(any(feature = "alloc", test))]
pub(super) fn bls12_381_pairing_residue_witness<O: HintOracle>(
    _oracle: &mut O,
    _pairs: &[crate::hints::Bls12381Pair],
) -> PairingClaim<crate::bls12_381::Fq12, crate::bls12_381::Fq6> {
    super::hints_unavailable()
}
