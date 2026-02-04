# BTreeMap (std) Example

This example enables `std` in the guest and uses `std::collections::BTreeMap` to compute a
simple sum derived from the input. The host runs, proves, and verifies the program.

## Build the guest

```sh
cd examples/std-btreemap/guest
cargo airbender build
```

## Run the host

```sh
cd examples/std-btreemap/host
cargo run
```

The default host run executes the guest only. To generate and verify a proof:

```sh
cargo run -- --prove
```
