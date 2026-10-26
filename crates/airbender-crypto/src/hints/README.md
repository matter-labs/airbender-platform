# Provenance of the hint, precompile and byte-order code

The code in `src/hints/`, `src/precompiles/` and `src/byte_order.rs` is the crypto side of
zksync-os's hints ("advice") and precompiles, ported into this crate. ModExp, which
`src/modexp/` ports, has its own table in [`../modexp/README.md`](../modexp/README.md).

Every ported file starts with one `// Ported from` line per source file, naming the zksync-os
revision and the path in it (`<rev>:<path>`). The tables below are those lines, and nothing else is
a source. The zksync-os revisions are:

| Revision | Branch | Subject |
| --- | --- | --- |
| `e855bca2` | `ye_eth_bls_hints` | perf(basic_system): run the EIP-2537 precompiles on checked oracle hints |
| `185a0568` | `ye_eth_modexp` | perf(basic_system): reduce single-digit modexp products with one quotient multiplication |

(Repository: `matter-labs/zksync-os`.) `185a0568` is the revision of the arithmetic advice of
ModExp, the wide quotient of its single-digit path and the quotient and remainder of its
multi-digit path: `hints/wide_div.rs`, `hints/host/arith.rs` and `src/modexp/`. The rest of
`hints/`, `precompiles/` and `byte_order.rs` is `e855bca2` (`byte_order.rs` is identical at both
revisions).

What changed against the sources is limited to:

- imports, and `HintOracle` / `HintError` / `QuerierMemory` in place of the `zk_ee` oracle types;
- the gates for the targets of this crate (`alloc`, 64-bit hosts for the oracle side, and RV32
  without `bigint_ops`, where there are no hints and the endpoints only run without an oracle);
- test-harness plumbing (the test doubles in `hints/test_support.rs`);
- `unsafe` contracts on `HintOracle::send_query`, `send_field_hint_query`, `query_field_hint`,
  `query_field_hint_into`, `answer_query` and `not_identity_claim`, which are safe in the sources
  (where `answer_query` is the `process_memory_query` of `oracle_provider`): an oracle that answers
  in this process reads the operands of a query in place, as values of the types of its op, so the
  caller must pass a live request whose operands are such values;
- the endpoints of `precompiles/`, which are the bodies of the inner functions of zksync-os's
  wrappers, taken out of them: the wrappers (gas, the padding and size checks of the input, the
  encoding of the output, the cycle markers) stay in zksync-os. The path, with or without hints, is
  chosen by `Option<&mut O>` (see `precompiles/mod.rs`).

The query IDs, the `FieldHintOp` numbers, the request layouts, the words of the answers, the order
of the queries and the operations that take hints are the zksync-os ones. Tests pin them
(`hints::tests::query_ids_match_zksync_os`, `field_hint_op_numbers_match_zksync_os`, and the
tape-replay tests of the endpoints).

## `src/hints/`

| File | Content | zksync-os source |
| --- | --- | --- |
| `mod.rs` | The query IDs, the module layout and the re-exports; the tests of the IDs and of the op numbers | `e855bca2:zk_ee/src/oracle/query_ids.rs`<br>`e855bca2:basic_system/src/system_functions/field_ops.rs`<br>`e855bca2:basic_system/src/system_functions/modexp/mod.rs` |
| `oracle.rs` | `HintError`, `HintOracle` (the primitives of the memory-based oracle), `NoHints` | `e855bca2:zk_ee/src/oracle/memory_io/oracle.rs` |
| `request.rs` | `FieldHintOp` (the 15 ops), `GenericFieldOpsHint`, `QuerierMemory`, the request readers, `send_field_hint_query` and `query_field_hint*` | `e855bca2:basic_system/src/system_functions/field_ops.rs`<br>`e855bca2:zk_ee/src/oracle/memory_io/host.rs` |
| `answer.rs` | `HintTarget`, `HintAnswer` and its impls for flags, arrays and tuples, `read_hint_answer` | `e855bca2:basic_system/src/system_functions/field_ops.rs` |
| `csr.rs` | `CsrHintOracle`: the non-determinism CSR as the word channel (RV32 only) | `e855bca2:zk_ee/src/oracle/memory_io/word_channel.rs`<br>`e855bca2:proof_running_system/src/io_oracle/mod.rs`<br>`e855bca2:zksync_os/src/main.rs` |
| `encoding.rs` | `HintEncoding` (the component order of the bn254 and bls12-381 field elements) and the `HintAnswer` impls of the fields | `e855bca2:basic_system/src/system_functions/curve_hints.rs` |
| `guest_layout.rs` | Where the oracle finds the operands in the memory of the RISC-V guest (points, pairs) | `e855bca2:basic_system/src/system_functions/curve_hints.rs` |
| `fields.rs` | The inverse and square-root hints of the bn254 and bls12-381 base fields and of `Fq12`, the dividers, and the bls12-381 `Fq2` extension | `e855bca2:basic_system/src/system_functions/curve_hints.rs`<br>`e855bca2:basic_system/src/system_functions/curve_hints/bls12_381_extension/mod.rs`<br>`e855bca2:basic_system/src/system_functions/curve_hints/bls12_381_extension/tests/mod.rs` |
| `pairing.rs` | `PairingClaim`, the residue witness hints, the `G2` slope hints and `HintedChain`, `checked_inverse`, `Bls12381Pair` | `e855bca2:basic_system/src/system_functions/curve_hints.rs` |
| `secp256k1.rs` | `Secp256k1Element`, the secp256k1 answers and `Secp256k1HooksWithOracle` | `e855bca2:basic_system/src/system_functions/field_ops.rs` |
| `wide_div.rs` | The wide division advice of the single-digit ModExp path (`U256_WIDE_DIV_REM_ADVICE_QUERY_ID`) | `185a0568:basic_system/src/system_functions/u256_advice.rs`<br>`185a0568:zk_ee/src/oracle/memory_io/query.rs`<br>`185a0568:zk_ee/src/oracle/memory_io/composite.rs`<br>`185a0568:zk_ee/src/oracle/memory_io/continuous.rs`<br>`185a0568:zk_ee/src/oracle/memory_io/oracle.rs` |
| `host/mod.rs` | `NativeQuerierMemory`, `answer_query` (the dispatch by query ID) | `e855bca2:zk_ee/src/oracle/memory_io/host.rs`<br>`e855bca2:oracle_provider/src/lib.rs` |
| `host/field.rs` | The answers to the field hints, and `not_identity_claim` | `e855bca2:callable_oracles/src/field_hints/mod.rs`<br>`e855bca2:callable_oracles/src/field_hints/impls.rs` |
| `host/arith.rs` | The answers to the ModExp advice: the wide quotient, and the quotient and remainder of the multi-digit path | `185a0568:callable_oracles/src/arithmetic/mod.rs`<br>`185a0568:zk_ee/src/oracle/memory_io/host.rs`<br>`185a0568:oracle_provider/src/lib.rs` |
| `host/recorder.rs` | `RecordingHintOracle`, `record_hints`: the native recording of the hint tape (the per-thread recording of the C API is in `airbender-crypto-c`) | `e855bca2:oracle_provider/src/lib.rs` |
| `test_support.rs` | Test doubles of the oracle and of the memory of the querier (`TapeOracle` is new; the others are ported) | `e855bca2:zk_ee/src/oracle/memory_io/host.rs`<br>`e855bca2:callable_oracles/src/lib.rs`<br>`e855bca2:oracle_provider/src/lib.rs`<br>`e855bca2:basic_system/src/system_functions/curve_hints.rs` |

