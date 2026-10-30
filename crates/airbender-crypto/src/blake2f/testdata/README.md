# BLAKE2 F test vectors

None of the expected outputs comes from this implementation.

`blake2b-kat.txt` is copied verbatim from the official BLAKE2 repository
([`testvectors/blake2b-kat.txt`](https://github.com/BLAKE2/BLAKE2/blob/ed1974ea83433eba7b2d95c5dcd9ac33cb847913/testvectors/blake2b-kat.txt),
blob `d9438ef1`): keyed BLAKE2b-512 of messages of 0 to 255 bytes. It only runs 12 rounds and
never sets the high counter word `t1`.

`eip152-vectors.txt` covers other round counts and `t1`. Each record is a name, an input in the
213-byte EIP-152 encoding, and the 64-byte output:

- `eip152-vector-4` to `-7`: EIP-152 test vectors 4 to 7
  ([eip-152.md](https://github.com/ethereum/EIPs/blob/5510973b40973b6aa774f04c9caba823c8ff8460/EIPS/eip-152.md#test-cases)).
  Vectors 0 to 3 are malformed inputs that `compress` cannot express, and vector 8
  (`0xffffffff` rounds) is too slow.
- `eest-*`: the successful cases of the execution spec tests
  ([`eip152_blake2`](https://github.com/ethereum/execution-spec-tests/tree/ca37419a162757c644c61829331321e36243547b/tests/istanbul/eip152_blake2)),
  named by `id`, without the four that repeat EIP-152 vectors and the 8,000,000-round case.
- `computed-rounds-20-high-counter`: 20 rounds with `t` the bytes `00` to `0f`, as no published
  vector sets `t1`. Its output comes from geth's generic Go F and an independent Python F, which
  agree.
