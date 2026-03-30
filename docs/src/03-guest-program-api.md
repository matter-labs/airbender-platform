# Guest Program API

Guest programs use `airbender-sdk` (imported as `airbender`) to read host inputs and commit outputs.

## Add Dependency

```toml
[dependencies]
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk" }
```

Enable `std` guest support only when needed:

```toml
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", features = ["std"] }
```

Enable `crypto` to expose `airbender::crypto` from the SDK. This is guest-oriented and always enables `airbender-crypto`'s `proving` feature:

```toml
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", features = ["crypto"] }
```

Allocator selection is feature-based (`allocator-talc` default, or `allocator-bump` / `allocator-custom`):

```toml
airbender = { package = "airbender-sdk", path = "../../crates/airbender-sdk", default-features = false, features = ["allocator-bump"] }
```

## Entry Point: `#[airbender::main]`

Write a regular Rust function and annotate it:

```rust
#[airbender::main]
fn main() -> u32 {
    42
}
```

Rules:

- function must not take arguments
- function must not be `async`
- function should return a value that can be committed (or `()`)

The macro provides the runtime entry point and commits the function result as guest output.

For custom allocator wiring (`allocator-custom`), you must provide an init hook:

```rust
#[airbender::main(allocator_init = crate::custom_allocator::init)]
fn main() -> u32 {
    42
}
```

## Reading Input Data

Use `airbender::guest::read::<T>()` for typed values:

```rust
use airbender::guest::read;

#[airbender::main]
fn main() -> u32 {
    let n: u32 = read().expect("failed to read input");
    n + 1
}
```

For custom transports (e.g. tests), use `read_with(&mut transport)`.

## Cycle Markers

Use `airbender::guest::cycle_marker()` to emit a profiling boundary that the
transpiler runner can observe. For the common "measure this block" case, use
`airbender::guest::record_cycles(...)`; call `cycle_marker()` directly when you
need manual boundaries:

```rust
use airbender::guest::record_cycles;

#[airbender::main]
fn main() -> u32 {
    record_cycles(|| 40 + 2)
}
```

Cycle markers are intended for transpiler profiling only. Real CPU/GPU proving
rejects binaries that contain marker CSRs, so treat them as development-only
artifacts.

## Committing Output

You have two common patterns:

1. Return a value from `#[airbender::main]` (automatic commit)
2. Call commit functions directly:

```rust
use airbender::guest::{commit, exit_error};

// Commit 8-word output and exit success.
commit(123u32);

// Exit with an error.
exit_error();
```

Built-in commit support includes `()`, `u32`, `u64`, `i64`, `bool`, and `[u32; 8]`.

## Custom Output Layouts

To map your own type into output registers, implement `Commit` from `airbender::guest` for 8-word output (`[u32; 8]`).

This keeps guest-host output contracts explicit and stable.

## How Input/Output Maps to Host

- Host `Inputs::push(...)` order == guest `read::<T>()` consumption order
- Guest output maps to host `Receipt` fields:
  - `output` (`x10..x17`)
  - `output_extended` (`x10..x25`, includes recursion-specific words)

## Complete Guest Examples

See end-to-end guest code in:

- [`examples/cycle-markers/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/cycle-markers/guest)
- [`examples/fibonacci/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/fibonacci/guest)
- [`examples/u256-add/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/u256-add/guest)
- [`examples/std-btreemap/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/std-btreemap/guest)
- [`examples/revm-basic/guest`](https://github.com/matter-labs/airbender-platform/tree/main/examples/revm-basic/guest)
