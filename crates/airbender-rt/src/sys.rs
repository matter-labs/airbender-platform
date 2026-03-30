//! Low-level CSR and exit helpers used by the guest runtime.

#[cfg(target_arch = "riscv32")]
pub fn read_word() -> u32 {
    riscv_common::csr_read_word()
}

#[cfg(not(target_arch = "riscv32"))]
pub fn read_word() -> u32 {
    panic!("csr_read_word is only available on riscv32")
}

#[cfg(target_arch = "riscv32")]
pub fn write_word(word: u32) {
    riscv_common::csr_write_word(word as usize);
}

#[cfg(not(target_arch = "riscv32"))]
pub fn write_word(_word: u32) {
    panic!("csr_write_word is only available on riscv32")
}

#[cfg(target_arch = "riscv32")]
pub fn emit_cycle_marker() {
    unsafe {
        // TODO: Make zksync-airbender the source of truth for this CSR value.
        core::arch::asm!(
            "csrrw x0, 0x7ff, x0",
            options(nomem, nostack, preserves_flags)
        )
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn emit_cycle_marker() {
    // Cycle markers are only meaningful on real guest targets.
}

#[cfg(target_arch = "riscv32")]
pub fn exit_success(words: &[u32; 8]) -> ! {
    riscv_common::zksync_os_finish_success(words)
}

#[cfg(not(target_arch = "riscv32"))]
pub fn exit_success(_words: &[u32; 8]) -> ! {
    panic!("exit_success is only available on riscv32")
}

#[cfg(target_arch = "riscv32")]
pub fn exit_error() -> ! {
    riscv_common::zksync_os_finish_error()
}

#[cfg(not(target_arch = "riscv32"))]
pub fn exit_error() -> ! {
    panic!("exit_error is only available on riscv32")
}
