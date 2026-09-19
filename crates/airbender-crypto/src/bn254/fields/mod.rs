#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
mod fq;

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
pub use self::fq::Fq;

#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
pub use ark_bn254::Fq;

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
mod fr;
#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
pub use self::fr::Fr;
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
pub use ark_bn254::Fr;

pub mod fq2;
pub use self::fq2::*;

pub mod fq6;
pub use self::fq6::*;

pub mod fq12;
pub use self::fq12::*;
