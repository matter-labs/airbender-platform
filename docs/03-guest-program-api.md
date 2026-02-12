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

## Committing Output

You have two common patterns:

1. Return a value from `#[airbender::main]` (automatic commit)
2. Call commit functions directly:

```rust
use airbender::guest::{commit, commit_extended, exit_error};

// Commit 8-word output and exit success.
commit(123u32);

// Commit 16-word extended output and exit success.
commit_extended([0u32; 16]);

// Exit with an error.
exit_error();
```

Built-in commit support includes `()`, `u32`, `u64`, `i64`, `bool`, `[u32; 8]`, and `[u32; 16]` (extended).

## Custom Output Layouts

To map your own type into output registers, implement commit traits from `airbender::guest`:

- `Commit` for 8-word output (`[u32; 8]`)
- `CommitExtended` for 16-word output (`[u32; 16]`)

This keeps guest-host output contracts explicit and stable.

## How Input/Output Maps to Host

- Host `Inputs::push(...)` order == guest `read::<T>()` consumption order
- Guest output maps to host `Receipt` fields:
  - `output` (`x10..x17`)
  - `output_extended` (`x10..x25`)

## Complete Guest Examples

See end-to-end guest code in:

- [`examples/fibonacci/guest`](../examples/fibonacci/guest/)
- [`examples/u256-add/guest`](../examples/u256-add/guest/)
- [`examples/std-btreemap/guest`](../examples/std-btreemap/guest/)
