#[cfg(not(target_arch = "riscv32"))]
mod naive;
#[cfg(not(target_arch = "riscv32"))]
pub use self::naive::Keccak256;

#[cfg(any(
    test,
    any(target_arch = "riscv32", feature = "testing", feature = "sha3_tests")
))]
pub mod delegated;

#[cfg(target_arch = "riscv32")]
pub use self::delegated::Keccak256;

// TODO: Add a platform-native guest integration harness for delegated Keccak tests.
