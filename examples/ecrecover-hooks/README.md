# ecrecover-hooks

Secp256k1 EC recovery using the hint-and-verify pattern with `Secp256k1Hooks`.

EC recovery requires three expensive operations: a field-element square root,
a field-element inverse, and a scalar inverse. Computing these from scratch
on a RISC-V guest is costly to prove. The hooks API lets you replace them with
precomputed hints that the guest verifies cheaply (one multiply each).

The **host** runs recovery once with `CapturingHooks` to record the results of
each expensive operation. These captured values become the hints.

The **guest** reads the hints from the input channel and uses
`PrecomputedHintHooks` — a `Secp256k1Hooks` implementation that verifies each
hint with a single cheap check (`candidate * input == 1` for inversions,
`candidate² == input` for square roots) and assigns the verified result.

## Build and run

```sh
cd examples/ecrecover-hooks/guest
cargo airbender build

cd ../host
cargo run --release
```

## Expected output

```
Hints precomputed on host:
  sqrt:       9e7c10ae1031...
  scalar_inv: fd262fef23f1...
  fe_inv:     793562308d66...

Guest execution: cycles=252160, output=0x861254a4
Host verification: matches.
```
