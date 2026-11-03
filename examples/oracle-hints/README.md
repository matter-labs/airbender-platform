# oracle-hints

A host oracle that answers the hint queries of `airbender-crypto` live, with
`airbender_oracle::processor::QueryDispatcher`.

The hinted paths of the primitives (ModExp, ecrecover, the pairing checks, the MSMs, KZG)
replace each expensive step with a query to the oracle: a multi-digit division, a field
inverse or square root, a pairing witness. The guest checks every answer with a cheap
operation, so the oracle is trusted by no one. Unlike `ecrecover-hooks`, where the host
precomputes the hints and passes them as inputs, here the host answers each query as the
guest makes it: the query names the address of the request in the guest's memory, and the
dispatcher reads it through the simulator while the guest waits.

The **host** builds a `QueryDispatcher` with one `OracleQueryProcessor`:
`airbender_crypto::hints::CryptoHints` (the `oracle_processor` feature), the crate's own
responder, which declares its three advice query IDs (`FIELD_OPS_ADVISE_QUERY_ID`,
`MODEXP_ADVICE_QUERY_ID`, `U256_WIDE_DIV_REM_ADVICE_QUERY_ID`) and answers them with
`answer_query` for the runs of the dispatcher's `RunMode`. The same routine serves both sides:

- `RunMode::NativeRunOnly`: the dispatcher is the `MemoryOracle` of the hinted computation
  run in this process (`modexp(..., Some(&mut oracle))`), which checks the answers like the
  guest does.
- `RunMode::RiscVRun`: the dispatcher is the non-determinism CSR source of the guest in the
  simulator (`runner.run_with_source`). The guest's input words are served on the same CSR
  before its first query (`GuestSource`), and a query's answer takes precedence over them.

The **guest** reads its inputs, then runs ModExp and ecrecover with
`CsrOracle::<HintError>::new(CsrChannel)` as their oracle, and returns a digest of both
results, which the host compares with its native run.

## Build and run

```sh
cd examples/oracle-hints/guest
cargo airbender build

cd ../host
cargo run --release
```

## Expected output

```
Native run with hints:
  power: fad09d7ffb443a9f781660d4dbe4b763beef23f28a108645eaa703bb88b126d5
  key:   861254a4cf8dfd2d60e2a33e3143eac6285886f0aed91711ab7e2c013f265c55fd456667f3b08657795fe6754b6fbc11186714c4e4f48d5b856800e3e81c30c8

Guest execution: cycles=150099, output=0xa372daf8
Host verification: matches.
```
