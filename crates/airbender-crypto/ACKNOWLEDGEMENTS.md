# ACKNOWLEDGEMENTS
This crate includes source code adapted from the following open-source projects. All reused code is marked in the source files with inline comments. Where applicable, significant modifications have been made.

---
## `./src/ark_ff/`
- [ark_ff](https://github.com/arkworks-rs/algebra/tree/master/ff) dual-licensed under [MIT](https://github.com/arkworks-rs/algebra/blob/master/LICENSE-MIT) and [Apache-2.0](https://github.com/arkworks-rs/algebra/blob/master/LICENSE-APACHE)
---
## `./src/secp256k1/`
-  [k256](https://github.com/RustCrypto/elliptic-curves/tree/master/k256) dual-licensed under [MIT](https://github.com/RustCrypto/elliptic-curves/blob/master/k256/LICENSE-MIT) and [Apache-2.0](https://github.com/RustCrypto/elliptic-curves/blob/master/k256/LICENSE-APACHE)
- [secp256k1](https://github.com/bitcoin-core/secp256k1) licensed under [MIT](https://github.com/bitcoin-core/secp256k1/blob/master/COPYING)
---

## `./src/secp256r1/`
- [p256](https://github.com/RustCrypto/elliptic-curves/tree/master/p256) dual-licensed under [MIT](https://github.com/RustCrypto/elliptic-curves/blob/master/p256/LICENSE-MIT) and [Apache-2.0](https://github.com/RustCrypto/elliptic-curves/blob/master/p256/LICENSE-APACHE)

---
## `./src/bls12_381/eip2537.rs`
- [ark_ec](https://github.com/arkworks-rs/algebra/tree/master/ec) dual-licensed under [MIT](https://github.com/arkworks-rs/algebra/blob/master/LICENSE-MIT) and [Apache-2.0](https://github.com/arkworks-rs/algebra/blob/master/LICENSE-APACHE)
  - `evaluate_polynomial` and `apply_isogeny_map` are heap-free reimplementations of [`IsogenyMap::apply`](https://github.com/arkworks-rs/algebra/blob/af564e48/ec/src/hashing/curve_maps/wb.rs#L42-L64)

Please refer to the source files for detailed comments