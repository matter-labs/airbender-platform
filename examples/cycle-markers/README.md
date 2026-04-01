# Cycle Markers

Demonstrates guest-side cycle markers and host-side marker collection. The guest uses `record_cycles(...)` around a delegated Keccak invocation; the host collects the snapshots and computes cycle counts and delegation usage for the profiled region.

Cycle markers are for transpiler profiling only. Binaries with markers must not be sent through real CPU/GPU proving.

## Build and run

```sh
cd examples/cycle-markers/guest
cargo airbender build

cd ../host
cargo run --release
```

Note: this example does not support `--prove` since cycle markers are incompatible with real proving.
