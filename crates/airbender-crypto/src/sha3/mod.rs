#[cfg(not(target_arch = "riscv32"))]
mod naive;
#[cfg(not(target_arch = "riscv32"))]
pub use self::naive::Keccak256;

// The core in the layout of the keccak delegation, software natively (the C bindings expose it)
pub mod delegated;

#[cfg(target_arch = "riscv32")]
pub use self::delegated::Keccak256;

// TODO: Add a platform-native guest integration harness for delegated Keccak tests.
