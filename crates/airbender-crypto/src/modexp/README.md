# ModExp

The arithmetic of the ModExp precompile (`0x05`). Header parsing, the EIP-7823 length cap and gas
stay with the caller. The module needs the `alloc` feature.

`modexp::modexp(base, exp, modulus, allocator, oracle)` takes big-endian operands:

- with `Some(oracle)` it runs `advice::modexp`, which asks for hints and checks them; a wrong
  hint panics, so a prover cannot change a result;
- with `None` it runs `software::modexp` (aurora-engine's MPNat), which needs no oracle.

The two paths can differ in leading zero bytes. `write_modexp_output(output, mod_len, dst)` pads or
truncates the result to `mod_len` bytes, after which they agree.

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
cargo test -p airbender-crypto --features alloc -- modexp
cargo test -p airbender-crypto --features alloc --test modexp_native
```

`tests/modexp_native.rs` runs the vectors on the naive digits and checks that the recorded hint
tapes match the unit tests' tapes.
