# Host Program API

Use `airbender-host` from native host applications to execute, prove, and verify guest programs.

## Add Dependency

```toml
[dependencies]
airbender-host = { path = "../../crates/airbender-host" }
```

GPU support is enabled by default. To keep a dev-only host binary, disable default features:

```toml
[dependencies]
airbender-host = { path = "../../crates/airbender-host", default-features = false }
```

## Core Workflow with `Program`

`Program` is the highest-level API.

Create runners/provers once and reuse them across multiple `runner.run(...)` / `prover.prove(...)` calls.

```rust
use airbender_host::{
    Inputs, Program, Prover, Result, Runner, VerificationRequest, Verifier,
};

fn run() -> Result<()> {
    let program = Program::load("../guest/dist/app")?;

    let mut inputs = Inputs::new();
    inputs.push(&10u32)?;

    let simulator = program.simulator_runner().build()?;
    let execution = simulator.run(inputs.words())?;
    println!("output x10={}", execution.receipt.output[0]);

    let prover = program.dev_prover().build()?;
    let prove_result = prover.prove(inputs.words())?;

    let verifier = program.dev_verifier().build()?;
    let vk = verifier.generate_vk()?;
    verifier.verify(
        &prove_result.proof,
        &vk,
        VerificationRequest::dev(inputs.words(), &55u32),
    )?;
    Ok(())
}
```

## `Inputs`

`Inputs` frames host data for guest reads.

- `Inputs::push(&value)` serializes typed data via Airbender codec
- `Inputs::push_bytes(&bytes)` pushes raw framed bytes
- `Inputs::words()` exposes the low-level `u32` word stream
- `Inputs::write_hex_file(path)` writes CLI-compatible hex input (`--input`)

Guest-side `read::<T>()` calls consume values in the same order they were pushed.

## Execution APIs

High-level:

- `Program::simulator_runner()`
- `Program::transpiler_runner()`
- `Program::dev_prover()`
- `Program::gpu_prover()`
- `Program::cpu_prover()`
- `Program::dev_verifier()`
- `Program::real_verifier(level)`
- `Runner::run(&input_words)`
- `Prover::prove(&input_words)`
- `Verifier::generate_vk()`
- `Verifier::verify(&proof, &vk, request)`
- `VerificationRequest::dev(...)` / `VerificationRequest::real(...)`

Lower-level:

- `SimulatorRunnerBuilder::new(app_bin).with_...().build()`
- `TranspilerRunnerBuilder::new(app_bin).with_...().with_jit().build()` (`with_jit()` is optional and x86_64-only)
- `DevProverBuilder::new(app_bin).with_...().build()`
- `GpuProverBuilder::new(app_bin).with_...().build()`
- `CpuProverBuilder::new(app_bin).with_...().build()`
- `DevVerifierBuilder::new(app_bin).build()`
- `RealVerifierBuilder::new(app_bin, level).build()`
- `compute_unified_vk(...)`, `compute_unrolled_vk(...)`
- `verify_proof(...)`, `verify_unrolled_proof(...)`

Verification APIs can enforce expected public outputs (`x10..x17`) in addition to proof validity.

## `Receipt` Output

`Receipt` captures post-execution registers and output slices:

- `receipt.output` maps to `x10..x17` (8 words)
- `receipt.output_extended` maps to `x10..x25` (16 words, includes recursion-chain fields)

`#[airbender::main]` return values and `guest::commit(...)` map to `receipt.output`.

## Prover Construction

- `DevProverBuilder::new(...)` accepts path and supports `with_cycles(...)`, `with_text_path(...)`, then `build()`.
- `GpuProverBuilder::new(...)` accepts path and supports `with_worker_threads(...)`, `with_level(...)`, then `build()`.
- `CpuProverBuilder::new(...)` accepts path and supports `with_worker_threads(...)`, `with_cycles(...)`, `with_ram_bound(...)`, then `build()`.
- `build()` returns `Result<...>` and performs path/config validation.
- CPU proving currently supports base-layer proving (`ProverLevel::Base`) only.
- GPU proving is enabled by default; if you disable default features, re-enable `gpu-prover`.

## Runner Construction

- `SimulatorRunnerBuilder::new(...)` accepts path and supports `with_cycles(...)`, `with_flamegraph(...)`, then `build()`.
- `TranspilerRunnerBuilder::new(...)` accepts path and supports `with_cycles(...)`, `with_text_path(...)`, then `build()`.

## Cycle Budget

For simulator execution, you can:

- pass an explicit cycle limit

If no explicit cycle limit is set through your flow, a default high value will be used.

## Complete Working Examples

See full host-side usage in:

- [`examples/fibonacci/host`](../examples/fibonacci/host/)
- [`examples/u256-add/host`](../examples/u256-add/host/)
- [`examples/std-btreemap/host`](../examples/std-btreemap/host/)
