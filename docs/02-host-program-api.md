# Host Program API

Use `airbender-host` from native host applications to execute, prove, and verify guest programs.

## Add Dependency

```toml
[dependencies]
airbender-host = { path = "../../crates/airbender-host" }
```

## Core Workflow with `Program`

`Program` is the highest-level API.

Create runners/provers once and reuse them across multiple `runner.run(...)` / `prover.prove(...)` calls.

```rust
use airbender_host::{Inputs, Program, Prover, ProverLevel, Result, Runner};

fn run() -> Result<()> {
    let program = Program::load("../guest/dist/app")?;

    let mut inputs = Inputs::new();
    inputs.push(&10u32)?;

    let simulator = program.simulator_runner().build()?;
    let execution = simulator.run(inputs.words())?;
    println!("output x10={}", execution.receipt.output[0]);

    let prover = program
        .gpu_prover()
        .with_level(ProverLevel::RecursionUnified)
        .build()?;
    let prove_result = prover.prove(inputs.words())?;
    let vk = program.compute_vk()?;
    program.verify(&prove_result.proof, &vk)?;
    Ok(())
}
```

## `Inputs`

`Inputs` frames host data for guest reads.

- `Inputs::push(&value)` serializes typed data via Airbender codec
- `Inputs::push_bytes(&bytes)` pushes raw framed bytes
- `Inputs::words()` exposes the low-level `u32` word stream

Guest-side `read::<T>()` calls consume values in the same order they were pushed.

## Execution APIs

High-level:

- `Program::simulator_runner()`
- `Program::transpiler_runner()`
- `Program::gpu_prover()`
- `Program::cpu_prover()`
- `Runner::run(&input_words)`
- `Prover::prove(&input_words)`
- `Program::compute_vk()`
- `Program::verify(&proof, &vk)`

Lower-level:

- `SimulatorRunnerBuilder::new(app_bin).with_...().build()`
- `TranspilerRunnerBuilder::new(app_bin).with_...().build()`
- `GpuProverBuilder::new(app_bin).with_...().build()`
- `CpuProverBuilder::new(app_bin).with_...().build()`
- `compute_unified_vk(...)`, `compute_unrolled_vk(...)`
- `verify_proof(...)`, `verify_unrolled_proof(...)`

## `Receipt` Output

`Receipt` captures post-execution registers and output slices:

- `receipt.output` maps to `x10..x17` (8 words)
- `receipt.output_extended` maps to `x10..x25` (16 words)

These correspond to guest commit helpers and `#[airbender::main]` return values.

## Prover Construction

- `GpuProverBuilder::new(...)` accepts path and supports `with_worker_threads(...)`, `with_level(...)`, then `build()`.
- `CpuProverBuilder::new(...)` accepts path and supports `with_worker_threads(...)`, `with_cycles(...)`, `with_ram_bound(...)`, then `build()`.
- `build()` returns `Result<...>` and performs path/config validation.
- CPU proving currently supports base-layer proving (`ProverLevel::Base`) only.

## Runner Construction

- `SimulatorRunnerBuilder::new(...)` accepts path and supports `with_cycles(...)`, `with_flamegraph(...)`, then `build()`.
- `TranspilerRunnerBuilder::new(...)` accepts path and supports `with_cycles(...)`, `with_text_path(...)`, then `build()`.

## Cycle Budget

For simulator execution, you can:

- pass an explicit cycle limit
- or rely on `AIRBENDER_MAX_CYCLES`

If neither is set through your flow, host utilities default to `100_000_000` cycles.

## Complete Working Examples

See full host-side usage in:

- [`examples/fibonacci/host`](../examples/fibonacci/host/)
- [`examples/u256-add/host`](../examples/u256-add/host/)
- [`examples/std-btreemap/host`](../examples/std-btreemap/host/)
