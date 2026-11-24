# ModExp

The arithmetic of the ModExp precompile (`0x05`). Header parsing, the EIP-7823 length cap and gas
stay with the caller; the cap is relied on here: an operand of more than 1024 significant bytes
panics, and every working value is a fixed-capacity vector on the stack (`FixedVec`), so nothing
is allocated.

`modexp::modexp(base, exp, modulus, oracle)` takes big-endian operands and returns a
`ModExpOutput` (a `FixedVec` of up to 1024 bytes):

- with `Some(oracle)` it runs `advice::modexp`, which asks for hints and checks them; a wrong
  hint panics, so a prover cannot change a result;
- with `None` it runs `software::modexp` (aurora-engine's MPNat), which needs no oracle.

The outputs can differ in leading zero bytes, between the two paths and between the arithmetics
of the hinted one. `write_modexp_output(&output, mod_len, dst)` pads or truncates the result to
`mod_len` bytes, after which they agree.

## Hints

| Query ID | Used for | Request | Answer |
| --- | --- | --- | --- |
| `0x4005_0031` | each product mod a modulus of up to 32 bytes | address of `[lo, hi, divisor]` addresses | the 512-bit quotient: 8 low words, then 8 high words |
| `0x4005_0010` | each reduction mod a longer modulus | address of a `ModExpAdviceParamsGeneric` | quotient and remainder lengths in words, then their words, little-endian |

Every answer is checked. For a modulus of up to 32 bytes: the high half of the quotient is zero,
and `product - q * m` is in `[0, m)`. For a longer one: the lengths are bounded before a word is
received, `q * m + r` is the product exactly (in the digits of the product, with no carry out of
them), and the result of the walk is below the modulus. A remainder in the middle of the walk
may be any representative of at most the modulus's digits: it is congruent, which is all the
next product needs.

## Features and targets

Two independent choices:

- `hinted_operations` makes the hinted path exist (`advice`), on every target. Without it
  `modexp` panics when given an oracle, as the precompiles do.
- `cfg(bigint_ops)` (the feature, except on `riscv64`: see `build.rs`) selects the arithmetic
  under the hinted path, and nothing else. With it (RV32, and the hosts that emulate its
  delegation) values are 256-bit digits and every operation is a sequence of delegation calls
  (`u256/delegated.rs`, `advice/arith/delegated.rs`). Without it (RV64 and the other hosts)
  values are `u64` limbs and the operations are carry chains (`u256/naive.rs`,
  `advice/arith/limbs.rs`): schoolbook rows, Karatsuba from 24 limbs, squares with each cross
  product once.

The algorithm above the arithmetic is one (`advice/mod.rs`, `single_digit.rs`, `bigint.rs`,
`exponent.rs`): the walk, the queries, the bounds and the checks do not depend on the size of a
digit. The request of `0x4005_0010` counts its dividend and its modulus in 256-bit digits and
the answer counts `u32` words; limbs are rounded up to whole 256-bit digits, with zero limbs
written after the value for the oracle to read. Both arithmetics ask the same queries in the
same order, so a hint tape recorded on one replays on the other.

The hinted path builds on `riscv32`, `riscv64` and 64-bit little-endian hosts (`compile_error!`
in `advice/bigint.rs`).

## Tests

```sh
cargo test -p airbender-crypto -- modexp
cargo test -p airbender-crypto --features proving -- modexp
cargo test -p airbender-crypto --features oracle_implementations --test modexp_native
cargo test -p airbender-crypto --features proving,oracle_implementations --test modexp_native
```

The first and third run the limb arithmetic, the others the delegated one on the host emulation
of the delegation (where the delegation counts are pinned). `tests/modexp_native.rs` runs
without `cfg(test)`: the vectors, operands of every size class against `software::modexp` and
`num-bigint`, replays, and tapes with wrong answers. Both arithmetics must record the tapes
whose digests the tests pin. The oracle side of the two queries is `oracle_implementations/`.
