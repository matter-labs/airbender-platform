#[cfg(bigint_ops)]
mod fq;

#[cfg(bigint_ops)]
pub use self::fq::Fq;

#[cfg(not(bigint_ops))]
pub use ark_bn254::Fq;

#[cfg(bigint_ops)]
mod fr;
#[cfg(bigint_ops)]
pub use self::fr::Fr;
#[cfg(not(bigint_ops))]
pub use ark_bn254::Fr;

pub mod fq2;
pub use self::fq2::*;

pub mod fq6;
pub use self::fq6::*;

pub mod fq12;
pub use self::fq12::*;

assert_tower_layout!(Fq, Fq2, Fq6, Fq12);
