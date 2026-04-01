# U256 Addition

A no_std guest that uses `ruint` to assert `a + b == c` for three U256 inputs. Shows how to use external crates in a guest program.

## Build and run

```sh
cd examples/u256-add/guest
cargo airbender build

cd ../host
cargo run --release              # execute only
cargo run --release -- --prove   # execute + prove + verify
```
