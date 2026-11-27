#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
mod naive;
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub use self::naive::Keccak256;

#[cfg(any(
    test,
    any(target_arch = "riscv32", feature = "testing", feature = "sha3_tests")
))]
pub mod delegated;

#[cfg(target_arch = "riscv32")]
pub use self::delegated::Keccak256;

// The RV64 guest: a sponge around a Keccak-f[1600] in assembly. The sponge alone also builds
// for the host tests, over a software permutation.
#[cfg(any(target_arch = "riscv64", test))]
mod rv64;
#[cfg(target_arch = "riscv64")]
pub use self::rv64::{
    keccak_f1600, keccak_f1600_complemented, Assembly, Keccak256, Keccak256Core, KeccakF1600,
    COMPLEMENTED_LANES,
};

// TODO: Add a platform-native guest integration harness for delegated Keccak tests.
