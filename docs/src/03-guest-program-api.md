# Guest Program API

A guest program runs on a RISC-V virtual machine. Its execution trace can be proven in zero knowledge. Guest programs are `no_std` by default and communicate with the host through a typed input/output channel.

## Add Dependency

```toml
[dependencies]
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk" }
```

Enable `std` when you need standard library collections or I/O:

```toml
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", features = ["std"] }
```

Enable `crypto` for prover-accelerated cryptographic primitives (see [Using Crypto](./04-crypto-on-guest-and-host.md)):

```toml
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", features = ["crypto"] }
```

The default allocator is `talc`. To switch to `bump` or `custom`:

```toml
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", default-features = false, features = ["allocator-bump"] }
```

## Entry Point

Write a regular Rust function and annotate it with `#[airbender::main]`:

```rust
#[airbender::main]
fn main() -> u32 {
    42
}
```

The macro sets up the runtime entry point and commits the return value as guest output. Your function:

- Must not take arguments
- Must not be `async`
- Should return a type that implements `Commit` (or `()`)

For `allocator-custom`, you must wire up the allocator init hook:

```rust
#[airbender::main(allocator_init = crate::custom_allocator::init)]
fn main() -> u32 {
    42
}
```

## Reading Input

Use `read::<T>()` to deserialize typed values from the host. Each call consumes the next value in the input stream, in the same order the host pushed them.

```rust
use airbender::guest::read;

#[airbender::main]
fn main() -> u32 {
    let n: u32 = read().expect("failed to read input");
    n + 1
}
```

For unit testing with mock inputs, use `read_with(&mut transport)` with a `MockTransport`.

## Committing Output

Two patterns:

**1. Return from `main` (preferred).** The return value is committed automatically:

```rust
#[airbender::main]
fn main() -> u32 {
    42 // written to output register x10
}
```

**2. Call `commit` directly.** Useful for early exits or complex control flow:

```rust
use airbender::guest::commit;

commit(123u32);  // write output and exit success (never returns)
```

**3. Exit with error.** Signal that the guest program failed:

```rust
use airbender::guest::exit_error;

exit_error();  // exit with error status (never returns)
```

Built-in `Commit` implementations: `()`, `u32`, `u64`, `i64`, `bool`, `[u32; 8]`.

## Custom Output Types

To commit your own type, implement the `Commit` trait. It maps your value to 8 `u32` words that land in output registers `x10..x17`:

```rust
use airbender::guest::Commit;

struct MyOutput {
    a: u32,
    b: u32,
}

impl Commit for MyOutput {
    fn commit_words(&self) -> [u32; 8] {
        let mut words = [0u32; 8];
        words[0] = self.a;
        words[1] = self.b;
        words
    }
}
```

## Cycle Markers

Cycle markers let you profile how many VM cycles a block of guest code takes. Use `record_cycles(...)` for the common case:

```rust
use airbender::guest::record_cycles;

#[airbender::main]
fn main() -> u32 {
    record_cycles(|| 40 + 2)
}
```

For manual boundaries, call `cycle_marker()` directly.

**Important:** cycle markers are for transpiler profiling only. Real CPU/GPU proving rejects binaries that contain marker CSRs, so don't ship them in production builds.

## How Input/Output Maps to Host

- Host `Inputs::push(...)` order must match guest `read::<T>()` order exactly.
- Guest output lands in host `Receipt` fields:
  - `receipt.output` → registers `x10..x17` (8 words)
  - `receipt.output_extended` → registers `x10..x25` (16 words, includes recursion-chain fields)

## Examples

- [`examples/fibonacci/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/fibonacci/guest) - basic no_std computation
- [`examples/u256-add/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/u256-add/guest) - no_std with external crates
- [`examples/std-btreemap/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/std-btreemap/guest) - std-enabled guest
- [`examples/cycle-markers/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/cycle-markers/guest) - profiling with delegation
- [`examples/revm-basic/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/revm-basic/guest) - revm transaction inside Airbender
- [`examples/reth-block-replay/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/reth-block-replay/guest) - replay a reth block from execution witness data