## `src/precompiles/`

| File | Content | zksync-os source |
| --- | --- | --- |
| `mod.rs` | Module docs, the re-exports, `hints_unavailable` and the test helpers are new; `WordAligned` is ported | `e855bca2:basic_system/src/system_functions/ecrecover.rs` |
| `ecrecover.rs` | `secp256k1_ecrecover` | `e855bca2:basic_system/src/system_functions/ecrecover.rs` |
| `bn254.rs` | `bn254_g1_add`, `bn254_g1_mul` | `e855bca2:basic_system/src/system_functions/bn254_ecadd.rs`<br>`e855bca2:basic_system/src/system_functions/bn254_ecmul.rs` |
| `bn254/pairing_check.rs` | `bn254_pairing_check` | `e855bca2:basic_system/src/system_functions/bn254_pairing_check.rs` |
| `bls12_381.rs` | The EIP-2537 encodings, `Bls12Error`, G1 and G2 addition and the field-to-curve maps | `e855bca2:basic_system/src/system_functions/bls12_381/mod.rs`<br>`e855bca2:basic_system/src/system_functions/bls12_381/addition/mod.rs`<br>`e855bca2:basic_system/src/system_functions/bls12_381/mappings/mod.rs` |
| `bls12_381/msm.rs` | `bls12_381_g1_msm`, `bls12_381_g2_msm` (Pippenger) | `e855bca2:basic_system/src/system_functions/bls12_381/msm/mod.rs`<br>`e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/mod.rs`<br>`e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/errors.rs`<br>`e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/single_pair.rs`<br>`e855bca2:basic_system/src/system_functions/bls12_381/msm/tests/multiple_pairs.rs` |
| `bls12_381/pairing.rs` | `bls12_381_pairing_check` | `e855bca2:basic_system/src/system_functions/bls12_381/pairing.rs`<br>`e855bca2:basic_system/tests/bls_pairing_hints.rs` |
| `kzg.rs` | `kzg_verify_proof`, `verify_kzg_proof_with_oracle` and its parsers `parse_g1_compressed_with_oracle` and `parse_scalar` (the point evaluation precompile) | `e855bca2:basic_system/src/system_functions/point_evaluation.rs` |

`precompiles/unavailable.rs` has no header: it is new. It holds the stand-ins of the hints on RV32
without `bigint_ops`, which panic when an oracle is given.

## `src/byte_order.rs`

| File | Content | zksync-os source |
| --- | --- | --- |
| `byte_order.rs` | The byte-order helpers of the RISC-V target (`rev8`, the big-endian readers, the limb conversions) that the precompiles and ModExp use. The file is identical at the two revisions. | `e855bca2:supporting_crates/delegated_u256/src/byte_order.rs`<br>`185a0568:supporting_crates/delegated_u256/src/byte_order.rs` |

## Tests outside `src/`

| File | Content | zksync-os source |
| --- | --- | --- |
| `tests/hints_bls_pairing_layout.rs` | The guest layout of an arbitrary BLS12-381 pairing request, against the native recording | `e855bca2:callable_oracles/tests/bls_pairing_layout.rs` |

The other files of `tests/` that start with `hints_`, `precompiles_` or `modexp_` are new. They run
the code in a build without `cfg(test)`, where the fields are the arkworks ones (see the crate
README).

## Checking this table

Every file with a header has a row, and every row has a file:

```sh
rg -l 'Ported from zksync-o[s]' crates/airbender-crypto/src
rg -o '^// Ported from zksync-o[s] \S+' crates/airbender-crypto/src/hints crates/airbender-crypto/src/precompiles crates/airbender-crypto/src/byte_order.rs
```

The first command lists the files of the tables above and of
[`../modexp/README.md`](../modexp/README.md), and no others (the pattern is spelled so that
the READMEs do not match it). The second prints each `rev:path`.
