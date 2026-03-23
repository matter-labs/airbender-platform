use airbender_host::{Program, Result, Runner};
use std::path::PathBuf;

fn main() -> Result<()> {
    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app");
    let program = Program::load(&dist_dir)?;
    let runner = program.transpiler_runner().build()?;
    let execution = runner.run(&[])?;
    let markers = execution
        .cycle_markers
        .as_ref()
        .expect("cycle markers must be collected for non-JIT transpiler runs");

    assert!(execution.reached_end, "guest execution did not reach the exit loop");
    assert_eq!(markers.markers.len(), 2, "expected exactly two cycle markers");

    let profiled_section = markers.markers[1].diff(&markers.markers[0]);
    assert!(
        profiled_section.cycles > 0,
        "profiled section must consume at least one cycle"
    );
    assert!(
        !profiled_section.delegations.is_empty(),
        "delegated Keccak section should report delegation activity"
    );

    println!(
        "Execution finished: cycles={}, output={}, section_cycles={}, section_delegations={:?}",
        execution.cycles_executed,
        execution.receipt.output[0],
        profiled_section.cycles,
        profiled_section.delegations
    );

    Ok(())
}
