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

The two paths can differ in leading zero bytes. `write_modexp_output(&output, mod_len, dst)` pads
or truncates the result to `mod_len` bytes, after which they agree.

## Hints

| Query ID | Used for | Request | Answer |
| --- | --- | --- | --- |
| `0x4005_0031` | each product mod a modulus of up to 32 bytes | address of `[lo, hi, divisor]` addresses | the 512-bit quotient: 8 low words, then 8 high words |
| `0x4005_0010` | each reduction mod a longer modulus | address of a `ModExpAdviceParamsGeneric` | quotient and remainder lengths in words, then their words, little-endian |

## Targets

The advice path builds only on `riscv32` and on 64-bit little-endian hosts (`compile_error!` in
`advice/bigint.rs`). Its 256-bit digits use the bigint delegation on RV32 with `bigint_ops`
(`u256/delegated.rs`) and `ruint` elsewhere (`u256/naive.rs`). Unit tests run the delegated
digits on the host emulation of the delegation.

## Tests

```sh
cargo test -p airbender-crypto -- modexp
cargo test -p airbender-crypto --features oracle_implementations --test modexp_native
```

`tests/modexp_native.rs` runs the vectors on the naive digits and checks that the recorded hint
tapes match the unit tests' tapes. The oracle side of the two queries is
`oracle_implementations/`.
