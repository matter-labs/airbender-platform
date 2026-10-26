# ModExp (ported from zksync-os `185a0568`)

The crypto half of the zksync-os ModExp precompile (`0x05`), ported from zksync-os revision
`185a0568` (branch `ye_eth_modexp`: "reduce single-digit modexp products with one quotient
multiplication"). The wrapper stays in zksync-os: the parsing of the header, the EIP-7823 length
cap, the gas and the native cost, and the cycle markers. Every ported file starts with one
`// Ported from` line per source file, naming the zksync-os revision and the path in it; the table
below is those lines.

The arithmetic, the advice protocol (the queries and their order) and the checks of the advice are
the zksync-os ones. A wrong hint panics, as in zksync-os, so a prover cannot change a result.

## Entry points

`modexp::modexp(base, exp, modulus, allocator, oracle) -> Vec<u8, A>` takes big-endian operands and
an `Option<&mut O>` oracle (see `crate::hints`):

- with an oracle, the advice path (`modexp::advice::modexp`), which asks the hints below and
  checks them;
- without one, the path without advice (`modexp::software::modexp`, the MPNat code that zksync-os
  uses off the proving target).

The caller chooses the path. zksync-os chooses by target and by its `USE_ADVICE` constant; the
oracle presence replaces that here. The two paths can return a different number of leading zero
bytes (a zero result is `[0]` without advice, and no bytes with it, as is the result for a modulus
of 0 or 1); `modexp::write_modexp_output(output, mod_len, dst)` pads the output with zeroes, or
truncates it, to `mod_len` bytes, into any `TryExtend<u8>` (a `Vec` is one), and then they agree.
That is the pad step of zksync-os's wrapper. `modexp::strip_leading_zeroes` is the helper that the
wrapper's cost model uses.

The module is behind the `alloc` feature (and `cfg(test)`): the operands and the digits are
allocated from the allocator that the caller passes.

## Hints

| Query ID | Asked by | Request | Answer |
| --- | --- | --- | --- |
| `0x4005_0031` (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`) | the single-digit path (a modulus of up to 32 bytes): each modular product | the address of an array of the addresses of the low half and the high half of the 512-bit dividend and of the divisor (`[usize; 3]`) | the quotient, 16 words (low half, then high half, 8 words each); the remainder is derived and checked to be below the divisor |
| `0x4005_0010` (`MODEXP_ADVICE_QUERY_ID`) | the multi-digit path (a longer modulus): the reductions | the address of a `ModExpAdviceParamsGeneric` | the lengths (in `u32` words) of the quotient and the remainder, then their words, little-endian, without leading zero limbs |

A query is two CSR writes, and the protocol adds no framing to an answer. The wide quotient has no
length prefix; the answer of the multi-digit reduction starts with its two length words, which
are not field limbs. See [`../hints/README.md`](../hints/README.md) for the provenance of the
oracle side (`hints/wide_div.rs`, `hints/host/arith.rs`).

## Target constraints

- `advice/bigint.rs` keeps zksync-os's `compile_error!`: the advice ModExp only builds on `riscv32`
  or on 64-bit little-endian targets. The module is part of every build with `alloc`, so on a
  32-bit non-RISC-V target or a big-endian one, `airbender-crypto` with `alloc` does not compile
  at all. Nothing in this workspace targets one.
- The digits of the single-digit path are `U256`s: the delegated one (`u256/delegated.rs`, with the
  delegation sequences that the guest's cycle and delegation counts depend on) on RV32 with
  `bigint_ops`, and the naive one (`u256/naive.rs`, over `ruint`) everywhere else. Test builds
  (`cfg(test)`) compile both, compare them, and run ModExp on the delegated one, on the host
  emulation of the delegation, which counts its calls.

## Files

| File | Content | zksync-os source |
| --- | --- | --- |
| `mod.rs` | `modexp` (the dispatch is new), `write_modexp_output`, `TryExtend`, `strip_leading_zeroes`, `ModExpAdviceParams*` and `read_modexp_advice_params`; the API-level tests | `185a0568:basic_system/src/system_functions/modexp/mod.rs`<br>`185a0568:zk_ee/src/common_traits/mod.rs` |
| `advice/mod.rs` | `advice::modexp` and the single-digit / multi-digit dispatch | `185a0568:basic_system/src/system_functions/modexp/advice/mod.rs` |
| `advice/bigint.rs` | The multi-digit big integers, `ModexpAdvisor` and the oracle advisor | `185a0568:basic_system/src/system_functions/modexp/advice/bigint.rs` |
| `advice/digit.rs` | The 256-bit digits of the multi-digit path (zksync-os's `advice/u256.rs`) | `185a0568:basic_system/src/system_functions/modexp/advice/u256.rs` |
| `advice/exponent.rs` | The exponent walk shared by both paths | `185a0568:basic_system/src/system_functions/modexp/advice/exponent.rs` |
| `advice/single_digit.rs` | The single-digit path: one 512-bit product, one advised quotient, its check | `185a0568:basic_system/src/system_functions/modexp/advice/single_digit.rs` |
| `software/mod.rs` | `software::modexp` (the code of aurora-engine, with the allocator passed around) | `185a0568:supporting_crates/modexp/src/lib.rs` |
| `software/arith.rs` | Word arithmetic (Montgomery product, inverse) | `185a0568:supporting_crates/modexp/src/arith.rs` |
| `software/mpnat.rs` | The multi-precision naturals | `185a0568:supporting_crates/modexp/src/mpnat.rs` |
| `u256/mod.rs` | The choice of the `U256` and the tests that compare the two | `185a0568:supporting_crates/u256/src/lib.rs` |
| `u256/naive.rs` | The `U256` of the targets without the bigint delegation (only the operations ModExp uses) | `185a0568:supporting_crates/u256/src/naive/mod.rs` |
| `u256/delegated.rs` | The `U256` of RV32 with the bigint delegation (only the operations ModExp uses) | `185a0568:supporting_crates/u256/src/risc_v/mod.rs`<br>`185a0568:supporting_crates/delegated_u256/src/lib.rs`<br>`185a0568:supporting_crates/delegated_u256/src/delegation.rs`<br>`185a0568:supporting_crates/delegated_u256/src/copy.rs`<br>`185a0568:supporting_crates/delegated_u256/src/arithmetic.rs`<br>`185a0568:supporting_crates/delegated_u256/src/utils.rs` |
| `test_vectors.rs` | The 62 ModExp vectors of the precompile tests of zksync-os (test only) | `185a0568:tests/instances/precompiles/src/lib.rs` |

Outside this directory:

| File | Content | zksync-os source |
| --- | --- | --- |
| `../hints/wide_div.rs` | The wide division advice query of the single-digit path | `185a0568:basic_system/src/system_functions/u256_advice.rs` and the `zk_ee` oracle query files (see its header) |
| `../hints/host/arith.rs` | The answers to both ModExp queries | `185a0568:callable_oracles/src/arithmetic/mod.rs` and its oracle files (see its header) |
| `../byte_order.rs` | `words_to_be_bytes_32`, `read_be_into_words` and `bytereverse_words_in_place` (RV32 only) | `185a0568:supporting_crates/delegated_u256/src/byte_order.rs` |

## Differences from the source

- `advice::modexp` has no logger (zksync-os's was never read), and `HintOracle` stands for its
  `IOOracle`.
- The delegated `U256` calls the delegation through this crate's raw interface
  (`bigint_op_delegation_with_carry_bit_raw`), with the same CSR sequence. Only the operations
  that ModExp uses are ported.
- zksync-os's test that a forged remainder is rejected for a multi-digit modulus never reached the
  advice of the multi-digit path (its modulus was one digit). Here its modulus is two digits.
- `modexp::advice::delegated_modexp_with_naive_advisor` is there with `feature = "testing"` (the
  zksync-os fuzzers import it), without a logger.
- `write_modexp_output` is a function here; in zksync-os it is the tail of the wrapper.

## Tests

`cargo test -p airbender-crypto --features alloc -- modexp:: hints::wide_div hints::host` runs:

- the 62 vectors of zksync-os through both paths (`modexp::tests::vectors_both_paths`);
- the delegation counts of zksync-os's reference runs (single-digit constants, the per-input
  differences of the multi-digit path, and the single-digit and benchmark shapes). In a debug
  build the single-digit path's `debug_assert!`s add delegations, which the tests subtract (zero
  in a release build);
- the checks of the wide division and of the answers of the oracle, including wrong, short and
  excess answers.

`tests/modexp_native.rs` runs the vectors and the tape recorder in a build without `cfg(test)`,
where the single-digit path runs on the naive `U256`; it needs `--features alloc`, like the
other `*_native.rs` tests, and checks that the tapes equal the ones of the unit tests (the
delegated `U256`), so that both variants record the same tape.
