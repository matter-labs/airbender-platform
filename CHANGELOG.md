# Changelog

## [0.2.0](https://github.com/matter-labs/airbender-platform/compare/v0.1.0...v0.2.0) (2026-05-22)


### ⚠ BREAKING CHANGES

* (Breaking!) Implement support for 80/100 bit security ([#59](https://github.com/matter-labs/airbender-platform/issues/59))

### Features

* (Breaking!) Implement support for 80/100 bit security ([#59](https://github.com/matter-labs/airbender-platform/issues/59)) ([6a81afc](https://github.com/matter-labs/airbender-platform/commit/6a81afcf992f586256b943ba3241254202de8901))
* add cycle marker api support ([#32](https://github.com/matter-labs/airbender-platform/issues/32)) ([7918d51](https://github.com/matter-labs/airbender-platform/commit/7918d518e9cc1bf71bffa0da2f7b7c8999ae9e0b))
* add maybe_* convenience methods to builder structs ([#19](https://github.com/matter-labs/airbender-platform/issues/19)) ([a540319](https://github.com/matter-labs/airbender-platform/commit/a540319db68d2102d91f11d5a6f6ca855c77937d))
* completely remove risc_v_simulator ([#26](https://github.com/matter-labs/airbender-platform/issues/26)) ([87cf903](https://github.com/matter-labs/airbender-platform/commit/87cf9038f270634ba2809229d1d9f8a0f7587905))
* **crypto:** add Secp256k1Hooks for oracle-based EC field operations ([#58](https://github.com/matter-labs/airbender-platform/issues/58)) ([71ebcec](https://github.com/matter-labs/airbender-platform/commit/71ebcec782d4b79a240ac1c296c9928ff48498c5))
* **crypto:** modernize blake2s test program and enable runner tests ([#62](https://github.com/matter-labs/airbender-platform/issues/62)) ([d209284](https://github.com/matter-labs/airbender-platform/commit/d2092848a3c1276c8be1529539bd46b9f12218f2))
* deterministic builds ([#31](https://github.com/matter-labs/airbender-platform/issues/31)) ([040e982](https://github.com/matter-labs/airbender-platform/commit/040e98221ec98df49f4fae63df05367073de81a2))
* expose raw unrolled proof access ([#34](https://github.com/matter-labs/airbender-platform/issues/34)) ([c24bfe2](https://github.com/matter-labs/airbender-platform/commit/c24bfe20f6c6d7520e01039cd983b0e911209438))
* **host:** check system RAM before initializing CPU prover ([#52](https://github.com/matter-labs/airbender-platform/issues/52)) ([7d5720b](https://github.com/matter-labs/airbender-platform/commit/7d5720be827c95fd825f249f397b9af2c24fa8da))
* implement Commit trait for Result&lt;T, E&gt; ([#45](https://github.com/matter-labs/airbender-platform/issues/45)) ([e4ad20d](https://github.com/matter-labs/airbender-platform/commit/e4ad20d6293607fe84bb5d650f55e8e024b1159c))
* Publish cargo doc for main branch ([#38](https://github.com/matter-labs/airbender-platform/issues/38)) ([1b939ed](https://github.com/matter-labs/airbender-platform/commit/1b939ed101c8c144f431a1b1d76abf48e40ac70b))
* Release pipeline ([#64](https://github.com/matter-labs/airbender-platform/issues/64)) ([da6a062](https://github.com/matter-labs/airbender-platform/commit/da6a062e5585f5f2ccc58f4c565b7bc5b7e142e0))
* rework cargo-airbender templating ([#47](https://github.com/matter-labs/airbender-platform/issues/47)) ([39e91e6](https://github.com/matter-labs/airbender-platform/commit/39e91e658a5ed36c7097d483696b704e2d4058f7))
* **rt:** make QuasiUart::write_byte and flush public ([#55](https://github.com/matter-labs/airbender-platform/issues/55)) ([097f148](https://github.com/matter-labs/airbender-platform/commit/097f14891ae663fc51a2312da964780c3bdafbc7))
* support panic-immediate-abort ([#36](https://github.com/matter-labs/airbender-platform/issues/36)) ([8e8f8d5](https://github.com/matter-labs/airbender-platform/commit/8e8f8d5d6c2e55967de12b8cd4b2ddad8703a8f1))
* Support passing the machine configuration to the transpiler ([#63](https://github.com/matter-labs/airbender-platform/issues/63)) ([825dd25](https://github.com/matter-labs/airbender-platform/commit/825dd25c9397afc53f1205cbb369c4072e7d3ea5))


### Bug Fixes

* **build:** canonicalize mount_root in reproducible builds ([#53](https://github.com/matter-labs/airbender-platform/issues/53)) ([693907d](https://github.com/matter-labs/airbender-platform/commit/693907d50b114d28a34adefe97aa45802ffccc12))
* **macros:** place _start_rust in .init.rust section ([#50](https://github.com/matter-labs/airbender-platform/issues/50)) ([a4810fa](https://github.com/matter-labs/airbender-platform/commit/a4810faf4fb667b98073eede055a70b470a06729))
* Make CI pass & update refs to use new repo ([#1](https://github.com/matter-labs/airbender-platform/issues/1)) ([5db832a](https://github.com/matter-labs/airbender-platform/commit/5db832a3001538081cfb6a246558edb0f1574340))
* resolve guest metadata from project dir ([#39](https://github.com/matter-labs/airbender-platform/issues/39)) ([dda33be](https://github.com/matter-labs/airbender-platform/commit/dda33be72f56fdaa35a35c9ac6ff39b1c9b01f60))
* resolve guest project from nested directories ([#28](https://github.com/matter-labs/airbender-platform/issues/28)) ([de811f1](https://github.com/matter-labs/airbender-platform/commit/de811f1ac180bbf6edd370d1c23bb8a2ad1c6cab))
* **rt:** make allocator init functions `unsafe fn` ([#51](https://github.com/matter-labs/airbender-platform/issues/51)) ([7e559f9](https://github.com/matter-labs/airbender-platform/commit/7e559f9572267c124ed2a3dc2926b0c088e5f8d0))
* use riscv_transpiler for running host programs ([#24](https://github.com/matter-labs/airbender-platform/issues/24)) ([51092e4](https://github.com/matter-labs/airbender-platform/commit/51092e486d4e7e771bb52cb45e39ce5189fdcd5b))
