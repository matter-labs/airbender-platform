# Cycle Markers Example

This example demonstrates the guest-side cycle marker API and host-side marker
collection through `airbender-host`.

The guest places two markers around a delegated Keccak invocation. The host
collects the marker snapshots and computes the cycles and delegation counts for
the profiled region.

Cycle markers are intended for transpiler profiling only. Programs that contain
them are development artifacts and must not be sent through the real CPU/GPU
proving flow.

## Build the guest

```sh
cd examples/cycle-markers/guest
cargo airbender build
```

## Run the host

```sh
cd examples/cycle-markers/host
cargo run
```
