use crate::BigInt;

pub(crate) mod delegation;
pub mod u256;
pub mod u512;

pub trait DelegatedModParams<const N: usize>: Default {
    const MODULUS_BITSIZE: usize;

    /// The elements are kept as any representative in `[0, 2 modulus)` instead of the canonical
    /// one below the modulus: a Montgomery multiplication then needs no conditional
    /// subtraction at its end (its result is below `2 modulus` whenever the inputs are), which
    /// saves one or two delegations of its eight, and the additions reduce by `2 modulus`.
    /// Requires `4 modulus < 2^(64 N)`, so that products and sums of representatives fit the
    /// bounds of the reduction, and `2 modulus < 2^(64 N - 1)` for the doubling by a product.
    /// Equality and zero tests then compare modulo the modulus (`u256::eq_mod`), and
    /// `into_bigint` canonicalizes.
    const REDUNDANT: bool = false;

    /// Provides a reference to the modululs for delegation purposes
    fn modulus() -> &'static BigInt<N>;

    /// `2 modulus`, the reduction constant of the redundant representation
    fn double_modulus() -> &'static BigInt<N> {
        unreachable!("only a field in the redundant representation reduces by twice its modulus")
    }
}

pub trait DelegatedMontParams<const N: usize>: DelegatedModParams<N> {
    /// Provides a reference to the reduction const (`-1/Self::modulus mod 2^256`) for Montgomerry reduction
    fn reduction_const() -> &'static BigInt<4>;
}

pub trait DelegatedBarretParams<const N: usize>: DelegatedModParams<N> {
    /// Provides a reference to `-Self::modulus mod 2^256` for Barret reduction
    fn neg_modulus() -> &'static BigInt<4>;
}
