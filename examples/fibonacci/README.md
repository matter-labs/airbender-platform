# Fibonacci

A no_std guest that computes the nth Fibonacci number. The host feeds `n`, runs the guest, and can generate and verify a proof of the computation.

Good starting point if you're new to Airbender.

## Build and run

```sh
cd examples/fibonacci/guest
cargo airbender build

cd ../host
cargo run --release              # execute only
cargo run --release -- --prove   # execute + prove + verify
```
