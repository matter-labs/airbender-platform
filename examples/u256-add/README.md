# U256 Addition (no_std) Example

This example uses `ruint` in a no_std guest to assert that `a + b == c` for three U256 inputs.
The host feeds the inputs, executes the program, generates a proof, and verifies it.

## Build the guest

```sh
cd examples/u256-add/guest
cargo airbender build
```

## Run the host

```sh
cd examples/u256-add/host
cargo run
```

The default host run executes the guest only. To generate and verify a proof:

```sh
cargo run -- --prove
```
