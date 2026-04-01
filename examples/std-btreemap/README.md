# BTreeMap (std)

A guest with `std` enabled, using `std::collections::BTreeMap` to compute a sum from structured input. Shows that standard library types work on the guest.

## Build and run

```sh
cd examples/std-btreemap/guest
cargo airbender build

cd ../host
cargo run --release              # execute only
cargo run --release -- --prove   # execute + prove + verify
```
