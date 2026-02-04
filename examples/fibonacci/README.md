# Fibonacci Example

This example has a no_std guest that computes the nth Fibonacci number and a host that runs,
proves, and verifies the guest program.

## Build the guest

```sh
cd examples/fibonacci/guest
cargo airbender build
```

## Run the host

```sh
cd examples/fibonacci/host
cargo run
```

The default host run executes the guest only. To generate and verify a proof:

```sh
cargo run -- --prove
```
