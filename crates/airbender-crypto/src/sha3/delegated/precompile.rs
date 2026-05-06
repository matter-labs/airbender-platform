#[cfg(not(target_arch = "riscv32"))]
compile_error!("invalid arch - should only be compiled for RISC-V");

use super::AlignedState;

use common_constants::delegation_types::keccak_special5;

pub(crate) fn keccak_f1600(state: &mut AlignedState) {
    keccak_special5::keccak_f1600(state);
}
