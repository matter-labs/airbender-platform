# Host Program API

Use `airbender-host` from native host applications to execute, prove, and verify guest programs.

## Add Dependency

```toml
[dependencies]
airbender-host = { path = "../../crates/airbender-host" }
```

## Core Workflow with `Program`

`Program` is the highest-level API.

```rust
use airbender_host::{Inputs, Program, Result};

fn run() -> Result<()> {
    let program = Program::load("../guest/dist/app")?;

    let mut inputs = Inputs::new();
    inputs.push(&10u32)?;

    let execution = program.execute(&inputs, None)?;
    println!("output x10={}", execution.receipt.output[0]);

    let prove_result = program.prove(&inputs, None)?;
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

- `Program::execute(&inputs, cycles)`
- `Program::prove(&inputs, worker_threads)`
- `Program::compute_vk()`
- `Program::verify(&proof, &vk)`

Lower-level (function-style):

- `run_simulator(...)`
- `run_simulator_with_flamegraph(...)`
- `run_transpiler(...)`
- `prove_with_options(...)`
- `compute_unified_vk(...)`, `compute_unrolled_vk(...)`
- `verify_proof(...)`, `verify_unrolled_proof(...)`

## `Receipt` Output

`Receipt` captures post-execution registers and output slices:

- `receipt.output` maps to `x10..x17` (8 words)
- `receipt.output_extended` maps to `x10..x25` (16 words)

These correspond to guest commit helpers and `#[airbender::main]` return values.

## Proof Options

`ProveOptions` controls proof behavior:

- `backend`: `Gpu` (default) or `Cpu`
- `worker_threads`: optional thread count
- `cycles`: optional cycle bound
- `ram_bound`: optional RAM bound (CPU backend)
- `level`: `Base`, `RecursionUnrolled`, `RecursionUnified`

Use GPU backend for recursion-friendly defaults. CPU backend is currently base-layer oriented.

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
