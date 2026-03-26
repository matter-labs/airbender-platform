# Host Program API

The host is where you load your guest program, feed it inputs, and decide whether to just run it or generate a proof. Everything here is normal Rust, no RISC-V, no `no_std`.

## Add Dependency

```toml
[dependencies]
airbender-host = { path = "../../crates/airbender-host" }
```

GPU support is enabled by default. If you only need the dev prover, disable default features:

```toml
[dependencies]
airbender-host = { path = "../../crates/airbender-host", default-features = false }
```

Always use `--release` when building or running host binaries.

## Core Workflow

Load the program, push inputs, run it, then prove and verify. Runners and provers are reusable: build them once, call `run()`/`prove()` as many times as you need.

```rust
use airbender_host::{
    Inputs, Program, Prover, Result, Runner, VerificationRequest, Verifier,
};

fn run() -> Result<()> {
    // Load guest artifacts from the dist directory
    let program = Program::load("../guest/dist/app")?;

    // Serialize inputs - order must match guest read() calls
    let mut inputs = Inputs::new();
    inputs.push(&10u32)?;

    // Run the guest
    let runner = program.transpiler_runner().build()?;
    let execution = runner.run(inputs.words())?;
    println!("output x10={}", execution.receipt.output[0]);

    // Prove execution
    let prover = program.dev_prover().build()?;
    let prove_result = prover.prove(inputs.words())?;

    // Verify the proof
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

## Inputs

`Inputs` serializes host data into the `u32` word stream that the guest reads.

**Push order matters.** Guest-side `read::<T>()` calls consume values in the exact order they were pushed. If you push a `u32` then a `Vec<u8>`, the guest must read a `u32` first and a `Vec<u8>` second. A mismatch will cause a decode error on the guest side.

- `push(&value)` - serialize any `serde::Serialize` type via the Airbender codec
- `push_bytes(&bytes)` - push raw bytes using the wire framing protocol
- `words()` - access the underlying `u32` word stream
- `write_hex_file(path)` - write a CLI-compatible hex input file (for use with `--input`)

## Running

`Program::transpiler_runner()` returns a builder for transpiler-based execution:

```rust
let runner = program.transpiler_runner()
    .with_cycles(1_000_000)  // optional cycle limit
    .with_jit()              // optional JIT on x86_64
    .build()?;
let result = runner.run(inputs.words())?;
```

The default cycle limit is high enough for most programs. JIT is faster but disables cycle marker collection.

## Proving

Three prover backends are available:

```rust
// Dev - no real cryptography, for local testing
let prover = program.dev_prover().build()?;

// GPU - full proving, requires NVIDIA GPU with 32GB+ VRAM
let prover = program.gpu_prover()
    .with_level(ProverLevel::RecursionUnified)
    .build()?;

// CPU - base layer only, mainly for debugging circuits
let prover = program.cpu_prover()
    .with_worker_threads(8)
    .build()?;
```

All provers share the same interface: `prover.prove(inputs.words())`.

## Verification

```rust
// Dev verification
let verifier = program.dev_verifier().build()?;
let vk = verifier.generate_vk()?;
verifier.verify(&proof, &vk, VerificationRequest::dev(inputs.words(), &expected))?;

// Real verification (GPU-generated proofs)
let verifier = program.real_verifier(ProverLevel::RecursionUnified).build()?;
let vk = verifier.generate_vk()?;
verifier.verify(&proof, &vk, VerificationRequest::real(&expected))?;
```

Verification can optionally enforce expected public outputs (`x10..x17`) in addition to proof validity.

## Receipt Output

After execution or proving, the `Receipt` contains the guest's output:

- `receipt.output` - registers `x10..x17` (8 words). This is where `#[airbender::main]` return values and `guest::commit(...)` land.
- `receipt.output_extended` - registers `x10..x25` (16 words, includes recursion-chain fields).

For non-JIT transpiler runs, `ExecutionResult::cycle_markers` contains the captured marker snapshots. JIT runs return `None`.

## Common Mistakes

- **Input order mismatch:** the host pushes values in a different order than the guest reads them. The guest will get a codec decode error.
- **Forgetting `--release`:** host binaries are significantly slower in debug mode. Proving can be orders of magnitude slower.
- **GPU features disabled:** if you installed `airbender-host` with `default-features = false`, GPU prover/verifier methods won't be available. Re-enable with `features = ["gpu-prover"]`.

## Examples

See full host-side usage in:

- [`examples/fibonacci/host`](https://github.com/matter-labs/airbender-platform/tree/main/examples/fibonacci/host)
- [`examples/u256-add/host`](https://github.com/matter-labs/airbender-platform/tree/main/examples/u256-add/host)
- [`examples/std-btreemap/host`](https://github.com/matter-labs/airbender-platform/tree/main/examples/std-btreemap/host)
- [`examples/cycle-markers/host`](https://github.com/matter-labs/airbender-platform/tree/main/examples/cycle-markers/host)
