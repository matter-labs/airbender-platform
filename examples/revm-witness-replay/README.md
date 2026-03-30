# REVM Witness Replay

Runs `revm` inside an Airbender guest and proves the execution end to end.

The guest creates a minimal in-memory EVM, sends a transaction to a small contract, and commits
the gas used. The host runs the same thing natively, compares, and optionally proves + verifies.

## Build and run

```sh
cd guest
cargo airbender build

cd ../host
cargo run            # simulate only
cargo run -- --prove # simulate + prove + verify
```

## Why the contract is trivial

The contract is stack-only (`PUSH, ADD, POP, STOP`) — no memory, no return data, no precompiles.

We wanted a richer contract calling `ecrecover` backed by `airbender-crypto`. That simulates fine
but crashes the prover. Notes below for the prover team.

## Proving issue: signed multiply

### What we saw

Contracts that use memory (even just CALLDATACOPY + RETURN) crash the prover with "illegal
instruction." Stack-only contracts prove fine.

| Contract                               | Simulates | Proves |
|----------------------------------------|-----------|--------|
| CALLDATACOPY + STATICCALL + RETURN     | yes       | no     |
| CALLDATACOPY + RETURN (no precompile)  | yes       | no     |
| STOP only                              | yes       | yes    |
| Stack arithmetic only                  | yes       | yes    |

### What we confirmed

- **The failing instruction is `mulh`.** Decoded `0x02f897b3` → `mulh a5, a7, a5` (RISC-V
  signed multiply-high, funct3=001, funct7=0x01).

- **The prover rejects it.** `FullUnsignedMachineDecoderConfig` in zksync-airbender sets
  `SUPPORT_SIGNED_MUL_DIV = false`. The decoder treats `mulh` as illegal.

### What we can't explain yet

The original theory was that revm's memory gas formula (`3*n + n*n/512`) produces the `mulh`.
But we checked the source — `memory_gas()` in revm-interpreter is all `u64`. On rv32, `u64*u64`
should lower to `mul` + `mulhu` (unsigned), not `mulh` (signed). So the gas formula probably
isn't the direct cause.

The `mulh` is real and memory is the trigger, but we don't know which part of the code path
actually emits it. Could be `Vec::resize`, the allocator, a dependency, or an LLVM lowering
quirk. Disassembling the guest ELF around the crashing PC would answer this.

### Other notes

Reportedly, flipping `SUPPORT_SIGNED_MUL_DIV` to true gets past this crash but then hits an
output mismatch. We haven't re-verified that ourselves.
