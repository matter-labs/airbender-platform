# Airbender Examples

Each example has a `guest/` (RISC-V program) and `host/` (native runner/prover) crate.

Each guest pins its Rust toolchain via `rust-toolchain.toml`. `rustup` picks it up automatically.

## Build and Run

From a guest directory:

```sh
cargo airbender build
```

From the corresponding host directory:

```sh
cargo run --release
```

See each example's own README for usage details and available options.

## Examples

- **fibonacci** - basic no_std computation (nth Fibonacci number)
- **u256-add** - no_std with `ruint` for 256-bit integer arithmetic
- **std-btreemap** - std-enabled guest using `BTreeMap`
- **cycle-markers** - transpiler profiling with cycle markers and delegation snapshots
- **revm-basic** - revm transaction inside Airbender
