//! Development-only helpers for placing cycle markers in guest code.

/// Emit a cycle marker boundary recognized by the transpiler runner.
///
/// Cycle markers are intended for local transpiler profiling. They should not
/// be used in programs that will be proved with the real CPU/GPU proving path.
#[inline(always)]
pub fn mark() {
    airbender_rt::sys::emit_cycle_marker();
}
