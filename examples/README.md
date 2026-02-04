# Airbender Examples

Each example contains two crates:

- `guest/`: guest program built with the Airbender toolchain.
- `host/`: native runner that executes, proves, and verifies the guest program.

## Build and Run

From a guest directory:

```sh
cargo airbender build
```

From the corresponding host directory:

```sh
cargo run
```

To generate and verify a proof:

```sh
cargo run -- --prove
```

## Examples

- `fibonacci`
- `u256-add` (no_std + `ruint`)
- `std-btreemap` (std + `BTreeMap`)
