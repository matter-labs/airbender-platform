# REVM Basic

Runs `revm` inside an Airbender guest and proves the execution end to end.

The guest creates a minimal in-memory EVM, executes a transaction against a contract that writes to
memory and returns data, then commits the gas used. The host runs the same logic natively, compares
the result, and optionally generates and verifies a proof.

## Build and run

From the repository root:

```sh
cargo airbender build --project examples/revm-basic/guest

cargo run --release --manifest-path examples/revm-basic/host/Cargo.toml            # simulate only
cargo run --release --manifest-path examples/revm-basic/host/Cargo.toml -- --prove # simulate + prove + verify
```
