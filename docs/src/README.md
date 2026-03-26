# Airbender Platform User Guide

Airbender lets you write Rust programs whose execution can be proven with zero-knowledge. Your code compiles to RISC-V, runs inside a virtual machine, and produces a cryptographic proof that the execution was correct, without revealing the inputs.

API reference docs for the current `main` branch are published at
[`/api/`](https://matter-labs.github.io/airbender-platform/api/).

The programming model has two sides:

- A **guest** program: the RISC-V code you want to prove.
- A **host** program: native Rust that feeds inputs to the guest, runs it, and optionally generates and verifies proofs.

## Reading Order

1. [Installation & Hello World](./01-installation-and-hello-world.md) - get set up and prove your first program.
2. [Guest Program API](./03-guest-program-api.md) - how to write guest programs.
3. [Host Program API](./02-host-program-api.md) - how to drive guests from the host side.
4. [Using Crypto](./04-crypto-on-guest-and-host.md) - shared crypto primitives with prover-accelerated delegation.
5. [CLI Reference](./05-cli-reference.md) - every `cargo airbender` command and flag.

## Examples

Complete guest + host examples live in the repository under [`examples/`](https://github.com/matter-labs/airbender-platform/tree/main/examples):

- [`fibonacci`](https://github.com/matter-labs/airbender-platform/tree/main/examples/fibonacci) - basic no_std computation
- [`u256-add`](https://github.com/matter-labs/airbender-platform/tree/main/examples/u256-add) - no_std with `ruint` for big integers
- [`std-btreemap`](https://github.com/matter-labs/airbender-platform/tree/main/examples/std-btreemap) - std-enabled guest with `BTreeMap`
- [`cycle-markers`](https://github.com/matter-labs/airbender-platform/tree/main/examples/cycle-markers) - transpiler profiling with delegation snapshots
